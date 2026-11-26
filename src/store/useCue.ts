import { create } from "zustand";

// 工作階段與工作區的領域型別之後補上；目前只建立 store 骨架。
export interface CueState {
  sessions: unknown[];
  workspaces: unknown[];
}

export const useCue = create<CueState>()(() => ({
  sessions: [],
  workspaces: [],
}));
