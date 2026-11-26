import { useState } from "react"
import { toCueError } from "../ipc/api"
import type { CueError } from "../types"

/**
 * 一次只執行一個非同步操作：執行中再呼叫 `run` 會被忽略，`pending` 可用來停用按鈕。
 * 失敗時把錯誤轉成 `CueError` 交給 `onError`，呼叫端不需要自己 catch。
 */
export function usePendingTask() {
  const [pending, setPending] = useState(false)

  function run(task: () => Promise<unknown>, onError: (error: CueError) => void) {
    if (pending) return
    setPending(true)
    task()
      .catch((e: unknown) => onError(toCueError(e)))
      .finally(() => setPending(false))
  }

  return { pending, run }
}
