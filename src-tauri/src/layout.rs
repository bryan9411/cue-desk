//! 建立工作階段時的目標視窗判定。
//!
//! 「專屬視窗」是所有存活窗格都歸屬於同一個工作區的視窗。混裝了未歸類或其他工作區窗格的
//! 視窗不算，避免在使用者另作他用的視窗裡插入新分頁。

use std::collections::{BTreeMap, HashMap};
use std::time::Instant;

use crate::model::WindowId;
use crate::registry::{Registry, SessionNode};

/// 找出工作區的專屬視窗。
///
/// 多個符合時選最近經由 Cue 聚焦過的；都沒聚焦過時選視窗序號最小（最前面）者。
pub fn dedicated_window(
    workspace_id: &str,
    nodes: &[SessionNode],
    focus_history: &HashMap<WindowId, Instant>,
) -> Option<WindowId> {
    // window_id → (是否全屬此工作區, 最小視窗序號)；BTreeMap 讓同分時的結果固定。
    let mut windows: BTreeMap<WindowId, (bool, u32)> = BTreeMap::new();
    for node in nodes.iter().filter(|n| n.alive) {
        let owned = node.workspace_id.as_deref() == Some(workspace_id);
        let entry = windows.entry(node.window_id).or_insert((true, node.window_index));
        entry.0 &= owned;
        entry.1 = entry.1.min(node.window_index);
    }

    windows
        .into_iter()
        .filter(|(_, (owned, _))| *owned)
        .min_by_key(|(id, (_, index))| {
            // 聚焦過的排在前面且越新越前；Reverse(None) 比任何 Reverse(Some) 大。
            (std::cmp::Reverse(focus_history.get(id).copied()), *index)
        })
        .map(|(id, _)| id)
}

/// 「新分頁」的目標視窗：有專屬視窗就用它；`None` 表示交給腳本使用最前視窗
/// （沒有任何視窗時由腳本改開新視窗）。
pub fn resolve_new_tab_target(workspace_id: &str, registry: &Registry) -> Option<WindowId> {
    dedicated_window(workspace_id, registry.nodes(), registry.focus_history())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{SessionKey, SessionKind, Status};
    use std::time::Duration;

    fn pane(id: &str, workspace: Option<&str>, window_id: WindowId, window_index: u32) -> SessionNode {
        SessionNode {
            id: format!("iterm2:{id}"),
            key: SessionKey::Iterm2 { unique_id: id.into() },
            kind: SessionKind::Zsh,
            status: Status::Idle,
            workspace_id: workspace.map(str::to_string),
            cwd: None,
            tty: String::new(),
            label: None,
            default_name: "zsh".into(),
            location_hint: String::new(),
            window_id,
            window_index,
            tab_index: 1,
            alive: true,
        }
    }

    fn exited(mut node: SessionNode) -> SessionNode {
        node.alive = false;
        node.status = Status::Exited;
        node
    }

    fn none() -> HashMap<WindowId, Instant> {
        HashMap::new()
    }

    #[test]
    fn window_with_only_workspace_panes_is_dedicated() {
        let nodes = [pane("A", Some("a"), 10, 1), pane("B", Some("a"), 10, 1)];
        assert_eq!(dedicated_window("a", &nodes, &none()), Some(10));
    }

    #[test]
    fn window_mixed_with_other_workspace_is_excluded() {
        let nodes = [pane("A", Some("a"), 10, 1), pane("B", Some("b"), 10, 1)];
        assert_eq!(dedicated_window("a", &nodes, &none()), None);
        assert_eq!(dedicated_window("b", &nodes, &none()), None);
    }

    #[test]
    fn unclassified_pane_disqualifies_window() {
        let nodes = [
            pane("A", Some("a"), 10, 1),
            pane("B", None, 10, 1),
            pane("C", Some("a"), 20, 2),
        ];
        assert_eq!(dedicated_window("a", &nodes, &none()), Some(20));
    }

    #[test]
    fn exited_panes_are_ignored() {
        let nodes = [pane("A", Some("a"), 10, 1), exited(pane("B", None, 10, 1))];
        assert_eq!(dedicated_window("a", &nodes, &none()), Some(10));

        let only_exited = [exited(pane("A", Some("a"), 10, 1))];
        assert_eq!(dedicated_window("a", &only_exited, &none()), None);
    }

    #[test]
    fn most_recently_focused_candidate_wins() {
        let nodes = [
            pane("A", Some("a"), 10, 1),
            pane("B", Some("a"), 20, 2),
            pane("C", Some("a"), 30, 3),
        ];
        let now = Instant::now();
        let history = HashMap::from([(20, now), (30, now + Duration::from_secs(1))]);
        assert_eq!(dedicated_window("a", &nodes, &history), Some(30));
    }

    #[test]
    fn focused_candidate_beats_unfocused_front_window() {
        let nodes = [pane("A", Some("a"), 10, 1), pane("B", Some("a"), 20, 2)];
        let history = HashMap::from([(20, Instant::now())]);
        assert_eq!(dedicated_window("a", &nodes, &history), Some(20));
    }

    #[test]
    fn unfocused_candidates_pick_smallest_window_index() {
        let nodes = [pane("A", Some("a"), 10, 3), pane("B", Some("a"), 20, 1), pane("C", Some("a"), 30, 2)];
        assert_eq!(dedicated_window("a", &nodes, &none()), Some(20));
    }

    #[test]
    fn focus_of_non_candidate_window_is_irrelevant() {
        let nodes = [pane("A", Some("a"), 10, 2), pane("B", Some("b"), 20, 1)];
        let history = HashMap::from([(20, Instant::now())]);
        assert_eq!(dedicated_window("a", &nodes, &history), Some(10));
    }

    #[test]
    fn no_candidate_returns_none() {
        assert_eq!(dedicated_window("a", &[], &none()), None);
        let nodes = [pane("A", Some("b"), 10, 1)];
        assert_eq!(dedicated_window("a", &nodes, &none()), None);
        assert_eq!(resolve_new_tab_target("a", &Registry::new()), None);
    }
}
