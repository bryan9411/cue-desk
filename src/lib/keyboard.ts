import type { KeyboardEvent } from "react"

/**
 * 輸入法正在選字。這時的 Enter 是用來確認候選字，
 * 不能當成送出或確認，否則中文輸入會在選字時就被送出。
 */
export function isImeComposing(e: KeyboardEvent): boolean {
  return e.nativeEvent.isComposing
}
