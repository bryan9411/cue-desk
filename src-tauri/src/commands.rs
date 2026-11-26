//! 前端可呼叫的 Tauri 指令。錯誤一律以 [`CueError`] 序列化為 `{ kind, message }`。
//!
//! 會呼叫 osascript 的指令都是 `async`，在 `spawn_blocking` 中執行，不阻塞主執行緒。

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State};

use crate::config::{CueConfig, Workspace};
use crate::discovery;
use crate::error::CueError;
use crate::layout;
use crate::shell::SessionKindToCreate;
use crate::state::{lock, AppState, Snapshot};
use crate::terminal::{Capabilities, CreateSpec, CreatedSession, OpenMode};

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
    let (created, spec) = tauri::async_runtime::spawn_blocking(move || {
        let created = backend.create(&spec);
        (created, spec)
    })
    .await
    .map_err(|e| CueError::Io(format!("建立工作階段的工作異常結束：{e}")))?;
    let created = created?;

    {
        let mut config = lock(&state.config);
        record_created(&mut config, &created, &spec);
        state.save_config(&config);
    }
    state.trigger_discovery();
    Ok(created.key.as_id())
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
    let state = app.state::<AppState>();
    let (id, added) = {
        let mut config = lock(&state.config);
        let result = insert_workspace(&mut config, Path::new(&path))?;
        if result.1 {
            state.save_config(&config);
        }
        result
    };
    if added {
        discovery::publish(&app);
        state.trigger_discovery();
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

    use crate::model::SessionKey;

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
