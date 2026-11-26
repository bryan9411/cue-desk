// 每個 Rust command 一個具型別的包裝函式；新增 command 時都加在這裡。
import { invoke } from "@tauri-apps/api/core";
import type { CueError, Snapshot } from "../types";

/** 判斷是否為 Rust 端序列化的 `CueError`（`{ kind, message }`）。 */
export function isCueError(value: unknown): value is CueError {
  if (typeof value !== "object" || value === null) return false;
  const v = value as Record<string, unknown>;
  return typeof v.kind === "string" && typeof v.message === "string";
}

/** 把任何 rejection 轉成 `CueError`，方便統一顯示。 */
export function toCueError(value: unknown): CueError {
  if (isCueError(value)) return value;
  if (value instanceof Error) return { kind: "unknown", message: value.message };
  if (typeof value === "string") return { kind: "unknown", message: value };
  return { kind: "unknown", message: "發生未知錯誤" };
}

/** 呼叫 command；失敗時一律以 `CueError` reject。 */
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toCueError(e);
  }
}

/** 目前的完整狀態。 */
export function getSnapshot(): Promise<Snapshot> {
  return call<Snapshot>("get_snapshot");
}

/** 聚焦工作階段。已結束的節點以 `session_exited` reject。 */
export function focusSession(id: string): Promise<void> {
  return call<void>("focus_session", { id });
}

/** 加入工作區，回傳工作區 id（已存在時回傳既有的 id）。 */
export function addWorkspace(path: string): Promise<string> {
  return call<string>("add_workspace", { path });
}

/** 設定工作區收合狀態並持久化。 */
export function setWorkspaceCollapsed(id: string, collapsed: boolean): Promise<void> {
  return call<void>("set_workspace_collapsed", { id, collapsed });
}
