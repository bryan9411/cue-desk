import { describe, expect, it } from "vitest";
import type { SessionKind, SessionNode, Status } from "../../types";
import { activeAiCount } from "../workspaceStats";

function node(id: string, workspaceId: string | null, kind: SessionKind, status: Status = "idle"): SessionNode {
  return {
    id,
    kind,
    status,
    workspaceId,
    cwd: workspaceId ? `/proj/${workspaceId}` : "/Users/me",
    tty: `tty-${id}`,
    label: null,
    defaultName: kind.type === "other" ? kind.name : kind.type,
    locationHint: "視窗 1 · 分頁 1",
    windowId: 1,
    tabIndex: 1,
    alive: status !== "exited",
  };
}

const claude: SessionKind = { type: "claude" };
const gemini: SessionKind = { type: "gemini" };
const zsh: SessionKind = { type: "zsh" };

describe("activeAiCount", () => {
  it("只計算 claude 與 gemini，不含 zsh 與其他程式", () => {
    const sessions = [
      node("1", "a", claude),
      node("2", "a", gemini),
      node("3", "a", zsh),
      node("4", "a", { type: "other", name: "vim" }),
    ];
    expect(activeAiCount("a", sessions)).toBe(2);
  });

  it("不計算已結束的節點，執行中、等待與完成都算", () => {
    const sessions = [
      node("1", "a", claude, "exited"),
      node("2", "a", claude, "running"),
      node("3", "a", gemini, "waiting"),
      node("4", "a", claude, "done"),
    ];
    expect(activeAiCount("a", sessions)).toBe(3);
  });

  it("只計算指定工作區，未歸類節點不算", () => {
    const sessions = [node("1", "a", claude), node("2", "b", claude), node("3", null, claude)];
    expect(activeAiCount("a", sessions)).toBe(1);
    expect(activeAiCount("b", sessions)).toBe(1);
    expect(activeAiCount("c", sessions)).toBe(0);
  });

  it("接受以 id 為 key 的節點表的值", () => {
    const record: Record<string, SessionNode> = { x: node("x", "a", claude), y: node("y", "a", gemini) };
    expect(activeAiCount("a", Object.values(record))).toBe(2);
  });
});
