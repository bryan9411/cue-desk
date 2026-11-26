import type { SessionNode } from "../types"

/** 會修改檔案的 AI 類型；zsh 與其他程式不算。 */
function isAi(node: SessionNode): boolean {
  return node.kind.type === "claude" || node.kind.type === "gemini"
}

/**
 * 工作區內仍在執行的 AI 工作階段數。
 * 同一資料夾只有一個分支，兩個以上的 AI 同時改檔可能互相覆蓋，用來顯示警示與防呆提醒。
 */
export function activeAiCount(workspaceId: string, sessions: Iterable<SessionNode>): number {
  let count = 0
  for (const node of sessions) {
    if (node.workspaceId === workspaceId && node.status !== "exited" && isAi(node)) count++
  }
  return count
}
