//! 前端可呼叫的 Tauri 指令。錯誤一律以 [`CueError`] 序列化為 `{ kind, message }`。
//!
//! 會呼叫 osascript 的指令都是 `async`，在 `spawn_blocking` 中執行，不阻塞主執行緒。

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State};

use crate::config::{CueConfig, Workspace};
use crate::discovery;
use crate::error::CueError;
use crate::state::{lock, AppState, Snapshot};

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
