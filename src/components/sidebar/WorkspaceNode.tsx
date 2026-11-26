import { ask } from "@tauri-apps/plugin-dialog"
import { revealItemInDir } from "@tauri-apps/plugin-opener"
import { useEffect, useId, useState } from "react"
import { createSession, removeWorkspace, setWorkspaceCollapsed } from "../../ipc/api"
import { activeAiCount } from "../../lib/workspaceStats"
import { useCue, type CueState } from "../../store/useCue"
import type { SessionNode as Node, Workspace } from "../../types"
import { ContextMenu, useContextMenu, type MenuEntry } from "../ContextMenu"
import { ChevronDown, ChevronRight } from "../icons/Icons"
import { WarningBadge } from "../WarningBadge"
import { BranchBadge } from "./BranchBadge"
import { SessionNode } from "./SessionNode"

/** 同一工作區有這麼多個 AI 在執行時顯示警示。 */
const AI_WARNING_THRESHOLD = 2

/** 選單「開一個 zsh」的文字，依目前使用的終端機。 */
function selectOpenZshLabel(s: CueState): string {
  if (s.backend?.id === "terminal") return "在終端機開一個 zsh"
  return "在 iTerm2 開一個 zsh"
}

interface Props {
  workspace: Workspace
  sessions: Node[]
}

/** 工作區節點：摺疊箭頭、名稱、多 AI 警示、子節點數，第二行為分支徽章；收合狀態寫回設定檔，右鍵開啟操作選單。 */
export function WorkspaceNode({ workspace, sessions }: Props) {
  const reportError = useCue((s) => s.reportError)
  const openCreateModal = useCue((s) => s.openCreateModal)
  const openZshLabel = useCue(selectOpenZshLabel)
  // 先在畫面上反映，快照回來後以 Rust 的值為準。
  const [collapsed, setCollapsed] = useState(workspace.collapsed)
  useEffect(() => setCollapsed(workspace.collapsed), [workspace.collapsed])
  const listId = useId()
  const menu = useContextMenu()

  function toggle() {
    const next = !collapsed
    setCollapsed(next)
    setWorkspaceCollapsed(workspace.id, next).catch((e: unknown) => {
      setCollapsed(!next)
      reportError(e)
    })
  }

  function openCreate() {
    openCreateModal(workspace.id)
  }

  function revealInFinder() {
    revealItemInDir(workspace.path).catch(reportError)
  }

  function openZsh() {
    // 固定開新視窗：分頁可能落在其他工作區的視窗裡。
    createSession(workspace.id, "zsh", null, "newWindow").catch(reportError)
  }

  async function confirmAndRemove() {
    const ok = await ask(
      `「${workspace.name}」會從 Cue 的清單移除，不會刪除資料夾或其中任何檔案。\n\n其中的終端機會依目前所在的資料夾重新歸類。`,
      { title: "移除工作區", kind: "warning", okLabel: "移除", cancelLabel: "取消" },
    )
    if (ok) await removeWorkspace(workspace.id)
  }

  function remove() {
    confirmAndRemove().catch(reportError)
  }

  const entries: MenuEntry[] = [
    { type: "item", label: "新增工作階段…", onSelect: openCreate },
    { type: "item", label: "在 Finder 開啟", onSelect: revealInFinder },
    { type: "item", label: openZshLabel, onSelect: openZsh },
    { type: "separator" },
    { type: "item", label: "移除工作區…", note: "不會刪除任何檔案", onSelect: remove },
  ]

  const Chevron = collapsed ? ChevronRight : ChevronDown
  const aiCount = activeAiCount(workspace.id, sessions)
  const showAiWarning = aiCount >= AI_WARNING_THRESHOLD

  return (
    <li className="pt-3 first:pt-1">
      <button
        type="button"
        onClick={toggle}
        {...menu.triggerProps}
        aria-expanded={!collapsed}
        aria-controls={listId}
        title={workspace.path}
        className="grid w-full grid-cols-[14px_minmax(0,1fr)_auto] items-center gap-x-1.5 gap-y-0.5 rounded-sm px-1 py-1 text-left hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        <Chevron size={14} className="text-muted" />
        <span className="truncate font-heading text-[15px] leading-5 [font-weight:var(--heading-weight)]">
          {workspace.name}
        </span>
        <span className="flex items-center">
          {showAiWarning && <WarningBadge count={aiCount} />}
          <span className="pr-1 font-mono text-[11px] text-muted" aria-label={`${sessions.length} 個工作階段`}>
            {sessions.length}
          </span>
        </span>
        <span className="col-span-2 col-start-2 flex min-w-0 justify-start">
          <BranchBadge branch={workspace.branch} detached={workspace.detached} />
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
  )
}
