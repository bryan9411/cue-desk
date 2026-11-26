import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Snapshot } from "../types";

type Handler = (event: { payload: Snapshot }) => void;

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  handlers: [] as Handler[],
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

import { subscribeSnapshots } from "../ipc/events";
import { focusSession } from "../ipc/api";

function snap(tag: string): Snapshot {
  return {
    workspaces: [{ id: tag, path: `/${tag}`, name: tag, collapsed: false }],
    sessions: [],
    unclassified: [],
    backend: {
      id: "iterm2",
      capabilities: {
        split: false,
        newTab: false,
        typeTextNoEnter: false,
        rename: false,
        closePane: false,
        preciseTabs: false,
      },
      installed: { iterm2: true, terminalApp: true },
      fallback: null,
    },
    preferredOpenMode: null,
  };
}

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("subscribeSnapshots", () => {
  beforeEach(() => {
    mocks.handlers.length = 0;
    mocks.invoke.mockReset();
    mocks.unlisten.mockReset();
    mocks.listen.mockReset().mockImplementation(async (_name: string, h: Handler) => {
      mocks.handlers.push(h);
      return mocks.unlisten;
    });
  });

  it("訂閱 sessions-updated 並套用初始快照", async () => {
    mocks.invoke.mockResolvedValue(snap("init"));
    const got: string[] = [];
    subscribeSnapshots((s) => got.push(s.workspaces[0].id), () => {});
    await flush();
    expect(mocks.listen).toHaveBeenCalledWith("sessions-updated", expect.any(Function));
    expect(mocks.invoke).toHaveBeenCalledWith("get_snapshot", undefined);
    mocks.handlers[0]({ payload: snap("event") });
    expect(got).toEqual(["init", "event"]);
  });

  it("初始快照回來前已收到事件時捨棄較舊的初始快照", async () => {
    let resolve!: (s: Snapshot) => void;
    mocks.invoke.mockReturnValue(new Promise((r) => (resolve = r)));
    const got: string[] = [];
    subscribeSnapshots((s) => got.push(s.workspaces[0].id), () => {});
    await flush();
    mocks.handlers[0]({ payload: snap("event") });
    resolve(snap("stale"));
    await flush();
    expect(got).toEqual(["event"]);
  });

  it("取消後不再套用，並解除事件訂閱", async () => {
    mocks.invoke.mockResolvedValue(snap("init"));
    const got: string[] = [];
    const stop = subscribeSnapshots((s) => got.push(s.workspaces[0].id), () => {});
    stop();
    await flush();
    expect(got).toEqual([]);
    expect(mocks.unlisten).toHaveBeenCalledTimes(1);
  });

  it("初始快照失敗時回報 CueError", async () => {
    mocks.invoke.mockRejectedValue({ kind: "io", message: "壞了" });
    const errors: unknown[] = [];
    subscribeSnapshots(() => {}, (e) => errors.push(e));
    await flush();
    expect(errors).toEqual([{ kind: "io", message: "壞了" }]);
  });
});

describe("command 包裝", () => {
  it("非 CueError 的 rejection 轉成 unknown", async () => {
    mocks.invoke.mockReset().mockRejectedValue("boom");
    await expect(focusSession("iterm2:A")).rejects.toEqual({ kind: "unknown", message: "boom" });
    expect(mocks.invoke).toHaveBeenCalledWith("focus_session", { id: "iterm2:A" });
  });
});
