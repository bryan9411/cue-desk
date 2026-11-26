import { create } from "zustand";
import type { OpenMode } from "../ipc/api";
import type { BackendInfo, BranchChange, Capabilities, CueError, SessionNode, Snapshot, Workspace } from "../types";

// 所有 session 狀態更新只經由 applySnapshot()，不要在別處直接改 sessions。
export interface CueState {
  /** 依設定檔順序的工作區。 */
  workspaces: Workspace[];
  /** 所有節點，以 id 為 key。 */
  sessions: Record<string, SessionNode>;
  /** 「未歸類」區要顯示的節點 id。 */
  unclassified: string[];
  /** 尚未取得快照前為 `null`。 */
  backend: BackendInfo | null;
  /** 建立工作階段 Modal 的預設開啟方式；沒有設定也沒用過時為 `null`。 */
  preferredOpenMode: OpenMode | null;
  /** 最近一次全域錯誤（例如無法取得快照）；個別節點的操作錯誤由元件自行顯示。 */
  lastError: CueError | null;
  /** 建立工作階段 Modal 要預選的工作區；`null` 為未開啟。 */
  createModal: { workspaceId: string } | null;

  /** 以 Rust 送來的完整快照取代目前狀態。 */
  applySnapshot: (snapshot: Snapshot) => void;
  /** 套用單一工作區的分支變動；工作區不存在或值相同時不改變狀態。 */
  applyBranchUpdate: (change: BranchChange) => void;
  setLastError: (error: CueError | null) => void;
  openCreateModal: (workspaceId: string) => void;
  closeCreateModal: () => void;
}

export const useCue = create<CueState>()((set) => ({
  workspaces: [],
  sessions: {},
  unclassified: [],
  backend: null,
  preferredOpenMode: null,
  lastError: null,
  createModal: null,

  applySnapshot: (snapshot) => {
    const sessions: Record<string, SessionNode> = {};
    for (const node of snapshot.sessions) sessions[node.id] = node;
    set({
      workspaces: snapshot.workspaces,
      sessions,
      unclassified: snapshot.unclassified,
      backend: snapshot.backend,
      preferredOpenMode: snapshot.preferredOpenMode,
    });
  },
  applyBranchUpdate: ({ workspaceId, branch, detached }) =>
    set((state) => {
      const index = state.workspaces.findIndex((w) => w.id === workspaceId);
      const current = state.workspaces[index];
      if (!current || (current.branch === branch && current.detached === detached)) return state;
      const workspaces = state.workspaces.slice();
      workspaces[index] = { ...current, branch, detached };
      return { workspaces };
    }),
  setLastError: (lastError) => set({ lastError }),
  openCreateModal: (workspaceId) => set({ createModal: { workspaceId } }),
  closeCreateModal: () => set({ createModal: null }),
}));

// ---- 衍生資料 ----

export interface WorkspaceGroup {
  workspace: Workspace;
  sessions: SessionNode[];
}

/**
 * 工作區內的排序：依視窗、再依分頁。
 * 視窗以穩定的 `windowId` 比較（視窗序號會隨前後順序改變，用它排序會讓節點在聚焦時跳動）。
 */
export function compareSessions(a: SessionNode, b: SessionNode): number {
  return a.windowId - b.windowId || a.tabIndex - b.tabIndex || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0);
}

/** 依工作區分組（工作區維持設定檔順序，含已結束的節點）。 */
export function groupByWorkspace(
  workspaces: Workspace[],
  sessions: Record<string, SessionNode>,
): WorkspaceGroup[] {
  const byWorkspace = new Map<string, SessionNode[]>(workspaces.map((w) => [w.id, []]));
  for (const node of Object.values(sessions)) {
    if (node.workspaceId !== null) byWorkspace.get(node.workspaceId)?.push(node);
  }
  return workspaces.map((workspace) => ({
    workspace,
    sessions: (byWorkspace.get(workspace.id) ?? []).sort(compareSessions),
  }));
}

/** 「未歸類」區的節點（只取快照列出的 id）。 */
export function pickUnclassified(ids: string[], sessions: Record<string, SessionNode>): SessionNode[] {
  return ids
    .map((id) => sessions[id])
    .filter((node): node is SessionNode => node !== undefined)
    .sort(compareSessions);
}

/** 依參考相等快取上一次的結果，讓 selector 在輸入未變時回傳同一個物件，避免重複渲染。 */
function memoizeLast<A extends unknown[], R>(fn: (...args: A) => R): (...args: A) => R {
  let lastArgs: A | undefined;
  let lastResult: R;
  return (...args: A) => {
    if (lastArgs && args.length === lastArgs.length && args.every((arg, i) => arg === lastArgs![i])) {
      return lastResult;
    }
    lastArgs = args;
    lastResult = fn(...args);
    return lastResult;
  };
}

const groupMemo = memoizeLast(groupByWorkspace);
const unclassifiedMemo = memoizeLast(pickUnclassified);

/** 依工作區分組並排序的節點。 */
export const selectWorkspaceGroups = (s: CueState): WorkspaceGroup[] => groupMemo(s.workspaces, s.sessions);

/** 目前後端支援的操作；尚未取得快照前為 `null`。 */
export const selectCapabilities = (s: CueState): Capabilities | null => s.backend?.capabilities ?? null;

/** 「未歸類」區的節點。 */
export const selectUnclassified = (s: CueState): SessionNode[] => unclassifiedMemo(s.unclassified, s.sessions);
