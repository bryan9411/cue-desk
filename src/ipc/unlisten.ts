/**
 * Tauri 的監聽要等 Promise 完成才拿得到取消函式，但 React effect 的清除必須同步。
 * 這裡回傳一個隨時可呼叫的取消函式：註冊還沒完成就被呼叫時，等註冊完成後立刻取消。
 */
export function syncUnlisten(registering: Promise<() => void>, onError: (error: unknown) => void): () => void {
  let cancelled = false
  let unlisten: (() => void) | undefined

  registering
    .then((fn) => {
      if (cancelled) fn()
      else unlisten = fn
    })
    .catch(onError)

  return () => {
    cancelled = true
    unlisten?.()
  }
}
