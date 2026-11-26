import { describe, expect, it } from "vitest";
import { useCue } from "../store/useCue";

describe("useCue store", () => {
  it("建立後 sessions 與 workspaces 為空陣列", () => {
    const state = useCue.getState();
    expect(state.sessions).toEqual([]);
    expect(state.workspaces).toEqual([]);
  });
});
