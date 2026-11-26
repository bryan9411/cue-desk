// 建立工作階段：原生 <dialog> 以 showModal() 開啟，背景不可操作、Esc 關閉。
import { useEffect, useId, useRef, useState, type ComponentType, type FormEvent, type KeyboardEvent } from "react"
import { usePendingTask } from "../hooks/usePendingTask"
import { createSession, type CreatableKind, type OpenMode } from "../ipc/api"
import { isImeComposing } from "../lib/keyboard"
import { LABEL_MAX_CHARS } from "../lib/label"
import { activeAiCount } from "../lib/workspaceStats"
import { selectCapabilities, useCue } from "../store/useCue"
import type { Capabilities, CueError, SessionNode, Workspace } from "../types"
import { LayoutSplitColumns, LayoutSplitRows, LayoutTab, LayoutWindow, Warning, type IconProps } from "./icons/Icons"

const KINDS: CreatableKind[] = ["claude", "gemini", "zsh"]

const MODES: { mode: OpenMode; label: string; Icon: ComponentType<IconProps> }[] = [
  { mode: "newWindow", label: "新視窗", Icon: LayoutWindow },
  { mode: "newTab", label: "新分頁", Icon: LayoutTab },
  { mode: "aiTopZshBottom", label: "AI 上・Zsh 下", Icon: LayoutSplitRows },
  { mode: "aiLeftZshRight", label: "AI 左・Zsh 右", Icon: LayoutSplitColumns },
]

const NO_CAPS: Capabilities = {
  split: false,
  newTab: false,
  typeTextNoEnter: false,
  rename: false,
  closePane: false,
  preciseTabs: false,
}

const TERMINAL_APP_NOTE = "終端機（Terminal.app）不支援分割，也無法不透過模擬按鍵開新分頁，只能開新視窗。"
const NO_SPLIT_NOTE = "目前的終端機不支援分割。"
const ZSH_SPLIT_NOTE = "分割是讓 AI 與 zsh 並排，類型選 zsh 時不適用。"

const GUARD_TEXT =
  "這個資料夾已有 AI 正在工作。同一資料夾只有單一分支，多個 AI 同時修改檔案可能互相覆蓋，建議先分工（例如各自負責不同檔案）。"

/** 不能用這個開啟方式的原因；可以用時為 `null`。 */
function modeBlocker(mode: OpenMode, kind: CreatableKind, caps: Capabilities): string | null {
  if (mode === "newWindow") return null
  if (!caps.newTab && !caps.split) return TERMINAL_APP_NOTE
  if (mode === "newTab") return caps.newTab ? null : TERMINAL_APP_NOTE
  if (!caps.split) return NO_SPLIT_NOTE
  return kind === "zsh" ? ZSH_SPLIT_NOTE : null
}

function tileClass(checked: boolean, disabled: boolean): string {
  const base =
    "flex items-center gap-2 rounded-sm border px-2 py-1.5 text-[13px] leading-5 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-focus-ring"
  if (disabled) return `${base} cursor-default border-border-subtle text-muted opacity-60`
  if (checked) return `${base} cursor-pointer border-border-strong bg-selected`
  return `${base} cursor-pointer border-border-subtle hover:bg-surface`
}

interface ModeOption {
  mode: OpenMode
  label: string
  Icon: ComponentType<IconProps>
  /** 不能選的原因；可以選時為 `null`。 */
  blocker: string | null
}

function modeOptions(kind: CreatableKind, caps: Capabilities): ModeOption[] {
  return MODES.map((option) => ({ ...option, blocker: modeBlocker(option.mode, kind, caps) }))
}

/** 各開啟方式不能用的原因，去掉重複後列在選項下方。 */
function uniqueBlockers(options: ModeOption[]): string[] {
  const blockers = options.map((option) => option.blocker).filter((blocker) => blocker !== null)
  return [...new Set(blockers)]
}

/** 要建立的是 AI，而且這個工作區已經有 AI 在執行時，提醒可能互相覆蓋檔案。 */
function shouldWarnConcurrentAi(
  workspace: Workspace | undefined,
  kind: CreatableKind,
  sessions: Record<string, SessionNode>,
): boolean {
  if (!workspace || kind === "zsh") return false
  return activeAiCount(workspace.id, Object.values(sessions)) >= 1
}

function labelHint(caps: Capabilities): string {
  const where = caps.rename ? "也會設為終端機標題。" : "只顯示在 Cue，終端機標題不變。"
  return `${where}留空時以類型命名。`
}

/** 輸入法選字中的 Enter 用來確認候選字，不能讓它觸發表單送出。 */
function preventImeEnterSubmit(e: KeyboardEvent<HTMLInputElement>) {
  if (e.key === "Enter" && isImeComposing(e)) e.preventDefault()
}

const fieldLabelClass = "mb-1.5 block text-[12px] leading-4 text-muted"

const fieldClass =
  "w-full rounded-sm border border-border-strong bg-base px-2 py-1 text-[13px] leading-5 text-primary focus-visible:outline-2 focus-visible:outline-focus-ring"

export function CreateSessionModal({ workspaceId: initialWorkspaceId }: { workspaceId: string }) {
  const workspaces = useCue((s) => s.workspaces)
  const sessions = useCue((s) => s.sessions)
  const caps = useCue(selectCapabilities) ?? NO_CAPS
  const closeCreateModal = useCue((s) => s.closeCreateModal)

  const [workspaceId, setWorkspaceId] = useState(initialWorkspaceId)
  const [kind, setKind] = useState<CreatableKind>("claude")
  const [label, setLabel] = useState("")
  // 使用者選的方式；目前類型或後端不能用時改以新視窗開啟，換回可用的類型後恢復原本的選擇。
  const [mode, setMode] = useState<OpenMode>(() => useCue.getState().preferredOpenMode ?? "newWindow")
  const { pending, run } = usePendingTask()
  const [error, setError] = useState<string | null>(null)

  const dialogRef = useRef<HTMLDialogElement>(null)
  const firstFieldRef = useRef<HTMLSelectElement>(null)
  const titleId = useId()
  const modeNoteId = useId()

  useEffect(() => {
    const dialog = dialogRef.current
    if (!dialog) return
    if (!dialog.open) dialog.showModal()
    firstFieldRef.current?.focus()
  }, [])

  // 開啟期間工作區被移除時，改選第一個；全部移除就關閉。
  const workspace = workspaces.find((w) => w.id === workspaceId) ?? workspaces[0]
  useEffect(() => {
    if (!workspace) dialogRef.current?.close()
  }, [workspace])

  const options = modeOptions(kind, caps)
  const effectiveMode = modeBlocker(mode, kind, caps) ? "newWindow" : mode
  const modeNotes = uniqueBlockers(options)
  const hasModeNotes = modeNotes.length > 0
  const showGuard = shouldWarnConcurrentAi(workspace, kind, sessions)

  function close() {
    dialogRef.current?.close()
  }

  function showError(err: CueError) {
    setError(err.message)
  }

  async function create(target: Workspace) {
    setError(null)
    await createSession(target.id, kind, label.trim() || null, effectiveMode)
    close()
  }

  function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!workspace) return
    run(() => create(workspace), showError)
  }

  return (
    <dialog
      ref={dialogRef}
      aria-labelledby={titleId}
      onClose={closeCreateModal}
      className="mx-auto mb-auto mt-3 max-h-[calc(100%-24px)] w-[calc(100%-24px)] max-w-90 overflow-y-auto rounded-lg border border-border-strong bg-base p-0 text-primary shadow-panel transition-[translate,opacity] duration-150 backdrop:bg-base/70 starting:-translate-y-2 starting:opacity-0 motion-reduce:transition-none"
    >
      <form onSubmit={handleSubmit} className="flex flex-col gap-4 px-4 pb-4 pt-3">
        <h2
          id={titleId}
          className="pb-2 font-heading text-[17px] leading-6 [border-bottom:var(--header-rule)] [font-weight:var(--heading-weight)]"
        >
          新增工作階段
        </h2>

        <div>
          <label htmlFor={`${titleId}-workspace`} className={fieldLabelClass}>
            工作區
          </label>
          <select
            id={`${titleId}-workspace`}
            ref={firstFieldRef}
            value={workspace?.id ?? ""}
            onChange={(e) => setWorkspaceId(e.target.value)}
            className={fieldClass}
          >
            {workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
          </select>
          {workspace && (
            <p className="mt-1 truncate text-[11px] leading-4 text-muted" title={workspace.path}>
              {workspace.path}
            </p>
          )}
        </div>

        <fieldset>
          <legend className={fieldLabelClass}>類型</legend>
          <div className="grid grid-cols-3 gap-1.5">
            {KINDS.map((k) => (
              <label key={k} className={`${tileClass(kind === k, false)} justify-center font-mono`}>
                <input
                  type="radio"
                  name="kind"
                  value={k}
                  checked={kind === k}
                  onChange={() => setKind(k)}
                  className="sr-only"
                />
                {k}
              </label>
            ))}
          </div>
        </fieldset>

        {showGuard && (
          <p className="-mt-1 flex gap-2 rounded-sm bg-surface px-2.5 py-2 text-[12px] leading-[18px]">
            <Warning size={14} className="mt-0.5 shrink-0 text-status-waiting" />
            <span>{GUARD_TEXT}</span>
          </p>
        )}

        <div>
          <label htmlFor={`${titleId}-label`} className={fieldLabelClass}>
            名稱
          </label>
          <input
            id={`${titleId}-label`}
            type="text"
            value={label}
            maxLength={LABEL_MAX_CHARS}
            placeholder="例如：權限架構"
            spellCheck={false}
            autoComplete="off"
            onChange={(e) => setLabel(e.target.value)}
            onKeyDown={preventImeEnterSubmit}
            className={`${fieldClass} outline-none placeholder:text-muted`}
          />
          <p className="mt-1 text-[11px] leading-4 text-muted">{labelHint(caps)}</p>
        </div>

        <fieldset aria-describedby={hasModeNotes ? modeNoteId : undefined}>
          <legend className={fieldLabelClass}>開啟方式</legend>
          <div className="grid grid-cols-2 gap-1.5">
            {options.map(({ mode: m, label: text, Icon, blocker }) => {
              const disabled = blocker !== null
              return (
                <label key={m} className={tileClass(effectiveMode === m, disabled)}>
                  <input
                    type="radio"
                    name="mode"
                    value={m}
                    checked={effectiveMode === m}
                    disabled={disabled}
                    onChange={() => setMode(m)}
                    className="sr-only"
                  />
                  <Icon size={18} className="shrink-0" />
                  <span className="min-w-0">{text}</span>
                </label>
              )
            })}
          </div>
          {hasModeNotes && (
            <div id={modeNoteId} className="mt-1.5 text-[11px] leading-4 text-muted">
              {modeNotes.map((note) => (
                <p key={note}>{note}</p>
              ))}
            </div>
          )}
        </fieldset>

        {error && (
          <p role="alert" className="text-[12px] leading-4 text-status-waiting">
            {error}
          </p>
        )}

        <div className="flex justify-end gap-2 pt-1">
          <button
            type="button"
            onClick={close}
            className="rounded-sm border border-border-subtle px-3 py-1 text-[13px] leading-5 hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring"
          >
            取消
          </button>
          <button
            type="submit"
            disabled={pending || !workspace}
            className="rounded-sm border border-border-strong bg-accent px-3 py-1 text-[13px] leading-5 text-on-accent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-focus-ring disabled:opacity-50"
          >
            {pending ? "新增中…" : "新增"}
          </button>
        </div>
      </form>
    </dialog>
  )
}
