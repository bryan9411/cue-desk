// 每個 Rust command 一個具型別的包裝函式；新增 command 時都加在這裡。
import { invoke } from "@tauri-apps/api/core"
import type { CueError, Snapshot } from "../types"

/** 判斷是否為 Rust 端序列化的 `CueError`（`{ kind, message }`）。 */
export function isCueError(value: unknown): value is CueError {
  if (typeof value !== "object" || value === null) return false
  const v = value as Record<string, unknown>
  return typeof v.kind === "string" && typeof v.message === "string"
}

/** 把任何 rejection 轉成 `CueError`，方便統一顯示。 */
export function toCueError(value: unknown): CueError {
  if (isCueError(value)) return value
  if (value instanceof Error) return { kind: "unknown", message: value.message }
  if (typeof value === "string") return { kind: "unknown", message: value }
  return { kind: "unknown", message: "發生未知錯誤" }
}

/** 呼叫 command；失敗時一律以 `CueError` reject。 */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args)
  } catch (e) {
    throw toCueError(e)
  }
}

/** 目前的完整狀態。 */
export function getSnapshot(): Promise<Snapshot> {
  return call<Snapshot>("get_snapshot")
}

/** 聚焦工作階段。已結束的節點以 `session_exited` reject。 */
export function focusSession(id: string): Promise<void> {
  return call<void>("focus_session", { id })
}

/** 加入工作區，回傳工作區 id（已存在時回傳既有的 id）。 */
export function addWorkspace(path: string): Promise<string> {
  return call<string>("add_workspace", { path })
}

/** 設定工作區收合狀態並持久化。 */
export function setWorkspaceCollapsed(id: string, collapsed: boolean): Promise<void> {
  return call<void>("set_workspace_collapsed", { id, collapsed })
}

/** 可以建立的工作階段類型；指令名稱由 Rust 端的固定列舉決定。 */
export type CreatableKind = "claude" | "gemini" | "zsh"

/** 建立工作階段的開啟方式。 */
export type OpenMode = "newWindow" | "newTab" | "aiTopZshBottom" | "aiLeftZshRight"

/** 就地補開 zsh 的位置。 */
export type SplitDirection = "right" | "below"

/** 在工作區建立工作階段，回傳新節點 id。後端不支援的開啟方式以 `unsupported` reject。 */
export function createSession(
  workspaceId: string,
  kind: CreatableKind,
  label: string | null,
  mode: OpenMode,
): Promise<string> {
  return call<string>("create_session", { workspaceId, kind, label, mode })
}

/** 在工作階段旁補開同工作區的 zsh；後端改開新視窗時回傳提示文字。 */
export function splitZsh(sessionId: string, direction: SplitDirection): Promise<string | null> {
  return call<string | null>("split_zsh", { sessionId, direction })
}

/** 重新命名工作階段；空字串清除標籤，回到預設名稱。 */
export function renameSession(id: string, label: string): Promise<void> {
  return call<void>("rename_session", { id, label })
}

/** 關閉工作階段；程式仍在執行時由終端機自己詢問。 */
export function closeSession(id: string): Promise<void> {
  return call<void>("close_session", { id })
}

/** 以原本的類型與資料夾重新開啟已結束的節點，回傳新節點 id。 */
export function reopenSession(id: string): Promise<string> {
  return call<string>("reopen_session", { id })
}

/** 移除已結束的節點與它的標籤紀錄。 */
export function removeNode(id: string): Promise<void> {
  return call<void>("remove_node", { id })
}

/** 以未歸類工作階段的 cwd 加入工作區，回傳工作區 id。 */
export function setAsWorkspace(sessionId: string): Promise<string> {
  return call<string>("set_as_workspace", { sessionId })
}

/** 不再把這個工作階段列入未歸類區。 */
export function ignoreSession(sessionId: string): Promise<void> {
  return call<void>("ignore_session", { sessionId })
}

/** 從清單移除工作區，不刪除任何檔案。 */
export function removeWorkspace(id: string): Promise<void> {
  return call<void>("remove_workspace", { id })
}

/** 拖入路徑的分類結果。 */
export interface ClassifiedPaths {
  folders: string[]
  others: string[]
}

/** 由 Rust 判斷拖入的路徑哪些是資料夾。 */
export function classifyPaths(paths: string[]): Promise<ClassifiedPaths> {
  return call<ClassifiedPaths>("classify_paths", { paths })
}

/**
 * 把路徑跳脫後填入工作階段的輸入列，不送出 Enter。
 * 不符合派工條件時以 `dispatch_rejected` reject，`message` 為給使用者看的原因。
 */
export function dispatchFiles(sessionId: string, paths: string[]): Promise<void> {
  return call<void>("dispatch_files", { sessionId, paths })
}
