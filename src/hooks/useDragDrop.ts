// 從 Finder 拖入側邊欄：資料夾加入工作區、檔案路徑填入工作階段的輸入列。
import type { Event } from "@tauri-apps/api/event"
import { getCurrentWebview, type DragDropEvent } from "@tauri-apps/api/webview"
import { getCurrentWindow } from "@tauri-apps/api/window"
import { useEffect } from "react"
import { addWorkspace, classifyPaths, dispatchFiles, toCueError } from "../ipc/api"
import { syncUnlisten } from "../ipc/unlisten"
import { useCue } from "../store/useCue"
import type { Capabilities, SessionNode } from "../types"

/** 工作區清單的落點名稱（`data-drop-target` 的值）。 */
export const WORKSPACE_LIST_TARGET = "workspace-list"

const SESSION_TARGET_PREFIX = "session:"

/** 工作階段節點的落點名稱。 */
export function sessionTarget(id: string): string {
  return `${SESSION_TARGET_PREFIX}${id}`
}

/**
 * 懸停時預判能否派工，回傳拒絕原因；可以時回傳 `null`。
 * 只用於提示，放下時一律以 Rust 端依最新狀態的判斷為準。文字與 Rust 端的原因一致。
 */
export function predictDispatch(caps: Capabilities | null, node: SessionNode): string | null {
  if (!caps?.typeTextNoEnter) return "終端機後備模式無法只填入文字而不送出 Enter"
  if (node.status === "waiting") return "AI 正在等你授權，路徑中的字元可能被當成選項按鍵"
  if (node.status === "exited") return "這個工作階段已結束"
  if (node.kind.type === "other") {
    return `前景是 ${node.kind.name || "其他程式"}，路徑字元可能被當成按鍵指令`
  }
  return null
}

/** 以 CSS 像素座標找出最近的落點。 */
function targetAt(x: number, y: number): string | null {
  const el = document.elementFromPoint(x, y)
  return el?.closest<HTMLElement>("[data-drop-target]")?.dataset.dropTarget ?? null
}

function baseName(path: string): string {
  return path.replace(/\/+$/, "").split("/").pop() || path
}

async function dropOnWorkspaceList(paths: string[]): Promise<void> {
  const { setDropFeedback } = useCue.getState()
  const { folders, others } = await classifyPaths(paths)
  if (folders.length === 0) {
    setDropFeedback({ target: WORKSPACE_LIST_TARGET, tone: "error", text: "只有資料夾能加入為工作區" })
    return
  }
  const failures: string[] = []
  for (const folder of folders) {
    try {
      await addWorkspace(folder)
    } catch (e) {
      failures.push(`無法加入 ${baseName(folder)}：${toCueError(e).message}`)
    }
  }
  if (others.length > 0) failures.push(`已略過 ${others.length} 個檔案，只有資料夾能加入為工作區`)
  if (failures.length === 0) {
    setDropFeedback(null)
    return
  }
  setDropFeedback({ target: WORKSPACE_LIST_TARGET, tone: "error", text: failures.join("\n") })
}

function filledText(count: number): string {
  const filled = count === 1 ? "已填入路徑" : `已填入 ${count} 個路徑`
  return `${filled}，補上指示後按 Enter 送出`
}

async function dropOnSession(id: string, paths: string[]): Promise<void> {
  const target = sessionTarget(id)
  const { setDropFeedback } = useCue.getState()
  try {
    await dispatchFiles(id, paths)
    setDropFeedback({ target, tone: "info", text: filledText(paths.length) })
  } catch (e) {
    setDropFeedback({ target, tone: "error", text: toCueError(e).message })
  }
}

async function handleDrop(target: string, paths: string[]): Promise<void> {
  if (paths.length === 0) return
  if (target === WORKSPACE_LIST_TARGET) return dropOnWorkspaceList(paths)
  if (target.startsWith(SESSION_TARGET_PREFIX)) return dropOnSession(target.slice(SESSION_TARGET_PREFIX.length), paths)
}

/** 建立拖放事件的處理函式；縮放倍率是每次拖曳各自的狀態，所以放在閉包裡。 */
function createDragDropHandler(): (event: Event<DragDropEvent>) => void {
  // 事件座標是實體像素；換螢幕後倍率可能改變，每次拖入時重新取得。
  let scale = window.devicePixelRatio || 1

  function locate(pos: { x: number; y: number }): string | null {
    return targetAt(pos.x / scale, pos.y / scale)
  }

  function refreshScale() {
    getCurrentWindow()
      .scaleFactor()
      .then((factor) => {
        scale = factor
      })
      .catch(() => {
        // 取不到時沿用目前的倍率。
      })
  }

  return function handleDragDropEvent(event) {
    const { setDropTarget, reportError } = useCue.getState()
    const payload = event.payload
    switch (payload.type) {
      case "enter":
        setDropTarget(locate(payload.position))
        refreshScale()
        break
      case "over":
        setDropTarget(locate(payload.position))
        break
      case "drop": {
        setDropTarget(null)
        const target = locate(payload.position)
        if (target) handleDrop(target, payload.paths).catch(reportError)
        break
      }
      case "leave":
        setDropTarget(null)
        break
    }
  }
}

/** 訂閱視窗的拖放事件：懸停時更新 `dropTarget`，放下時依落點處理，拖到其他地方不做任何事。 */
export function useDragDrop(): void {
  useEffect(() => {
    const { reportError } = useCue.getState()
    return syncUnlisten(getCurrentWebview().onDragDropEvent(createDragDropHandler()), reportError)
  }, [])
}
