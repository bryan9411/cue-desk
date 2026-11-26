import { beforeEach, describe, expect, it } from "vitest";
import type { BackendInfo, SessionNode, Snapshot, Workspace } from "../types";
import { selectUnclassified, selectWorkspaceGroups, useCue } from "../store/useCue";

const backend: BackendInfo = {
  id: "iterm2",
  capabilities: {
    split: true,
    newTab: true,
    typeTextNoEnter: true,
    rename: true,
    closePane: true,
    preciseTabs: false,
  },
  installed: { iterm2: true, terminalApp: true },
  fallback: null,
};

function ws(id: string): Workspace {
  return { id, path: `/proj/${id}`, name: id, collapsed: false };
}

function node(id: string, workspaceId: string | null, windowId: number, tabIndex: number, alive = true): SessionNode {
  return {
    id,
    kind: { type: "zsh" },
    status: alive ? "idle" : "exited",
    workspaceId,
    cwd: workspaceId ? `/proj/${workspaceId}` : "/Users/me",
    tty: `tty-${id}`,
    label: null,
    defaultName: "zsh",
    locationHint: `視窗 ${windowId} · 分頁 ${tabIndex}`,
    windowId,
    tabIndex,
    alive,
  };
}

function snapshot(partial: Partial<Snapshot> = {}): Snapshot {
  return { workspaces: [], sessions: [], unclassified: [], backend, ...partial };
}

describe("applySnapshot", () => {
  beforeEach(() => {
    useCue.setState({ workspaces: [], sessions: {}, unclassified: [], backend: null, lastError: null });
  });

  it("以 id 為 key 存放節點並寫入後端資訊", () => {
    useCue.getState().applySnapshot(snapshot({ sessions: [node("a", null, 1, 1), node("b", null, 1, 2)] }));
    const s = useCue.getState();
    expect(Object.keys(s.sessions).sort()).toEqual(["a", "b"]);
    expect(s.sessions.b.tabIndex).toBe(2);
    expect(s.backend).toEqual(backend);
  });

  it("依工作區分組，工作區維持設定檔順序，空工作區也列出", () => {
    useCue.getState().applySnapshot(
      snapshot({
        workspaces: [ws("b"), ws("a"), ws("empty")],
        sessions: [node("1", "a", 1, 1), node("2", "b", 2, 1), node("3", "a", 3, 1)],
      }),
    );
    const groups = selectWorkspaceGroups(useCue.getState());
    expect(groups.map((g) => g.workspace.id)).toEqual(["b", "a", "empty"]);
    expect(groups.map((g) => g.sessions.map((n) => n.id))).toEqual([["2"], ["1", "3"], []]);
  });

  it("工作區內依視窗、再依分頁排序，已結束的節點保留", () => {
    useCue.getState().applySnapshot(
      snapshot({
        workspaces: [ws("a")],
        sessions: [
          node("w9t1", "a", 9, 1),
          node("w2t3", "a", 2, 3, false),
          node("w2t1", "a", 2, 1),
          node("w5t2", "a", 5, 2),
          node("w2t2", "a", 2, 2),
        ],
      }),
    );
    const [group] = selectWorkspaceGroups(useCue.getState());
    expect(group.sessions.map((n) => n.id)).toEqual(["w2t1", "w2t2", "w2t3", "w5t2", "w9t1"]);
    expect(group.sessions[2].alive).toBe(false);
  });

  it("未歸類只取快照列出的 id，不混入工作區分組", () => {
    useCue.getState().applySnapshot(
      snapshot({
        workspaces: [ws("a")],
        sessions: [
          node("in-a", "a", 1, 1),
          node("u2", null, 4, 1),
          node("u1", null, 3, 1),
          node("ignored", null, 5, 1),
          node("gone", null, 6, 1, false),
        ],
        unclassified: ["u2", "u1", "missing"],
      }),
    );
    const state = useCue.getState();
    expect(selectUnclassified(state).map((n) => n.id)).toEqual(["u1", "u2"]);
    expect(selectWorkspaceGroups(state)[0].sessions.map((n) => n.id)).toEqual(["in-a"]);
  });

  it("新快照完整取代舊狀態", () => {
    const { applySnapshot } = useCue.getState();
    applySnapshot(snapshot({ workspaces: [ws("a")], sessions: [node("old", "a", 1, 1)], unclassified: [] }));
    applySnapshot(snapshot({ workspaces: [ws("a")], sessions: [node("new", null, 1, 1)], unclassified: ["new"] }));
    const state = useCue.getState();
    expect(Object.keys(state.sessions)).toEqual(["new"]);
    expect(selectWorkspaceGroups(state)[0].sessions).toEqual([]);
    expect(selectUnclassified(state).map((n) => n.id)).toEqual(["new"]);
  });

  it("輸入未變時 selector 回傳同一個物件", () => {
    useCue.getState().applySnapshot(snapshot({ workspaces: [ws("a")], sessions: [node("1", "a", 1, 1)] }));
    const state = useCue.getState();
    expect(selectWorkspaceGroups(state)).toBe(selectWorkspaceGroups(state));
    expect(selectUnclassified(state)).toBe(selectUnclassified(state));
  });
});
