import { ask } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { useEffect, useId, useState } from "react";
import { createSession, removeWorkspace, setWorkspaceCollapsed, toCueError } from "../../ipc/api";
import { useCue } from "../../store/useCue";
import type { CueError, SessionNode as Node, Workspace } from "../../types";
import { ContextMenu, useContextMenu, type MenuEntry } from "../ContextMenu";
import { ChevronDown, ChevronRight } from "../icons/Icons";
import { SessionNode } from "./SessionNode";

interface Props {
  workspace: Workspace;
  sessions: Node[];
}

/** 工作區節點：摺疊箭頭、名稱、子節點數；收合狀態寫回設定檔，右鍵開啟操作選單。 */
export function WorkspaceNode({ workspace, sessions }: Props) {
  const setLastError = useCue((s) => s.setLastError);
  const openCreateModal = useCue((s) => s.openCreateModal);
  const openZshLabel = useCue((s) => (s.backend?.id === "terminal" ? "在終端機開一個 zsh" : "在 iTerm2 開一個 zsh"));
  // 先在畫面上反映，快照回來後以 Rust 的值為準。
  const [collapsed, setCollapsed] = useState(workspace.collapsed);
  useEffect(() => setCollapsed(workspace.collapsed), [workspace.collapsed]);
  const listId = useId();
  const menu = useContextMenu();

  const toggle = () => {
    const next = !collapsed;
    setCollapsed(next);
    setWorkspaceCollapsed(workspace.id, next).catch((e: CueError) => {
      setCollapsed(!next);
      setLastError(e);
    });
  };

  const report = (e: unknown) => setLastError(toCueError(e));

  const confirmRemove = async () => {
    const ok = await ask(
      `「${workspace.name}」會從 Cue 的清單移除，不會刪除資料夾或其中任何檔案。\n\n其中的終端機會依目前所在的資料夾重新歸類。`,
      { title: "移除工作區", kind: "warning", okLabel: "移除", cancelLabel: "取消" },
    );
    if (ok) await removeWorkspace(workspace.id);
  };

  const entries: MenuEntry[] = [
    { type: "item", label: "新增工作階段…", onSelect: () => openCreateModal(workspace.id) },
    { type: "item", label: "在 Finder 開啟", onSelect: () => revealItemInDir(workspace.path).catch(report) },
    {
      type: "item",
      label: openZshLabel,
      // 固定開新視窗：分頁可能落在其他工作區的視窗裡。
      onSelect: () => createSession(workspace.id, "zsh", null, "newWindow").catch(report),
    },
    { type: "separator" },
    {
      type: "item",
      label: "移除工作區…",
      note: "不會刪除任何檔案",
      onSelect: () => confirmRemove().catch(report),
    },
  ];

  const Chevron = collapsed ? ChevronRight : ChevronDown;

  return (
    <li className="pt-3 first:pt-1">
      <button
        type="button"
        onClick={toggle}
        {...menu.triggerProps}
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
      {menu.anchor && (
        <ContextMenu anchor={menu.anchor} title={workspace.name} entries={entries} onClose={menu.close} />
      )}
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
