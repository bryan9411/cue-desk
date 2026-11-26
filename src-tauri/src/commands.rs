//! 前端可呼叫的 Tauri 指令。錯誤一律以 [`CueError`] 序列化為 `{ kind, message }`。
//!
//! 會呼叫 osascript 的指令都是 `async`，在 `spawn_blocking` 中執行，不阻塞主執行緒。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use crate::config::{self, CueConfig, Workspace};
use crate::discovery;
use crate::error::CueError;
use crate::layout;
use crate::model::{SessionKey, SessionKind};
use crate::registry::SessionNode;
use crate::shell::SessionKindToCreate;
use crate::state::{lock, workspace_roots, AppState, Snapshot};
use crate::terminal::{
    Capabilities, CreateSpec, CreatedSession, OpenMode, SplitDirection, TerminalBackend,
};

/// 目前的完整狀態。
#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Snapshot {
    state.snapshot()
}

/// 聚焦工作階段。已結束的節點回傳 `SessionExited`。
#[tauri::command]
pub async fn focus_session(app: AppHandle, id: String) -> Result<(), CueError> {
    let state = app.state::<AppState>();
    let (key, window_id) = {
        let registry = lock(&state.registry);
        let node = registry.get(&id).ok_or(CueError::ObjectNotFound)?;
        if !node.alive {
            return Err(CueError::SessionExited);
        }
        (node.key.clone(), node.window_id)
    };
    let backend = state.backend();
    tauri::async_runtime::spawn_blocking(move || backend.focus(&key))
        .await
        .map_err(|e| CueError::Io(format!("聚焦工作異常結束：{e}")))??;
    lock(&state.registry).record_focus(window_id);
    Ok(())
}

/// 在工作區建立工作階段，回傳新節點的 id（分割開啟方式為 AI 窗格）。
///
/// `kind` 只接受 `"claude"`／`"gemini"`／`"zsh"`，其他值在反序列化時就被拒絕；
/// 後端不支援的開啟方式回傳 `Unsupported`，不會改用其他方式。
#[tauri::command]
pub async fn create_session(
    app: AppHandle,
    workspace_id: String,
    kind: SessionKindToCreate,
    label: Option<String>,
    mode: OpenMode,
) -> Result<String, CueError> {
    let label = label.map(|l| config::normalize_label(&l)).transpose()?.filter(|l| !l.is_empty());
    let state = app.state::<AppState>();
    let backend = state.backend();
    check_mode(backend.capabilities(), mode)?;

    let workspace_path = {
        let config = lock(&state.config);
        let ws = config.workspaces.iter().find(|w| w.id == workspace_id).ok_or(CueError::WorkspaceNotFound)?;
        ws.path.clone()
    };
    if !workspace_path.is_dir() {
        return Err(CueError::InvalidWorkspace("工作區資料夾不存在".into()));
    }
    let target_window = match mode {
        OpenMode::NewWindow => None,
        _ => layout::resolve_new_tab_target(&workspace_id, &lock(&state.registry)),
    };

    let spec = CreateSpec { workspace_path, kind, label, mode, target_window };
    let created = create_with_title(backend, &spec).await?;

    {
        let mut config = lock(&state.config);
        record_created(&mut config, &created, &spec);
        state.save_config(&config);
    }
    state.trigger_discovery();
    Ok(created.key.as_id())
}

/// 建立工作階段；帶標籤時同步終端機標題。
async fn create_with_title(backend: Arc<dyn TerminalBackend>, spec: &CreateSpec) -> Result<CreatedSession, CueError> {
    let spec = spec.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let created = backend.create(&spec)?;
        if let Some(label) = &spec.label {
            // 標題同步失敗不影響建立結果：標籤已綁在 Cue 的節點上。
            if let Err(e) = sync_title(backend.as_ref(), &created.key, label) {
                eprintln!("[cue] 無法同步新工作階段的標題：{e}");
            }
        }
        Ok(created)
    })
    .await
    .map_err(|e| CueError::Io(format!("建立工作階段的工作異常結束：{e}")))?
}

/// 重新開啟已結束的節點：以相同類型、標籤與最後 cwd 建立新工作階段，回傳新節點的 id。
///
/// 新節點繼承標籤，舊節點與其設定紀錄一併移除。
#[tauri::command]
pub async fn reopen_session(app: AppHandle, id: String) -> Result<String, CueError> {
    let state = app.state::<AppState>();
    let backend = state.backend();
    let spec = {
        let config = lock(&state.config);
        let registry = lock(&state.registry);
        let node = registry.get(&id).ok_or(CueError::ObjectNotFound)?;
        let plan = reopen_plan(node, &config)?;
        let mode = reopen_mode(config.preferred_open_mode(), backend.capabilities());
        let target_window = match (mode, &plan.workspace_id) {
            (OpenMode::NewTab, Some(ws)) => layout::resolve_new_tab_target(ws, &registry),
            _ => None,
        };
        CreateSpec { workspace_path: plan.cwd, kind: plan.kind, label: plan.label, mode, target_window }
    };
    if !spec.workspace_path.is_dir() {
        return Err(CueError::InvalidWorkspace("最後的工作目錄已不存在".into()));
    }
    let created = create_with_title(backend, &spec).await?;

    let new_id = created.key.as_id();
    {
        let mut config = lock(&state.config);
        let mut registry = lock(&state.registry);
        config.forget_node(&id);
        if let Some(label) = &spec.label {
            config.set_node_label(new_id.clone(), label);
        }
        registry.remove(&id);
        state.save_config(&config);
    }
    discovery::publish(&app);
    state.trigger_discovery();
    Ok(new_id)
}

/// 重新開啟所需的參數。
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReopenPlan {
    kind: SessionKindToCreate,
    cwd: PathBuf,
    label: Option<String>,
    /// 舊節點最後的歸屬，用來挑選新分頁的目標視窗。
    workspace_id: Option<String>,
}

/// 由已結束的節點推導重新開啟的參數。只能啟動固定的三種類型，其他程式以 zsh 開回原資料夾。
fn reopen_plan(node: &SessionNode, config: &CueConfig) -> Result<ReopenPlan, CueError> {
    if node.alive {
        return Err(CueError::SessionAlive);
    }
    let kind = match node.kind {
        SessionKind::Claude => SessionKindToCreate::Claude,
        SessionKind::Gemini => SessionKindToCreate::Gemini,
        SessionKind::Zsh | SessionKind::Other(_) => SessionKindToCreate::Zsh,
    };
    let cwd = node
        .cwd
        .clone()
        .or_else(|| config.nodes.get(&node.id).and_then(|m| m.last_cwd.clone()))
        .ok_or_else(|| CueError::InvalidWorkspace("無法取得這個工作階段最後的工作目錄".into()))?;
    Ok(ReopenPlan { kind, cwd, label: node.label.clone(), workspace_id: node.workspace_id.clone() })
}

/// 重新開啟的開啟方式：沿用使用者的預設，但只建立一個工作階段。
/// 分割開啟方式會多開一個 zsh 窗格，改用新分頁；後端不支援分頁時開新視窗。
fn reopen_mode(preferred: Option<OpenMode>, caps: Capabilities) -> OpenMode {
    match preferred {
        Some(OpenMode::NewWindow) => OpenMode::NewWindow,
        _ if caps.new_tab => OpenMode::NewTab,
        _ => OpenMode::NewWindow,
    }
}

/// 移除已結束的節點與它在設定檔中的紀錄（含標籤）。
#[tauri::command]
pub fn remove_node(app: AppHandle, id: String) -> Result<(), CueError> {
    let state = app.state::<AppState>();
    {
        let mut config = lock(&state.config);
        let mut registry = lock(&state.registry);
        let node = registry.get(&id).ok_or(CueError::ObjectNotFound)?;
        if node.alive {
            return Err(CueError::SessionAlive);
        }
        registry.remove(&id);
        if config.forget_node(&id) {
            state.save_config(&config);
        }
    }
    discovery::publish(&app);
    Ok(())
}

/// 以未歸類工作階段的 cwd 加入工作區，回傳工作區 id；該工作階段隨即依 cwd 規則歸入。
#[tauri::command]
pub fn set_as_workspace(app: AppHandle, session_id: String) -> Result<String, CueError> {
    let cwd = {
        let state = app.state::<AppState>();
        let registry = lock(&state.registry);
        let node = registry.get(&session_id).ok_or(CueError::ObjectNotFound)?;
        node.cwd.clone().ok_or_else(|| CueError::InvalidWorkspace("無法取得這個工作階段的工作目錄".into()))?
    };
    add_workspace_path(&app, &cwd)
}

/// 未歸類區的「忽略」：之後不再列入未歸類區（進入工作區後照常顯示）。
#[tauri::command]
pub fn ignore_session(app: AppHandle, session_id: String) -> Result<(), CueError> {
    let state = app.state::<AppState>();
    {
        let mut config = lock(&state.config);
        if lock(&state.registry).get(&session_id).is_none() {
            return Err(CueError::ObjectNotFound);
        }
        if !config.ignore_node(&session_id) {
            return Ok(());
        }
        state.save_config(&config);
    }
    discovery::publish(&app);
    Ok(())
}

/// 從清單移除工作區（不刪除任何檔案），其下的工作階段立即依 cwd 規則重新歸屬。
#[tauri::command]
pub fn remove_workspace(app: AppHandle, id: String) -> Result<(), CueError> {
    let state = app.state::<AppState>();
    {
        let mut config = lock(&state.config);
        if !config.remove_workspace(&id) {
            return Err(CueError::WorkspaceNotFound);
        }
        state.save_config(&config);
        state.sync_git(&config);
        lock(&state.registry).reattribute(&workspace_roots(&config));
    }
    discovery::publish(&app);
    Ok(())
}

/// 重新命名工作階段。`label` 為空字串時清除標籤，節點回到預設名稱。
///
/// 標籤綁在節點身分上、存於設定檔；已結束的節點也可以改，重新開啟時沿用。
/// 存活的節點在支援的後端（iTerm2）同步設定窗格名稱；清除標籤時不還原終端機標題。
#[tauri::command]
pub async fn rename_session(app: AppHandle, id: String, label: String) -> Result<(), CueError> {
    let label = config::normalize_label(&label)?;
    let state = app.state::<AppState>();
    let (key, alive) = {
        let registry = lock(&state.registry);
        let node = registry.get(&id).ok_or(CueError::ObjectNotFound)?;
        (node.key.clone(), node.alive)
    };
    {
        let mut config = lock(&state.config);
        config.set_node_label(id, &label);
        state.save_config(&config);
    }
    // 下一輪探索會依設定檔重算節點標籤並推送快照。
    state.trigger_discovery();

    if !alive || label.is_empty() {
        return Ok(());
    }
    let backend = state.backend();
    let result = tauri::async_runtime::spawn_blocking(move || sync_title(backend.as_ref(), &key, &label))
        .await
        .map_err(|e| CueError::Io(format!("重新命名的工作異常結束：{e}")))?;
    match result {
        // 窗格剛好在這段期間被關閉：標籤已保存，探索會把節點標為已結束。
        Err(CueError::ObjectNotFound) => Ok(()),
        other => other,
    }
}

/// 把標籤同步到終端機標題；後端不支援重新命名時略過（標籤只顯示在 Cue）。
fn sync_title(backend: &dyn TerminalBackend, key: &SessionKey, label: &str) -> Result<(), CueError> {
    if backend.capabilities().rename {
        backend.rename(key, label)
    } else {
        Ok(())
    }
}

/// 確認後端支援這個開啟方式。開新視窗是所有後端都支援的基本操作。
fn check_mode(caps: Capabilities, mode: OpenMode) -> Result<(), CueError> {
    let supported = match mode {
        OpenMode::NewWindow => true,
        OpenMode::NewTab => caps.new_tab,
        OpenMode::AiTopZshBottom | OpenMode::AiLeftZshRight => caps.new_tab && caps.split,
    };
    if supported {
        Ok(())
    } else {
        Err(CueError::Unsupported)
    }
}

/// 關閉工作階段（使用者主動操作）。程式仍在執行時交由終端機自己的確認機制。
///
/// 節點不在這裡標為已結束：使用者可能在確認對話框按取消，一律以探索結果為準。
#[tauri::command]
pub async fn close_session(app: AppHandle, id: String) -> Result<(), CueError> {
    let state = app.state::<AppState>();
    let key = {
        let registry = lock(&state.registry);
        let node = registry.get(&id).ok_or(CueError::ObjectNotFound)?;
        if !node.alive {
            return Err(CueError::SessionExited);
        }
        node.key.clone()
    };
    let backend = state.backend();
    let result = tauri::async_runtime::spawn_blocking(move || backend.close(&key))
        .await
        .map_err(|e| CueError::Io(format!("關閉工作階段的工作異常結束：{e}")))?;
    state.trigger_discovery();
    already_closed_is_ok(result)
}

/// 窗格在探索之間已被關閉，和關閉成功的結果相同。
fn already_closed_is_ok(result: Result<(), CueError>) -> Result<(), CueError> {
    match result {
        Err(CueError::ObjectNotFound) => Ok(()),
        other => other,
    }
}

/// 不支援分割的後端改開新視窗時，回給前端的提示。
const SPLIT_FALLBACK_NOTICE: &str = "後備模式改以新視窗並排";

/// 在工作階段旁補開同工作區的 zsh。原工作階段不送任何輸入。
///
/// 工作區路徑取節點所屬工作區的根目錄；未歸類節點用它的 cwd。
/// 成功時回傳提示文字：後端不支援分割、改開新視窗時為 `Some`。
#[tauri::command]
pub async fn split_zsh(
    app: AppHandle,
    session_id: String,
    direction: SplitDirection,
) -> Result<Option<String>, CueError> {
    let state = app.state::<AppState>();
    let (key, cwd) = {
        // 與探索迴圈相同的上鎖順序（先 config 再 registry），避免互相等待。
        let config = lock(&state.config);
        let registry = lock(&state.registry);
        let node = registry.get(&session_id).ok_or(CueError::ObjectNotFound)?;
        if !node.alive {
            return Err(CueError::SessionExited);
        }
        let cwd = split_target_dir(&config, node.workspace_id.as_deref(), node.cwd.as_deref())?;
        (node.key.clone(), cwd)
    };
    if !cwd.is_dir() {
        return Err(CueError::InvalidWorkspace("工作區資料夾不存在".into()));
    }
    let backend = state.backend();
    let notice = (!backend.capabilities().split).then(|| SPLIT_FALLBACK_NOTICE.to_owned());
    tauri::async_runtime::spawn_blocking(move || backend.split_zsh(&key, direction, &cwd))
        .await
        .map_err(|e| CueError::Io(format!("補開 zsh 的工作異常結束：{e}")))??;
    state.trigger_discovery();
    Ok(notice)
}

/// 補開 zsh 的目標資料夾：已歸屬的節點用工作區根目錄，未歸類節點用 cwd。
fn split_target_dir(
    config: &CueConfig,
    workspace_id: Option<&str>,
    cwd: Option<&Path>,
) -> Result<PathBuf, CueError> {
    match workspace_id {
        Some(id) => config
            .workspaces
            .iter()
            .find(|w| w.id == id)
            .map(|w| w.path.clone())
            .ok_or(CueError::WorkspaceNotFound),
        None => cwd
            .map(Path::to_path_buf)
            .ok_or_else(|| CueError::InvalidWorkspace("無法取得這個工作階段的工作目錄".into())),
    }
}

/// 建立成功後寫回設定：標籤立即綁到新節點（不等下一輪探索），並記住這次的開啟方式。
fn record_created(config: &mut CueConfig, created: &CreatedSession, spec: &CreateSpec) {
    if let Some(label) = &spec.label {
        config.set_node_label(created.key.as_id(), label);
    }
    config.set_last_open_mode(spec.mode);
}

/// 加入工作區：驗證為資料夾、取 realpath、去重。回傳工作區 id（已存在時回傳既有的 id）。
#[tauri::command]
pub fn add_workspace(app: AppHandle, path: String) -> Result<String, CueError> {
    add_workspace_path(&app, Path::new(&path))
}

/// 加入工作區並立即重算既有節點的歸屬，不等下一輪探索。
fn add_workspace_path(app: &AppHandle, path: &Path) -> Result<String, CueError> {
    let state = app.state::<AppState>();
    let (id, added) = {
        let mut config = lock(&state.config);
        let result = insert_workspace(&mut config, path)?;
        if result.1 {
            state.save_config(&config);
            state.sync_git(&config);
            lock(&state.registry).reattribute(&workspace_roots(&config));
        }
        result
    };
    if added {
        discovery::publish(app);
    }
    Ok(id)
}

/// 設定工作區收合狀態。
#[tauri::command]
pub fn set_workspace_collapsed(app: AppHandle, id: String, collapsed: bool) -> Result<(), CueError> {
    let state = app.state::<AppState>();
    {
        let mut config = lock(&state.config);
        let ws = config.workspaces.iter_mut().find(|w| w.id == id).ok_or(CueError::WorkspaceNotFound)?;
        if ws.collapsed == collapsed {
            return Ok(());
        }
        ws.collapsed = collapsed;
        state.save_config(&config);
    }
    discovery::publish(&app);
    Ok(())
}

/// 把資料夾加入設定（純邏輯，方便測試）。回傳 `(id, 是否新加入)`。
fn insert_workspace(config: &mut CueConfig, path: &Path) -> Result<(String, bool), CueError> {
    let real = resolve_dir(path)?;
    if let Some(existing) = config.workspaces.iter().find(|w| w.path == real) {
        return Ok((existing.id.clone(), false));
    }
    let id = config.new_workspace_id();
    config.workspaces.push(Workspace::new(id.clone(), real));
    Ok((id, true))
}

/// 取資料夾的 realpath；必須是絕對路徑且確實為資料夾。
fn resolve_dir(path: &Path) -> Result<PathBuf, CueError> {
    if !path.is_absolute() {
        return Err(CueError::InvalidWorkspace("路徑必須是絕對路徑".into()));
    }
    let real = std::fs::canonicalize(path)
        .map_err(|_| CueError::InvalidWorkspace("找不到這個路徑".into()))?;
    if !real.is_dir() {
        return Err(CueError::InvalidWorkspace("不是資料夾".into()));
    }
    Ok(real)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    use crate::model::DiscoveredSession;

    /// 記錄 `rename` 呼叫的後端。
    struct RenameRecorder {
        caps: Capabilities,
        renamed: Mutex<Vec<(SessionKey, String)>>,
    }

    impl TerminalBackend for RenameRecorder {
        fn id(&self) -> &'static str {
            "recorder"
        }
        fn capabilities(&self) -> Capabilities {
            self.caps
        }
        fn is_running(&self) -> bool {
            true
        }
        fn discover(&self) -> Result<Vec<DiscoveredSession>, CueError> {
            Ok(Vec::new())
        }
        fn focus(&self, _key: &SessionKey) -> Result<(), CueError> {
            Ok(())
        }
        fn rename(&self, key: &SessionKey, name: &str) -> Result<(), CueError> {
            lock(&self.renamed).push((key.clone(), name.to_owned()));
            Ok(())
        }
    }

    #[test]
    fn title_sync_follows_rename_capability() {
        let key = SessionKey::Iterm2 { unique_id: "A".into() };

        let plain = RenameRecorder { caps: Capabilities::default(), renamed: Mutex::default() };
        assert_eq!(sync_title(&plain, &key, "前端"), Ok(()));
        assert!(lock(&plain.renamed).is_empty(), "不支援重新命名的後端不呼叫 rename");

        let caps = Capabilities { rename: true, ..Capabilities::default() };
        let iterm = RenameRecorder { caps, renamed: Mutex::default() };
        assert_eq!(sync_title(&iterm, &key, "前端"), Ok(()));
        assert_eq!(*lock(&iterm.renamed), vec![(key, "前端".to_owned())]);
    }

    #[test]
    fn closing_a_vanished_session_succeeds() {
        assert_eq!(already_closed_is_ok(Ok(())), Ok(()));
        assert_eq!(already_closed_is_ok(Err(CueError::ObjectNotFound)), Ok(()));
        assert_eq!(already_closed_is_ok(Err(CueError::AutomationDenied)), Err(CueError::AutomationDenied));
        let multi = Err(CueError::Script("Terminal.app 無法單獨關閉分頁".into()));
        assert_eq!(already_closed_is_ok(multi.clone()), multi);
    }

    #[test]
    fn mode_must_match_capabilities() {
        let none = Capabilities::default();
        assert_eq!(check_mode(none, OpenMode::NewWindow), Ok(()));
        for mode in [OpenMode::NewTab, OpenMode::AiTopZshBottom, OpenMode::AiLeftZshRight] {
            assert_eq!(check_mode(none, mode), Err(CueError::Unsupported), "{mode:?}");
        }

        let tabs = Capabilities { new_tab: true, ..Capabilities::default() };
        assert_eq!(check_mode(tabs, OpenMode::NewTab), Ok(()));
        assert_eq!(check_mode(tabs, OpenMode::AiTopZshBottom), Err(CueError::Unsupported));

        let full = Capabilities { new_tab: true, split: true, ..Capabilities::default() };
        for mode in [OpenMode::NewWindow, OpenMode::NewTab, OpenMode::AiTopZshBottom, OpenMode::AiLeftZshRight] {
            assert_eq!(check_mode(full, mode), Ok(()), "{mode:?}");
        }
    }

    #[test]
    fn split_uses_workspace_root_or_unclassified_cwd() {
        let mut config = CueConfig::default();
        config.workspaces.push(Workspace::new("ws1".into(), PathBuf::from("/repo")));
        let cwd = Path::new("/repo/src/deep");

        assert_eq!(split_target_dir(&config, Some("ws1"), Some(cwd)), Ok(PathBuf::from("/repo")));
        assert_eq!(split_target_dir(&config, None, Some(cwd)), Ok(cwd.to_path_buf()));
        assert_eq!(split_target_dir(&config, Some("gone"), Some(cwd)), Err(CueError::WorkspaceNotFound));
        assert!(matches!(split_target_dir(&config, None, None), Err(CueError::InvalidWorkspace(_))));
    }

    #[test]
    fn created_session_binds_label_and_remembers_mode() {
        let mut config = CueConfig::default();
        let created = CreatedSession {
            key: SessionKey::Iterm2 { unique_id: "AI".into() },
            zsh: Some(SessionKey::Iterm2 { unique_id: "ZSH".into() }),
        };
        let spec = CreateSpec {
            workspace_path: PathBuf::from("/tmp"),
            kind: SessionKindToCreate::Claude,
            label: Some(" 前端 ".into()),
            mode: OpenMode::AiTopZshBottom,
            target_window: None,
        };
        record_created(&mut config, &created, &spec);
        assert_eq!(config.nodes["iterm2:AI"].label.as_deref(), Some("前端"));
        assert!(!config.nodes.contains_key("iterm2:ZSH"), "標籤只綁 AI 窗格");
        assert_eq!(config.preferred_open_mode(), Some(OpenMode::AiTopZshBottom));

        let created = CreatedSession {
            key: SessionKey::TerminalApp { window_id: 7, tty: "ttys001".into() },
            zsh: None,
        };
        let spec = CreateSpec { label: None, mode: OpenMode::NewWindow, ..spec };
        record_created(&mut config, &created, &spec);
        assert!(!config.nodes.contains_key("terminal:7:ttys001"));
        assert_eq!(config.preferred_open_mode(), Some(OpenMode::NewWindow));
    }

    fn exited_node(kind: SessionKind, cwd: Option<&str>) -> SessionNode {
        SessionNode {
            id: "iterm2:OLD".into(),
            key: SessionKey::Iterm2 { unique_id: "OLD".into() },
            kind,
            status: crate::model::Status::Exited,
            workspace_id: Some("ws1".into()),
            cwd: cwd.map(PathBuf::from),
            tty: "ttys001".into(),
            label: Some("權限架構".into()),
            default_name: "claude".into(),
            location_hint: "視窗 1 · 分頁 1".into(),
            window_id: 1,
            window_index: 1,
            tab_index: 1,
            alive: false,
        }
    }

    #[test]
    fn reopen_keeps_kind_label_and_last_cwd() {
        let config = CueConfig::default();
        let plan = reopen_plan(&exited_node(SessionKind::Claude, Some("/repo/src")), &config).unwrap();
        assert_eq!(
            plan,
            ReopenPlan {
                kind: SessionKindToCreate::Claude,
                cwd: PathBuf::from("/repo/src"),
                label: Some("權限架構".into()),
                workspace_id: Some("ws1".into()),
            }
        );

        let gemini = reopen_plan(&exited_node(SessionKind::Gemini, Some("/r")), &config).unwrap();
        assert_eq!(gemini.kind, SessionKindToCreate::Gemini);
        let zsh = reopen_plan(&exited_node(SessionKind::Zsh, Some("/r")), &config).unwrap();
        assert_eq!(zsh.kind, SessionKindToCreate::Zsh);
        let other = reopen_plan(&exited_node(SessionKind::Other("vim".into()), Some("/r")), &config).unwrap();
        assert_eq!(other.kind, SessionKindToCreate::Zsh, "其他程式以 zsh 開回原資料夾");
    }

    #[test]
    fn reopen_falls_back_to_recorded_cwd_and_rejects_alive_nodes() {
        let mut config = CueConfig::default();
        let node = exited_node(SessionKind::Claude, None);
        assert!(matches!(reopen_plan(&node, &config), Err(CueError::InvalidWorkspace(_))));

        config.nodes.entry(node.id.clone()).or_default().last_cwd = Some(PathBuf::from("/recorded"));
        assert_eq!(reopen_plan(&node, &config).unwrap().cwd, PathBuf::from("/recorded"));

        let alive = SessionNode { alive: true, ..exited_node(SessionKind::Claude, Some("/r")) };
        assert_eq!(reopen_plan(&alive, &config), Err(CueError::SessionAlive));
    }

    #[test]
    fn reopen_opens_a_single_session() {
        let tabs = Capabilities { new_tab: true, split: true, ..Capabilities::default() };
        assert_eq!(reopen_mode(None, tabs), OpenMode::NewTab);
        assert_eq!(reopen_mode(Some(OpenMode::NewWindow), tabs), OpenMode::NewWindow);
        for split in [OpenMode::AiTopZshBottom, OpenMode::AiLeftZshRight] {
            assert_eq!(reopen_mode(Some(split), tabs), OpenMode::NewTab, "{split:?}");
        }

        let none = Capabilities::default();
        for mode in [None, Some(OpenMode::NewTab), Some(OpenMode::AiTopZshBottom)] {
            assert_eq!(reopen_mode(mode, none), OpenMode::NewWindow, "{mode:?}");
        }
    }

    #[test]
    fn adds_workspace_with_realpath_and_folder_name() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("proj");
        std::fs::create_dir(&sub).unwrap();
        let mut config = CueConfig::default();

        let (id, added) = insert_workspace(&mut config, &sub).unwrap();
        assert!(added);
        let ws = &config.workspaces[0];
        assert_eq!(ws.id, id);
        assert_eq!(ws.path, std::fs::canonicalize(&sub).unwrap());
        assert_eq!(ws.name, "proj");
        assert!(!ws.collapsed);
    }

    #[test]
    fn duplicate_paths_are_deduplicated_after_realpath() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("proj");
        std::fs::create_dir(&sub).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&sub, &link).unwrap();
        let mut config = CueConfig::default();

        let (first, _) = insert_workspace(&mut config, &sub).unwrap();
        let (again, added) = insert_workspace(&mut config, &link).unwrap();
        assert!(!added);
        assert_eq!(first, again);
        // 結尾斜線或 `..` 也視為同一個資料夾
        let dotted = dir.path().join("proj/../proj/");
        assert_eq!(insert_workspace(&mut config, &dotted).unwrap(), (first, false));
        assert_eq!(config.workspaces.len(), 1);
    }

    #[test]
    fn rejects_files_missing_and_relative_paths() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.txt");
        std::fs::write(&file, "x").unwrap();
        let mut config = CueConfig::default();

        for path in [file, dir.path().join("missing"), PathBuf::from("relative/dir")] {
            let err = insert_workspace(&mut config, &path).unwrap_err();
            assert_eq!(err.kind(), "invalid_workspace", "{path:?}");
        }
        assert!(config.workspaces.is_empty());
    }
}
