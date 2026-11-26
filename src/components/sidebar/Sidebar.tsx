import { open } from "@tauri-apps/plugin-dialog"
import { useDragDrop, WORKSPACE_LIST_TARGET } from "../../hooks/useDragDrop"
import { usePendingTask } from "../../hooks/usePendingTask"
import { addWorkspace } from "../../ipc/api"
import { selectWorkspaceGroups, useCue } from "../../store/useCue"
import { CueMark, Folder, Plus } from "../icons/Icons"
import { DropHint } from "./DropHint"
import { UnclassifiedSection } from "./UnclassifiedSection"
import { WorkspaceNode } from "./WorkspaceNode"

/** 側邊欄頂端的錯誤列，附「關閉」按鈕。文字中的換行會照樣顯示。 */
function DismissibleAlert({ text, onDismiss }: { text: string; onDismiss: () => void }) {
  return (
    <div role="alert" className="flex items-start gap-2 border-b border-border-subtle px-3 py-2 text-[12px] leading-4">
      <p className="flex-1 whitespace-pre-line text-status-waiting">{text}</p>
      <button
        type="button"
        onClick={onDismiss}
        className="shrink-0 rounded-sm text-muted underline underline-offset-2 hover:text-primary focus-visible:outline-2 focus-visible:outline-focus-ring"
      >
        關閉
      </button>
    </div>
  )
}

/** 拖曳懸停在工作區清單上時加上虛線外框。 */
function listClass(dragOver: boolean): string {
  const base = "min-h-0 flex-1 overflow-y-auto px-2 pb-4"
  if (!dragOver) return base
  return `${base} rounded-lg outline-1 -outline-offset-4 outline-dashed outline-accent`
}

/** 側邊欄：頂部標誌與加入工作區、工作區樹、最底部的「未歸類」區；也是拖入資料夾與檔案的落點。 */
export function Sidebar() {
  const groups = useCue(selectWorkspaceGroups)
  const loaded = useCue((s) => s.backend !== null)
  const lastError = useCue((s) => s.lastError)
  const setLastError = useCue((s) => s.setLastError)
  const reportError = useCue((s) => s.reportError)
  const overList = useCue((s) => s.dropTarget === WORKSPACE_LIST_TARGET)
  const listFeedback = useCue((s) => (s.dropFeedback?.target === WORKSPACE_LIST_TARGET ? s.dropFeedback : null))
  const setDropFeedback = useCue((s) => s.setDropFeedback)
  const { pending: picking, run } = usePendingTask()
  useDragDrop()

  const showEmptyState = loaded && groups.length === 0

  async function pickAndAddWorkspace() {
    const path = await open({ directory: true, multiple: false, title: "選擇工作區資料夾" })
    if (typeof path === "string") await addWorkspace(path)
  }

  function handleAddWorkspace() {
    run(pickAndAddWorkspace, reportError)
  }

  function dismissLastError() {
    setLastError(null)
  }

  function dismissListFeedback() {
    setDropFeedback(null)
  }

  return (
    <div className="relative flex h-full flex-col bg-sidebar text-primary">
      <header
        data-tauri-drag-region
        className="flex items-center gap-2 px-3 pb-2 pt-3 [border-bottom:var(--header-rule)]"
      >
        <CueMark size={18} className="pointer-events-none shrink-0 text-accent" />
        <h1 className="pointer-events-none flex-1 font-heading text-title leading-none [font-weight:var(--heading-weight)]">
          Cue
        </h1>
        <button
          type="button"
          onClick={handleAddWorkspace}
          disabled={picking}
          aria-label="加入工作區"
          title="加入工作區"
          className="rounded-sm p-1 text-muted hover:bg-surface hover:text-primary focus-visible:outline-2 focus-visible:outline-focus-ring disabled:opacity-50"
        >
          <Plus size={18} />
        </button>
      </header>

      {lastError && <DismissibleAlert text={lastError.message} onDismiss={dismissLastError} />}
      {listFeedback && <DismissibleAlert text={listFeedback.text} onDismiss={dismissListFeedback} />}

      <nav aria-label="工作區" data-drop-target={WORKSPACE_LIST_TARGET} className={listClass(overList)}>
        {showEmptyState ? (
          <div className="flex flex-col items-start gap-3 px-2 pt-6 text-[13px] leading-5 text-muted">
            <Folder size={22} />
            <p>加入專案資料夾後，在其中開啟的終端機會自動歸到它底下。也可以從 Finder 把資料夾拖到這裡。</p>
            <button
              type="button"
              onClick={handleAddWorkspace}
              disabled={picking}
              className="flex items-center gap-1.5 rounded-sm border border-border-strong px-2.5 py-1 text-primary hover:bg-surface focus-visible:outline-2 focus-visible:outline-focus-ring disabled:opacity-50"
            >
              <Plus size={14} />
              加入工作區
            </button>
          </div>
        ) : (
          <ul>
            {groups.map(({ workspace, sessions }) => (
              <WorkspaceNode key={workspace.id} workspace={workspace} sessions={sessions} />
            ))}
          </ul>
        )}
        <UnclassifiedSection />
      </nav>

      {overList && (
        <DropHint
          tone="accept"
          text="放開以加入資料夾為工作區"
          className="pointer-events-none absolute inset-x-3 bottom-3 rounded-sm border border-border-subtle bg-surface px-2 py-1.5"
        />
      )}
    </div>
  )
}
