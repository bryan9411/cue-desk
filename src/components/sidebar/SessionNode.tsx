import { useEffect, useRef, useState, type ReactNode } from "react"
import { predictDispatch, sessionTarget } from "../../hooks/useDragDrop"
import { usePendingTask } from "../../hooks/usePendingTask"
import {
  closeSession,
  focusSession,
  removeNode,
  renameSession,
  reopenSession,
  splitZsh,
  type SplitDirection,
} from "../../ipc/api"
import { selectCapabilities, useCue } from "../../store/useCue"
import type { CueError, SessionKind, SessionNode as Node } from "../../types"
import { ContextMenu, useContextMenu, type MenuEntry } from "../ContextMenu"
import { StatusLight } from "../StatusLight"
import { DropHint } from "./DropHint"
import { RenameField } from "./RenameField"

type Message = { tone: "error" | "info"; text: string }

const MESSAGE_STYLE: Record<Message["tone"], { role: "alert" | "status"; color: string }> = {
  error: { role: "alert", color: "text-status-waiting" },
  info: { role: "status", color: "text-muted" },
}

/** 節點下方訊息與拖放提示的縮排，對齊名稱而不是燈號。 */
const FEEDBACK_INSET = "ml-[30px] mr-2 mb-1"

const ROW_CLASS = "group grid w-full grid-cols-[10px_1fr] items-center gap-x-2.5 rounded-sm px-2 py-1.5 text-left"

const DROP_ACCEPT_TEXT = "放開後填入路徑，不會送出 Enter"

function kindLabel(kind: SessionKind): string {
  return kind.type === "other" ? kind.name : kind.type
}

/** 拖曳懸停時的虛線外框；不能放下時改用警示色。 */
function dropRingClass(dragOver: boolean, rejected: boolean): string {
  if (!dragOver) return ""
  const color = rejected ? "outline-status-waiting" : "outline-accent"
  return `bg-surface outline-1 -outline-offset-1 outline-dashed ${color}`
}

function NodeMessage({ message }: { message: Message }) {
  const { role, color } = MESSAGE_STYLE[message.tone]
  return (
    <p role={role} className={`${FEEDBACK_INSET} text-[12px] leading-4 ${color}`}>
      {message.text}
    </p>
  )
}

interface Props {
  node: Node
  /** 取代位置提示的第二行文字（未歸類區用來顯示 cwd）。 */
  detail?: string
  /** 是否提供右鍵選單；未歸類區只用列下方的按鈕操作。 */
  withMenu?: boolean
  /** 顯示在節點下方的操作列。 */
  children?: ReactNode
}

/**
 * 工作階段節點：點擊聚焦終端機，右鍵開啟操作選單；結果訊息顯示在節點下方。
 * 也是拖檔派工的落點：懸停時預判並顯示能否填入路徑，放下後的結果同樣顯示在下方。
 */
export function SessionNode({ node, detail, withMenu = true, children }: Props) {
  const caps = useCue(selectCapabilities)
  const [message, setMessage] = useState<Message | null>(null)
  const { pending, run } = usePendingTask()
  const [renaming, setRenaming] = useState(false)
  // 重新命名送出後先顯示新名稱，直到快照帶回新的標籤。
  const [optimisticLabel, setOptimisticLabel] = useState<string | null | undefined>(undefined)
  const menu = useContextMenu()
  const rowRef = useRef<HTMLButtonElement>(null)
  // 以 Enter／Esc 結束重新命名時把焦點還給節點；點別處結束時不搶焦點。
  const refocusRow = useRef(false)
  const dropTarget = sessionTarget(node.id)
  const dragOver = useCue((s) => s.dropTarget === dropTarget)
  const dropFeedback = useCue((s) => (s.dropFeedback?.target === dropTarget ? s.dropFeedback : null))

  useEffect(() => setOptimisticLabel(undefined), [node.label])
  useEffect(() => {
    if (renaming || !refocusRow.current) return
    refocusRow.current = false
    rowRef.current?.focus()
  }, [renaming])

  // 放下的結果轉成節點自己的訊息，之後與其他操作的訊息一樣在下一次操作時清除。
  useEffect(() => {
    if (!dropFeedback) return
    setMessage({ tone: dropFeedback.tone, text: dropFeedback.text })
    useCue.getState().setDropFeedback(null)
  }, [dropFeedback])

  function showError(error: CueError) {
    setMessage({ tone: "error", text: error.message })
  }

  /** 執行一個操作；`action` 回傳字串時當作提示顯示在節點下方。 */
  function runAction(action: () => Promise<string | null>) {
    run(async () => {
      setMessage(null)
      const notice = await action()
      setMessage(notice ? { tone: "info", text: notice } : null)
    }, showError)
  }

  /** 執行不需要提示的操作；失敗時一樣顯示錯誤。 */
  function runWithoutNotice(action: () => Promise<unknown>) {
    runAction(async () => {
      await action()
      return null
    })
  }

  function focus() {
    runWithoutNotice(() => focusSession(node.id))
  }

  function reopen() {
    runWithoutNotice(() => reopenSession(node.id))
  }

  function remove() {
    runWithoutNotice(() => removeNode(node.id))
  }

  function close() {
    runWithoutNotice(() => closeSession(node.id))
  }

  function split(direction: SplitDirection) {
    runAction(() => splitZsh(node.id, direction))
  }

  function startRename() {
    setRenaming(true)
  }

  function endRename(byKey: boolean) {
    refocusRow.current = byKey
    setRenaming(false)
  }

  async function renameOrRevert(value: string) {
    try {
      await renameSession(node.id, value)
    } catch (e) {
      setOptimisticLabel(undefined)
      throw e
    }
  }

  function commitRename(value: string, byKey: boolean) {
    endRename(byKey)
    setOptimisticLabel(value === "" ? null : value)
    runWithoutNotice(() => renameOrRevert(value))
  }

  const exited = node.status === "exited"
  const label = optimisticLabel === undefined ? node.label : optimisticLabel
  const name = label ?? node.defaultName
  const secondLine = detail ?? node.locationHint
  const dropRejection = dragOver ? predictDispatch(caps, node) : null
  const menuTriggerProps = withMenu ? menu.triggerProps : {}

  const splitNote = caps?.split ? undefined : "終端機後備模式改開新視窗"
  const renameItem: MenuEntry = {
    type: "item",
    label: "重新命名",
    onSelect: startRename,
    note: caps?.rename ? undefined : "名稱只顯示在 Cue，終端機標題不變",
  }

  const exitedEntries: MenuEntry[] = [
    { type: "item", label: "重新開啟", onSelect: reopen },
    renameItem,
    { type: "separator" },
    { type: "item", label: "移除", note: "從清單移除，並忘掉它的名稱", onSelect: remove },
  ]

  const liveEntries: MenuEntry[] = [
    { type: "item", label: "聚焦", onSelect: focus },
    renameItem,
    { type: "separator" },
    { type: "item", label: "在右側補開同工作區 Zsh", note: splitNote, onSelect: () => split("right") },
    { type: "item", label: "在下方補開同工作區 Zsh", note: splitNote, onSelect: () => split("below") },
    { type: "separator" },
    {
      type: "item",
      label: "關閉",
      disabled: !caps?.closePane,
      note: caps?.closePane ? undefined : "目前的終端機不支援從 Cue 關閉",
      onSelect: close,
    },
  ]

  const entries = exited ? exitedEntries : liveEntries

  function renderFeedback() {
    if (dragOver) {
      return (
        <DropHint
          tone={dropRejection ? "reject" : "accept"}
          text={dropRejection ?? DROP_ACCEPT_TEXT}
          className={FEEDBACK_INSET}
        />
      )
    }
    if (message) return <NodeMessage message={message} />
    return null
  }

  const kindTag = (
    <span className="shrink-0 rounded-sm border border-border-subtle px-1 font-mono text-[11px] leading-4 text-muted">
      {kindLabel(node.kind)}
    </span>
  )
  const secondLineText = (
    <span className="col-start-2 truncate font-mono text-[11px] leading-4 text-muted" title={secondLine}>
      {secondLine}
    </span>
  )
  const dimmed = exited && !renaming

  return (
    <li data-drop-target={dropTarget} className={dimmed ? "opacity-55" : undefined}>
      {renaming ? (
        <div className={`${ROW_CLASS} bg-surface`}>
          <StatusLight status={node.status} />
          <span className="flex min-w-0 items-center gap-2">
            {kindTag}
            <RenameField
              initial={node.label ?? ""}
              placeholder={node.defaultName}
              ariaLabel={`重新命名 ${name}`}
              onCommit={commitRename}
              onCancel={endRename}
            />
          </span>
          {secondLineText}
        </div>
      ) : (
        <button
          ref={rowRef}
          type="button"
          onClick={focus}
          {...menuTriggerProps}
          aria-busy={pending || undefined}
          className={`${ROW_CLASS} hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring active:bg-selected ${dropRingClass(dragOver, dropRejection !== null)}`}
        >
          <StatusLight status={node.status} />
          <span className="flex min-w-0 items-baseline gap-2">
            {kindTag}
            {label !== null && <span className="truncate text-[13px] text-primary">{label}</span>}
          </span>
          {secondLineText}
        </button>
      )}
      {children}
      {renderFeedback()}
      {withMenu && menu.anchor && (
        <ContextMenu anchor={menu.anchor} title={name} entries={entries} onClose={menu.close} />
      )}
    </li>
  )
}
