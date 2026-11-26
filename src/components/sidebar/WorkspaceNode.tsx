import { useEffect, useId, useState } from "react";
import { setWorkspaceCollapsed } from "../../ipc/api";
import { useCue } from "../../store/useCue";
import type { CueError, SessionNode as Node, Workspace } from "../../types";
import { ChevronDown, ChevronRight } from "../icons/Icons";
import { SessionNode } from "./SessionNode";

interface Props {
  workspace: Workspace;
  sessions: Node[];
}

/** 工作區節點：摺疊箭頭、名稱、子節點數；收合狀態寫回設定檔。 */
export function WorkspaceNode({ workspace, sessions }: Props) {
  const setLastError = useCue((s) => s.setLastError);
  // 先在畫面上反映，快照回來後以 Rust 的值為準。
  const [collapsed, setCollapsed] = useState(workspace.collapsed);
  useEffect(() => setCollapsed(workspace.collapsed), [workspace.collapsed]);
  const listId = useId();

  const toggle = () => {
    const next = !collapsed;
    setCollapsed(next);
    setWorkspaceCollapsed(workspace.id, next).catch((e: CueError) => {
      setCollapsed(!next);
      setLastError(e);
    });
  };

  const Chevron = collapsed ? ChevronRight : ChevronDown;

  return (
    <li className="pt-3 first:pt-1">
      <button
        type="button"
        onClick={toggle}
        aria-expanded={!collapsed}
        aria-controls={listId}
        title={workspace.path}
        className="flex w-full items-center gap-1.5 rounded-sm px-1 py-1 text-left hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <Chevron size={14} className="shrink-0 text-muted" />
        <span className="min-w-0 flex-1 truncate font-heading text-[15px] leading-5 [font-weight:var(--heading-weight)]">
          {workspace.name}
        </span>
        <span className="shrink-0 pr-1 font-mono text-[11px] text-muted" aria-label={`${sessions.length} 個工作階段`}>
          {sessions.length}
        </span>
      </button>
      {!collapsed && (
        <ul id={listId} className="ml-[11px] mt-0.5 border-l border-border-subtle pl-1">
          {sessions.length === 0 ? (
            <li className="px-2 py-1.5 text-[12px] leading-4 text-muted">在這個資料夾開啟終端機，就會出現在這裡。</li>
          ) : (
            sessions.map((node) => <SessionNode key={node.id} node={node} />)
          )}
        </ul>
      )}
    </li>
  );
}
