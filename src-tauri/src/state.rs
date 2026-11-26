//! App 全域狀態：設定檔、節點登錄與目前的終端機後端。
//!
//! Rust 端是工作階段狀態的單一真相；前端只透過 [`Snapshot`] 取得狀態。
//! 同時需要設定檔與登錄時，一律**先鎖 `config` 再鎖 `registry`**，避免死結。

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};

use serde::Serialize;
use tokio::sync::Notify;

use crate::attribution::WorkspaceRoots;
use crate::config::{self, ConfigWriter, CueConfig, TerminalBackendPref};
use crate::registry::{Registry, SessionNode};
use crate::terminal::detect::{self, BackendPref, BackendSelection, FallbackReason, TerminalApp};
use crate::terminal::iterm2::ITerm2Backend;
use crate::terminal::terminal_app::TerminalAppBackend;
use crate::terminal::{Capabilities, OpenMode, TerminalBackend};

/// 取得鎖；持有者 panic 過也照常取得（狀態仍可用，不讓整個 App 跟著停擺）。
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// 已安裝的終端機。Terminal.app 為系統內建，固定為 `true`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledTerminals {
    pub iterm2: bool,
    pub terminal_app: bool,
}

/// 目前後端的資訊。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendInfo {
    /// 後端代號（`iterm2`、`terminal`）。
    pub id: &'static str,
    pub capabilities: Capabilities,
    pub installed: InstalledTerminals,
    /// 沒有採用設定檔指定後端的原因。
    pub fallback: Option<FallbackReason>,
}

/// 送到前端的工作區（不含設定檔的保留欄位）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceView {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub collapsed: bool,
}

/// 前端需要的完整狀態。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// 依設定檔順序的工作區。
    pub workspaces: Vec<WorkspaceView>,
    /// 所有節點（含已結束）；以 `workspaceId` 分組，`None` 為未歸類。
    pub sessions: Vec<SessionNode>,
    /// 「未歸類」區要顯示的節點 id（已排除忽略清單與已結束的節點）。
    pub unclassified: Vec<String>,
    pub backend: BackendInfo,
    /// 建立工作階段 Modal 的預設開啟方式（`defaultOpenMode`，其次上次用過的）；都沒有時為 `None`。
    pub preferred_open_mode: Option<OpenMode>,
}

/// 依設定檔與登錄組出快照（純函式，方便測試）。
pub fn build_snapshot(config: &CueConfig, registry: &Registry, backend: BackendInfo) -> Snapshot {
    Snapshot {
        workspaces: config
            .workspaces
            .iter()
            .map(|w| WorkspaceView {
                id: w.id.clone(),
                path: w.path.clone(),
                name: w.name.clone(),
                collapsed: w.collapsed,
            })
            .collect(),
        sessions: registry.nodes().to_vec(),
        unclassified: registry.unclassified(&config.ignored).map(|n| n.id.clone()).collect(),
        backend,
        preferred_open_mode: config.preferred_open_mode(),
    }
}

/// 工作區根目錄（realpath 已解析），供歸屬比對。
pub fn workspace_roots(config: &CueConfig) -> WorkspaceRoots<String> {
    let list: Vec<(String, PathBuf)> =
        config.workspaces.iter().map(|w| (w.id.clone(), w.path.clone())).collect();
    WorkspaceRoots::new(&list)
}

/// 設定檔的後端偏好對應到偵測模組的偏好。
pub fn backend_pref(pref: TerminalBackendPref) -> BackendPref {
    match pref {
        TerminalBackendPref::Auto => BackendPref::Auto,
        TerminalBackendPref::Iterm2 => BackendPref::Iterm2,
        TerminalBackendPref::Terminal => BackendPref::TerminalApp,
    }
}

fn make_backend(app: TerminalApp) -> Arc<dyn TerminalBackend> {
    match app {
        TerminalApp::Iterm2 => Arc::new(ITerm2Backend::new()),
        TerminalApp::TerminalApp => Arc::new(TerminalAppBackend::new()),
    }
}

pub struct AppState {
    pub config: Mutex<CueConfig>,
    pub registry: Mutex<Registry>,
    pub backend: RwLock<Arc<dyn TerminalBackend>>,
    /// 啟動時的後端選擇結果與安裝狀態。
    selection: BackendSelection,
    installed: InstalledTerminals,
    /// 取不到 `HOME` 時為 `None`（只在記憶體中運作，不寫檔）。
    writer: Option<ConfigWriter>,
    /// 要求探索迴圈立即執行下一輪。
    pub(crate) discovery_trigger: Notify,
    /// 最後一次送給前端的快照，用來判斷是否有變動。
    pub(crate) last_emitted: Mutex<Option<Snapshot>>,
}

impl AppState {
    /// 讀取 `~/.cue/config.json`，並依 `terminalBackend` 選定後端（只在啟動時套用）。
    pub fn load() -> Self {
        let path = config::default_config_path();
        let config = match &path {
            Some(p) => config::load(p),
            None => {
                eprintln!("[cue] 取不到 HOME，設定不會被儲存");
                CueConfig::default()
            }
        };
        let writer = path.map(|p| ConfigWriter::spawn(p, config::DEFAULT_DEBOUNCE));

        let selection = detect::select_backend(backend_pref(config.terminal_backend));
        let installed = InstalledTerminals {
            iterm2: detect::is_installed(TerminalApp::Iterm2),
            terminal_app: true,
        };
        Self {
            config: Mutex::new(config),
            registry: Mutex::new(Registry::new()),
            backend: RwLock::new(make_backend(selection.app)),
            selection,
            installed,
            writer,
            discovery_trigger: Notify::new(),
            last_emitted: Mutex::new(None),
        }
    }

    /// 目前的後端。只複製 `Arc`，呼叫端不需持有鎖。
    pub fn backend(&self) -> Arc<dyn TerminalBackend> {
        Arc::clone(&self.backend.read().unwrap_or_else(PoisonError::into_inner))
    }

    pub fn backend_info(&self) -> BackendInfo {
        let backend = self.backend();
        BackendInfo {
            id: backend.id(),
            capabilities: backend.capabilities(),
            installed: self.installed,
            fallback: self.selection.fallback,
        }
    }

    /// 目前狀態的快照。
    pub fn snapshot(&self) -> Snapshot {
        let backend = self.backend_info();
        let config = lock(&self.config);
        let registry = lock(&self.registry);
        build_snapshot(&config, &registry, backend)
    }

    /// 排入一次寫檔（debounce 合併、原子寫入）。
    pub fn save_config(&self, config: &CueConfig) {
        if let Some(w) = &self.writer {
            w.save(config.clone());
        }
    }

    /// 立即寫入尚未寫入的設定（App 結束前）。
    pub fn flush_config(&self) {
        if let Some(w) = &self.writer {
            w.flush();
        }
    }

    /// 要求立即重新探索（建立、關閉、加入工作區、Hook 之後）。
    pub fn trigger_discovery(&self) {
        self.discovery_trigger.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Workspace;
    use crate::model::{Bounds, DiscoveredSession, SessionKey, SessionKind};
    use crate::process::Foreground;
    use serde_json::json;
    use std::collections::HashMap;

    fn backend_info() -> BackendInfo {
        BackendInfo {
            id: "iterm2",
            capabilities: Capabilities::default(),
            installed: InstalledTerminals { iterm2: true, terminal_app: true },
            fallback: None,
        }
    }

    fn iterm(id: &str, tty: &str) -> DiscoveredSession {
        DiscoveredSession {
            key: SessionKey::Iterm2 { unique_id: id.into() },
            tty: tty.into(),
            name: String::new(),
            window_id: 1,
            window_index: 1,
            window_bounds: Bounds::default(),
            tab_index: 1,
        }
    }

    #[test]
    fn backend_pref_mapping() {
        assert_eq!(backend_pref(TerminalBackendPref::Auto), BackendPref::Auto);
        assert_eq!(backend_pref(TerminalBackendPref::Iterm2), BackendPref::Iterm2);
        assert_eq!(backend_pref(TerminalBackendPref::Terminal), BackendPref::TerminalApp);
    }

    #[test]
    fn snapshot_groups_unclassified_and_hides_ignored() {
        let ws = tempfile::tempdir().unwrap();
        let ws_path = std::fs::canonicalize(ws.path()).unwrap();
        let mut config = CueConfig {
            workspaces: vec![Workspace::new("a".into(), ws_path.clone())],
            ignored: vec!["iterm2:C".into()],
            ..CueConfig::default()
        };
        let fg: HashMap<String, Foreground> = [
            ("ttys001".to_string(), Foreground { pid: 11, kind: SessionKind::Claude }),
            ("ttys002".to_string(), Foreground { pid: 12, kind: SessionKind::Zsh }),
            ("ttys003".to_string(), Foreground { pid: 13, kind: SessionKind::Zsh }),
        ]
        .into();
        let cwds: HashMap<i32, PathBuf> = [
            (11, ws_path.clone()),
            (12, PathBuf::from("/")),
            (13, PathBuf::from("/")),
        ]
        .into();
        let mut registry = Registry::new();
        let roots = workspace_roots(&config);
        registry.merge(
            &[iterm("A", "ttys001"), iterm("B", "ttys002"), iterm("C", "ttys003")],
            &fg,
            &cwds,
            &roots,
            &mut config,
        );

        let snap = build_snapshot(&config, &registry, backend_info());
        assert_eq!(snap.workspaces.len(), 1);
        assert_eq!(snap.sessions.len(), 3);
        assert_eq!(snap.unclassified, vec!["iterm2:B".to_string()]);

        let v = serde_json::to_value(&snap).unwrap();
        assert_eq!(v["workspaces"][0]["id"], "a");
        assert_eq!(v["workspaces"][0]["collapsed"], false);
        assert_eq!(v["sessions"][0]["workspaceId"], "a");
        assert_eq!(v["backend"]["id"], "iterm2");
        assert_eq!(v["backend"]["installed"], json!({ "iterm2": true, "terminalApp": true }));
        assert_eq!(v["backend"]["fallback"], json!(null));
        assert_eq!(v["preferredOpenMode"], json!(null));
    }

    #[test]
    fn snapshot_carries_preferred_open_mode() {
        let mut config = CueConfig::default();
        config.set_last_open_mode(OpenMode::NewTab);
        let snap = build_snapshot(&config, &Registry::new(), backend_info());
        assert_eq!(serde_json::to_value(&snap).unwrap()["preferredOpenMode"], json!("newTab"));

        config.preferences.insert("defaultOpenMode".into(), json!("aiTopZshBottom"));
        let snap = build_snapshot(&config, &Registry::new(), backend_info());
        assert_eq!(snap.preferred_open_mode, Some(OpenMode::AiTopZshBottom));
    }

    #[test]
    fn workspace_view_omits_extra_fields() {
        let mut ws = Workspace::new("a".into(), PathBuf::from("/tmp"));
        ws.extra.insert("future".into(), json!(1));
        let config = CueConfig { workspaces: vec![ws], ..CueConfig::default() };
        let snap = build_snapshot(&config, &Registry::new(), backend_info());
        let v = serde_json::to_value(&snap.workspaces[0]).unwrap();
        assert_eq!(v, json!({ "id": "a", "path": "/tmp", "name": "tmp", "collapsed": false }));
    }
}
