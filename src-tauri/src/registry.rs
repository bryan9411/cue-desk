//! 節點登錄：合併每輪探索結果，管理節點生命週期並回寫設定檔的節點資訊。
//!
//! - 節點身分一律用 [`SessionKey`]；視窗序號、分頁序號只用於位置提示。
//! - 消失的節點標為 `Exited` 並保留，直到使用者移除。iTerm2 重啟後所有 `unique ID`
//!   都會改變，舊節點因此全部變成 `Exited`，新節點依歸屬規則重新認領，不做跨 ID 的猜測對應。
//! - `Running`／`Waiting`／`Done` 只由 Hook 設定；合併只會在工作階段消失時改為 `Exited`。

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;

use crate::attribution::WorkspaceRoots;
use crate::config::{CueConfig, NodeMeta};
use crate::model::{DiscoveredSession, SessionKey, SessionKind, Status, WindowId};
use crate::process::Foreground;

/// 側邊欄上的一個工作階段節點。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNode {
    /// `SessionKey` 的穩定字串形式，同時是前端節點 id 與設定檔 `nodes` 的鍵。
    pub id: String,
    #[serde(skip)]
    pub key: SessionKey,
    pub kind: SessionKind,
    pub status: Status,
    /// 所屬工作區；`None` 為未歸類。
    pub workspace_id: Option<String>,
    /// 前景程式的工作目錄；無法取得時為 `None`。
    pub cwd: Option<PathBuf>,
    pub tty: String,
    /// 使用者自訂標籤（來自設定檔）。
    pub label: Option<String>,
    /// 類型名稱（`claude`、`gemini`、`zsh` 或執行檔名）。
    pub default_name: String,
    /// iTerm2：「視窗 N · 分頁 M」；Terminal.app：「視窗 N」。只用於顯示。
    pub location_hint: String,
    pub window_id: WindowId,
    /// 視窗由前到後的序號（1 起算），聚焦或切換視窗後會改變。
    #[serde(skip)]
    pub window_index: u32,
    pub tab_index: u32,
    /// 最近一次探索是否仍存在。
    pub alive: bool,
}

impl SessionNode {
    /// 顯示名稱：有自訂標籤時用標籤，否則用類型名稱。
    pub fn name(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.default_name)
    }
}

/// 一次合併的結果，供探索迴圈決定是否送出事件與寫檔。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MergeOutcome {
    /// 節點清單有變動（需要送出 `sessions-updated`）。
    pub nodes_changed: bool,
    /// 設定檔的 `nodes` 有變動（需要寫檔）。
    pub config_changed: bool,
}

/// 所有節點，保持出現順序（既有節點位置不變，新節點接在後面）。
#[derive(Debug, Clone, Default)]
pub struct Registry {
    nodes: Vec<SessionNode>,
    /// 經由 Cue 聚焦過的視窗與最後聚焦時間，用來在多個候選視窗中挑選。
    focus_history: HashMap<WindowId, Instant>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nodes(&self) -> &[SessionNode] {
        &self.nodes
    }

    pub fn get(&self, id: &str) -> Option<&SessionNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn focus_history(&self) -> &HashMap<WindowId, Instant> {
        &self.focus_history
    }

    /// 記錄視窗剛被聚焦。
    pub fn record_focus(&mut self, window_id: WindowId) {
        self.focus_history.insert(window_id, Instant::now());
    }

    /// 合併一輪探索結果。
    ///
    /// - `foregrounds`：以 tty 為鍵的前景程式（[`crate::process::foreground_by_tty`]）。
    /// - `cwds`：以前景程式 pid 為鍵的工作目錄（[`crate::process::cwds`]）。
    /// - 終端機未執行時傳入空的 `discovered`，所有存活節點即標為 `Exited`。
    ///
    /// 前景程式暫時查不到（`ps` 取樣競態）時沿用既有類型；cwd 暫時取不到時沿用既有 cwd，
    /// 新節點則歸為未歸類，下一輪再補。
    pub fn merge(
        &mut self,
        discovered: &[DiscoveredSession],
        foregrounds: &HashMap<String, Foreground>,
        cwds: &HashMap<i32, PathBuf>,
        roots: &WorkspaceRoots<String>,
        config: &mut CueConfig,
    ) -> MergeOutcome {
        let before = self.nodes.clone();
        let mut index: HashMap<SessionKey, usize> =
            self.nodes.iter().enumerate().map(|(i, n)| (n.key.clone(), i)).collect();
        let mut seen: HashSet<SessionKey> = HashSet::new();

        for d in discovered {
            // 同一身分重複回報時只取第一筆。
            if !seen.insert(d.key.clone()) {
                continue;
            }
            let fg = foregrounds.get(&d.tty);
            let cwd = fg.and_then(|f| cwds.get(&f.pid)).cloned();

            if let Some(&i) = index.get(&d.key) {
                let node = &mut self.nodes[i];
                if let Some(fg) = fg {
                    node.kind = fg.kind.clone();
                }
                if cwd.is_some() {
                    node.cwd = cwd;
                }
                node.tty = d.tty.clone();
                node.location_hint = location_hint(d);
                node.window_id = d.window_id;
                node.window_index = d.window_index;
                node.tab_index = d.tab_index;
                if !node.alive {
                    // 同一身分再次出現（例如探索暫時失敗後恢復），視為新的閒置工作階段。
                    node.alive = true;
                    node.status = Status::Idle;
                }
            } else {
                let id = d.key.as_id();
                let kind = fg
                    .map(|f| f.kind.clone())
                    .or_else(|| config.nodes.get(&id).and_then(|m| m.last_kind.clone()))
                    .unwrap_or(SessionKind::Zsh);
                index.insert(d.key.clone(), self.nodes.len());
                self.nodes.push(SessionNode {
                    id,
                    key: d.key.clone(),
                    default_name: default_name(&kind),
                    kind,
                    status: Status::Idle,
                    workspace_id: None,
                    cwd,
                    tty: d.tty.clone(),
                    label: None,
                    location_hint: location_hint(d),
                    window_id: d.window_id,
                    window_index: d.window_index,
                    tab_index: d.tab_index,
                    alive: true,
                });
            }
        }

        for node in self.nodes.iter_mut().filter(|n| n.alive && !seen.contains(&n.key)) {
            node.alive = false;
            node.status = Status::Exited;
        }

        // 歸屬、標籤與名稱每輪重算；已結束的節點以最後 cwd 重新歸屬，工作區移除後不會指向不存在的 id。
        for node in &mut self.nodes {
            node.workspace_id = node.cwd.as_deref().and_then(|cwd| roots.attribute(cwd));
            node.label = config
                .nodes
                .get(&node.id)
                .and_then(|m| m.label.clone())
                .filter(|l| !l.trim().is_empty());
            node.default_name = default_name(&node.kind);
        }

        // 視窗關閉後其 ID 不會再出現，留著只會累積。
        let open_windows: HashSet<WindowId> =
            self.nodes.iter().filter(|n| n.alive).map(|n| n.window_id).collect();
        self.focus_history.retain(|w, _| open_windows.contains(w));

        let config_changed = sync_config(&self.nodes, config);
        MergeOutcome { nodes_changed: self.nodes != before, config_changed }
    }

    /// 移除已結束的節點（使用者按「移除」）。存活的節點不可移除，回傳 `false`。
    pub fn remove(&mut self, id: &str) -> bool {
        let Some(i) = self.nodes.iter().position(|n| n.id == id && !n.alive) else {
            return false;
        };
        self.nodes.remove(i);
        true
    }

    /// 快照的「未歸類」區：存活、不屬於任何工作區、且不在忽略清單中的節點。
    /// 已結束的未歸類節點沒有可用的操作，不列入。
    pub fn unclassified<'a>(&'a self, ignored: &'a [String]) -> impl Iterator<Item = &'a SessionNode> {
        self.nodes
            .iter()
            .filter(move |n| n.alive && n.workspace_id.is_none() && !ignored.contains(&n.id))
    }

    /// 屬於指定工作區的節點（含已結束）。
    pub fn in_workspace<'a>(&'a self, workspace_id: &'a str) -> impl Iterator<Item = &'a SessionNode> {
        self.nodes.iter().filter(move |n| n.workspace_id.as_deref() == Some(workspace_id))
    }
}

/// 類型名稱。
fn default_name(kind: &SessionKind) -> String {
    match kind {
        SessionKind::Claude => "claude".into(),
        SessionKind::Gemini => "gemini".into(),
        SessionKind::Zsh => "zsh".into(),
        SessionKind::Other(name) => name.clone(),
    }
}

/// 位置提示。Terminal.app 的原生分頁在 AppleScript 中各自是一個視窗，分頁序號永遠是 1，不顯示。
fn location_hint(d: &DiscoveredSession) -> String {
    match d.key {
        SessionKey::Iterm2 { .. } => format!("視窗 {} · 分頁 {}", d.window_index, d.tab_index),
        SessionKey::TerminalApp { .. } => format!("視窗 {}", d.window_index),
    }
}

/// 回寫存活節點的最後已知 tty、cwd、類型，並清掉沒有標籤且已不在登錄中的紀錄，
/// 避免每次 iTerm2 重啟後的舊 ID 無限累積。有任何變動時回傳 `true`。
fn sync_config(nodes: &[SessionNode], config: &mut CueConfig) -> bool {
    let mut changed = false;

    for node in nodes.iter().filter(|n| n.alive) {
        let meta = config.nodes.entry(node.id.clone()).or_insert_with(|| {
            changed = true;
            NodeMeta::default()
        });
        changed |= replace(&mut meta.last_tty, Some(node.tty.clone()));
        if node.cwd.is_some() {
            changed |= replace(&mut meta.last_cwd, node.cwd.clone());
        }
        changed |= replace(&mut meta.last_kind, Some(node.kind.clone()));
    }

    let known: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let before = config.nodes.len();
    config.nodes.retain(|id, meta| meta.label.is_some() || known.contains(id.as_str()));
    changed | (config.nodes.len() != before)
}

fn replace<T: PartialEq>(slot: &mut T, value: T) -> bool {
    if *slot == value {
        false
    } else {
        *slot = value;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Workspace;
    use crate::model::Bounds;
    use std::path::Path;

    // 不存在的路徑：realpath 失敗時退回字面路徑，可直接以字面前綴測試歸屬。
    const WS_A: &str = "/cue-registry-test/proj-a";
    const WS_B: &str = "/cue-registry-test/proj-b";
    const HOME: &str = "/cue-registry-test/home";

    fn iterm(id: &str, tty: &str, window_index: u32, tab_index: u32) -> DiscoveredSession {
        DiscoveredSession {
            key: SessionKey::Iterm2 { unique_id: id.into() },
            tty: tty.into(),
            name: String::new(),
            window_id: 100 + i64::from(window_index),
            window_index,
            window_bounds: Bounds::default(),
            tab_index,
        }
    }

    fn terminal(window_id: WindowId, tty: &str, window_index: u32) -> DiscoveredSession {
        DiscoveredSession {
            key: SessionKey::TerminalApp { window_id, tty: tty.into() },
            tty: tty.into(),
            name: String::new(),
            window_id,
            window_index,
            window_bounds: Bounds::default(),
            tab_index: 1,
        }
    }

    /// 一輪探索的程序資訊：tty → (pid, 類型, cwd)。
    #[derive(Default)]
    struct Procs {
        fg: HashMap<String, Foreground>,
        cwds: HashMap<i32, PathBuf>,
    }

    impl Procs {
        fn with(mut self, tty: &str, pid: i32, kind: SessionKind, cwd: Option<&str>) -> Self {
            self.fg.insert(tty.into(), Foreground { pid, kind });
            if let Some(cwd) = cwd {
                self.cwds.insert(pid, PathBuf::from(cwd));
            }
            self
        }
    }

    fn config() -> CueConfig {
        CueConfig {
            workspaces: vec![
                Workspace::new("a".into(), PathBuf::from(WS_A)),
                Workspace::new("b".into(), PathBuf::from(WS_B)),
            ],
            ..CueConfig::default()
        }
    }

    fn roots(config: &CueConfig) -> WorkspaceRoots<String> {
        let list: Vec<(String, PathBuf)> =
            config.workspaces.iter().map(|w| (w.id.clone(), w.path.clone())).collect();
        WorkspaceRoots::new(&list)
    }

    fn merge(reg: &mut Registry, cfg: &mut CueConfig, discovered: &[DiscoveredSession], procs: &Procs) -> MergeOutcome {
        let roots = roots(cfg);
        reg.merge(discovered, &procs.fg, &procs.cwds, &roots, cfg)
    }

    fn node<'a>(reg: &'a Registry, id: &str) -> &'a SessionNode {
        reg.get(id).unwrap_or_else(|| panic!("找不到節點 {id}"))
    }

    #[test]
    fn new_sessions_are_idle_attributed_and_named() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default()
            .with("ttys001", 11, SessionKind::Claude, Some(&format!("{WS_A}/src")))
            .with("ttys002", 12, SessionKind::Zsh, Some(HOME));
        let out = merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 2), iterm("B", "ttys002", 2, 1)], &procs);

        assert!(out.nodes_changed);
        let a = node(&reg, "iterm2:A");
        assert_eq!(a.status, Status::Idle);
        assert!(a.alive);
        assert_eq!(a.kind, SessionKind::Claude);
        assert_eq!(a.workspace_id.as_deref(), Some("a"));
        assert_eq!(a.default_name, "claude");
        assert_eq!(a.name(), "claude");
        assert_eq!(a.location_hint, "視窗 1 · 分頁 2");
        assert_eq!(a.cwd.as_deref(), Some(Path::new(&format!("{WS_A}/src"))));

        let b = node(&reg, "iterm2:B");
        assert_eq!(b.workspace_id, None);
        assert_eq!(b.default_name, "zsh");
        assert_eq!(reg.nodes().iter().map(|n| n.id.as_str()).collect::<Vec<_>>(), ["iterm2:A", "iterm2:B"]);
    }

    #[test]
    fn terminal_app_hint_shows_window_only() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default().with("ttys005", 15, SessionKind::Other("vim".into()), Some(WS_B));
        merge(&mut reg, &mut cfg, &[terminal(4242, "ttys005", 3)], &procs);

        let n = node(&reg, "terminal:4242:ttys005");
        assert_eq!(n.location_hint, "視窗 3");
        assert_eq!(n.default_name, "vim");
        assert_eq!(n.workspace_id.as_deref(), Some("b"));
    }

    #[test]
    fn existing_node_updates_fields_and_unchanged_round_reports_no_change() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let zsh = Procs::default().with("ttys001", 11, SessionKind::Zsh, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &zsh);

        let again = merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &zsh);
        assert_eq!(again, MergeOutcome::default());

        // 視窗前後順序改變、前景改為 claude
        let claude = Procs::default().with("ttys001", 20, SessionKind::Claude, Some(WS_A));
        let out = merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 2, 3)], &claude);
        assert!(out.nodes_changed);
        let a = node(&reg, "iterm2:A");
        assert_eq!(a.kind, SessionKind::Claude);
        assert_eq!(a.default_name, "claude");
        assert_eq!(a.location_hint, "視窗 2 · 分頁 3");
        assert_eq!(reg.nodes().len(), 1);
    }

    #[test]
    fn vanished_node_becomes_exited_and_is_kept() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default()
            .with("ttys001", 11, SessionKind::Claude, Some(WS_A))
            .with("ttys002", 12, SessionKind::Zsh, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1), iterm("B", "ttys002", 1, 2)], &procs);

        let out = merge(&mut reg, &mut cfg, &[iterm("B", "ttys002", 1, 1)], &procs);
        assert!(out.nodes_changed);
        let a = node(&reg, "iterm2:A");
        assert_eq!(a.status, Status::Exited);
        assert!(!a.alive);
        assert_eq!(a.workspace_id.as_deref(), Some("a"));
        assert_eq!(a.location_hint, "視窗 1 · 分頁 1");
        assert_eq!(reg.nodes().len(), 2);
    }

    #[test]
    fn hook_statuses_are_preserved_until_exit() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default().with("ttys001", 11, SessionKind::Claude, Some(WS_A));
        let found = [iterm("A", "ttys001", 1, 1)];
        merge(&mut reg, &mut cfg, &found, &procs);

        for status in [Status::Running, Status::Waiting, Status::Done] {
            reg.nodes[0].status = status;
            let out = merge(&mut reg, &mut cfg, &found, &procs);
            assert!(!out.nodes_changed);
            assert_eq!(reg.nodes[0].status, status);
        }

        merge(&mut reg, &mut cfg, &[], &procs);
        assert_eq!(reg.nodes[0].status, Status::Exited);
    }

    #[test]
    fn reappearing_exited_node_becomes_idle() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default().with("ttys001", 11, SessionKind::Zsh, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &procs);
        merge(&mut reg, &mut cfg, &[], &procs);
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &procs);

        let a = node(&reg, "iterm2:A");
        assert!(a.alive);
        assert_eq!(a.status, Status::Idle);
    }

    #[test]
    fn iterm2_restart_exits_all_and_claims_new_ids() {
        let mut reg = Registry::new();
        let mut cfg = config();
        cfg.nodes.insert("iterm2:OLD1".into(), NodeMeta { label: Some("前端".into()), ..NodeMeta::default() });
        let before = Procs::default()
            .with("ttys001", 11, SessionKind::Claude, Some(WS_A))
            .with("ttys002", 12, SessionKind::Zsh, Some(WS_B));
        merge(&mut reg, &mut cfg, &[iterm("OLD1", "ttys001", 1, 1), iterm("OLD2", "ttys002", 1, 2)], &before);

        // 重啟後 tty 剛好沿用，仍不得對應到舊節點。
        let after = Procs::default()
            .with("ttys001", 31, SessionKind::Claude, Some(WS_A))
            .with("ttys002", 32, SessionKind::Zsh, Some(WS_B));
        merge(&mut reg, &mut cfg, &[iterm("NEW1", "ttys001", 1, 1), iterm("NEW2", "ttys002", 1, 2)], &after);

        assert_eq!(reg.nodes().len(), 4);
        for old in ["iterm2:OLD1", "iterm2:OLD2"] {
            assert_eq!(node(&reg, old).status, Status::Exited);
        }
        let new1 = node(&reg, "iterm2:NEW1");
        assert_eq!(new1.status, Status::Idle);
        assert_eq!(new1.workspace_id.as_deref(), Some("a"));
        assert_eq!(new1.label, None, "標籤不跨 ID 沿用");
        assert_eq!(node(&reg, "iterm2:NEW2").workspace_id.as_deref(), Some("b"));
        assert_eq!(node(&reg, "iterm2:OLD1").label.as_deref(), Some("前端"));
    }

    #[test]
    fn cd_changes_attribution_and_falls_back_to_unclassified() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let found = [iterm("A", "ttys001", 1, 1)];
        let mut at = |cwd: &str| {
            let procs = Procs::default().with("ttys001", 11, SessionKind::Zsh, Some(cwd));
            merge(&mut reg, &mut cfg, &found, &procs);
            reg.nodes[0].workspace_id.clone()
        };

        assert_eq!(at(HOME), None);
        assert_eq!(at(&format!("{WS_A}/deep/dir")).as_deref(), Some("a"));
        assert_eq!(at(WS_B).as_deref(), Some("b"));
        assert_eq!(at("/cue-registry-test"), None);
    }

    #[test]
    fn missing_process_info_keeps_previous_values() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let found = [iterm("A", "ttys001", 1, 1)];
        let procs = Procs::default().with("ttys001", 11, SessionKind::Claude, Some(WS_A));
        merge(&mut reg, &mut cfg, &found, &procs);

        // 前景程式與 cwd 都暫時查不到
        let out = merge(&mut reg, &mut cfg, &found, &Procs::default());
        assert!(!out.nodes_changed);
        let a = node(&reg, "iterm2:A");
        assert_eq!(a.kind, SessionKind::Claude);
        assert_eq!(a.workspace_id.as_deref(), Some("a"));

        // 新節點查不到程序資訊：類型預設 zsh、未歸類
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1), iterm("B", "ttys009", 1, 2)], &procs);
        let b = node(&reg, "iterm2:B");
        assert_eq!(b.kind, SessionKind::Zsh);
        assert_eq!(b.workspace_id, None);
        assert_eq!(b.cwd, None);
    }

    #[test]
    fn label_is_restored_from_config() {
        let mut reg = Registry::new();
        let mut cfg = config();
        cfg.nodes.insert(
            "iterm2:A".into(),
            NodeMeta { label: Some("後端 API".into()), last_kind: Some(SessionKind::Gemini), ..NodeMeta::default() },
        );
        cfg.nodes.insert("iterm2:B".into(), NodeMeta { label: Some("  ".into()), ..NodeMeta::default() });

        // A 的前景程式暫時查不到：類型取設定檔最後已知值
        let procs = Procs::default().with("ttys002", 12, SessionKind::Zsh, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1), iterm("B", "ttys002", 1, 2)], &procs);

        let a = node(&reg, "iterm2:A");
        assert_eq!(a.label.as_deref(), Some("後端 API"));
        assert_eq!(a.name(), "後端 API");
        assert_eq!(a.kind, SessionKind::Gemini);
        assert_eq!(a.default_name, "gemini");
        assert_eq!(node(&reg, "iterm2:B").label, None, "空白標籤視為沒有標籤");
    }

    #[test]
    fn ignored_unclassified_nodes_are_hidden() {
        let mut reg = Registry::new();
        let mut cfg = config();
        cfg.ignored = vec!["iterm2:B".into(), "iterm2:C".into()];
        let procs = Procs::default()
            .with("ttys001", 11, SessionKind::Zsh, Some(HOME))
            .with("ttys002", 12, SessionKind::Zsh, Some(HOME))
            .with("ttys003", 13, SessionKind::Zsh, Some(WS_A));
        merge(
            &mut reg,
            &mut cfg,
            &[iterm("A", "ttys001", 1, 1), iterm("B", "ttys002", 1, 2), iterm("C", "ttys003", 1, 3)],
            &procs,
        );

        let ids: Vec<&str> = reg.unclassified(&cfg.ignored).map(|n| n.id.as_str()).collect();
        assert_eq!(ids, ["iterm2:A"]);
        // 被忽略的節點進入工作區後照常顯示
        let ws: Vec<&str> = reg.in_workspace("a").map(|n| n.id.as_str()).collect();
        assert_eq!(ws, ["iterm2:C"]);

        // 已結束的未歸類節點不列入未歸類區
        merge(&mut reg, &mut cfg, &[iterm("B", "ttys002", 1, 2), iterm("C", "ttys003", 1, 3)], &procs);
        assert_eq!(reg.unclassified(&cfg.ignored).count(), 0);
    }

    #[test]
    fn config_nodes_are_written_back_only_on_change() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let found = [iterm("A", "ttys001", 1, 1)];
        let procs = Procs::default().with("ttys001", 11, SessionKind::Claude, Some(WS_A));

        let out = merge(&mut reg, &mut cfg, &found, &procs);
        assert!(out.config_changed);
        let meta = &cfg.nodes["iterm2:A"];
        assert_eq!(meta.last_tty.as_deref(), Some("ttys001"));
        assert_eq!(meta.last_cwd.as_deref(), Some(Path::new(WS_A)));
        assert_eq!(meta.last_kind, Some(SessionKind::Claude));

        assert!(!merge(&mut reg, &mut cfg, &found, &procs).config_changed);

        // cwd 取不到時不覆寫最後已知 cwd
        assert!(!merge(&mut reg, &mut cfg, &found, &Procs::default()).config_changed);
        assert_eq!(cfg.nodes["iterm2:A"].last_cwd.as_deref(), Some(Path::new(WS_A)));

        let moved = Procs::default().with("ttys001", 11, SessionKind::Zsh, Some(WS_B));
        assert!(merge(&mut reg, &mut cfg, &found, &moved).config_changed);
        assert_eq!(cfg.nodes["iterm2:A"].last_cwd.as_deref(), Some(Path::new(WS_B)));
        assert_eq!(cfg.nodes["iterm2:A"].last_kind, Some(SessionKind::Zsh));
    }

    #[test]
    fn stale_unlabeled_config_nodes_are_pruned() {
        let mut reg = Registry::new();
        let mut cfg = config();
        cfg.nodes.insert("iterm2:GONE".into(), NodeMeta { last_tty: Some("ttys009".into()), ..NodeMeta::default() });
        cfg.nodes.insert("iterm2:NAMED".into(), NodeMeta { label: Some("保留".into()), ..NodeMeta::default() });
        let procs = Procs::default().with("ttys001", 11, SessionKind::Zsh, Some(WS_A));

        let out = merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &procs);
        assert!(out.config_changed);
        assert!(!cfg.nodes.contains_key("iterm2:GONE"));
        assert!(cfg.nodes.contains_key("iterm2:NAMED"));
        assert!(cfg.nodes.contains_key("iterm2:A"));

        // 已結束但仍在登錄中的節點保留紀錄；移除後才清掉
        merge(&mut reg, &mut cfg, &[], &procs);
        assert!(cfg.nodes.contains_key("iterm2:A"));
        assert!(reg.remove("iterm2:A"));
        assert!(merge(&mut reg, &mut cfg, &[], &procs).config_changed);
        assert!(!cfg.nodes.contains_key("iterm2:A"));
    }

    #[test]
    fn only_exited_nodes_can_be_removed() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default().with("ttys001", 11, SessionKind::Zsh, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &procs);

        assert!(!reg.remove("iterm2:A"));
        assert!(!reg.remove("iterm2:NOPE"));
        merge(&mut reg, &mut cfg, &[], &procs);
        assert!(reg.remove("iterm2:A"));
        assert!(reg.nodes().is_empty());
    }

    #[test]
    fn removed_workspace_unattributes_exited_nodes() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default().with("ttys001", 11, SessionKind::Claude, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &procs);
        merge(&mut reg, &mut cfg, &[], &procs);

        cfg.workspaces.retain(|w| w.id != "a");
        merge(&mut reg, &mut cfg, &[], &procs);
        assert_eq!(node(&reg, "iterm2:A").workspace_id, None);
    }

    #[test]
    fn node_serializes_camel_case_without_key() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default().with("ttys001", 11, SessionKind::Claude, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &procs);

        let v = serde_json::to_value(&reg.nodes()[0]).unwrap();
        assert_eq!(v["id"], "iterm2:A");
        assert_eq!(v["workspaceId"], "a");
        assert_eq!(v["defaultName"], "claude");
        assert_eq!(v["locationHint"], "視窗 1 · 分頁 1");
        assert_eq!(v["status"], "idle");
        assert!(v.get("key").is_none());
        assert!(v.get("windowIndex").is_none());
    }

    #[test]
    fn focus_history_drops_closed_windows() {
        let mut reg = Registry::new();
        let mut cfg = config();
        let procs = Procs::default()
            .with("ttys001", 11, SessionKind::Zsh, Some(WS_A))
            .with("ttys002", 12, SessionKind::Zsh, Some(WS_A));
        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1), iterm("B", "ttys002", 2, 1)], &procs);
        reg.record_focus(101);
        reg.record_focus(102);

        merge(&mut reg, &mut cfg, &[iterm("A", "ttys001", 1, 1)], &procs);
        assert!(reg.focus_history().contains_key(&101));
        assert!(!reg.focus_history().contains_key(&102));
    }
}
