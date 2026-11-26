// Rust 端事件的訂閱與取消。
import { listen } from "@tauri-apps/api/event";
import type { CueError, Snapshot } from "../types";
import { getSnapshot, toCueError } from "./api";

/** 快照有變動時 Rust 送出的事件名稱，內容為完整快照。 */
export const SESSIONS_UPDATED = "sessions-updated";

/** 訂閱 `sessions-updated`；回傳的函式可在任何時候呼叫以取消（含訂閱尚未完成時）。 */
export function onSessionsUpdated(handler: (snapshot: Snapshot) => void): () => void {
  let cancelled = false;
  let unlisten: (() => void) | undefined;
  listen<Snapshot>(SESSIONS_UPDATED, (event) => {
    if (!cancelled) handler(event.payload);
  })
    .then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    })
    .catch(() => {
      // 訂閱失敗時沒有可做的補救；初始快照仍會載入。
    });
  return () => {
    cancelled = true;
    unlisten?.();
  };
}

/**
 * 先訂閱事件再取初始快照，讓兩者之間的變動不會遺漏。
 * 若初始快照回來之前已收到事件，事件較新，捨棄初始快照。
 */
export function subscribeSnapshots(
  onSnapshot: (snapshot: Snapshot) => void,
  onError: (error: CueError) => void,
): () => void {
  let cancelled = false;
  let receivedEvent = false;
  const stop = onSessionsUpdated((snapshot) => {
    receivedEvent = true;
    onSnapshot(snapshot);
  });
  getSnapshot()
    .then((snapshot) => {
      if (!cancelled && !receivedEvent) onSnapshot(snapshot);
    })
    .catch((e: unknown) => {
      if (!cancelled) onError(toCueError(e));
    });
  return () => {
    cancelled = true;
    stop();
  };
}
