// 前端領域型別，欄位必須與 Rust 端經 IPC 送來的格式一致；兩邊不同時以 Rust 為準。

/** 工作階段狀態。`running`／`waiting`／`done` 只由 Hook 驅動。 */
export type Status = "idle" | "running" | "waiting" | "done" | "exited";

/** 前景程式類型；`other` 帶執行檔名稱。 */
export type SessionKind =
  | { type: "claude" }
  | { type: "gemini" }
  | { type: "zsh" }
  | { type: "other"; name: string };

/** 側邊欄上的一個工作階段節點。 */
export interface SessionNode {
  /** 穩定身分（`iterm2:<unique id>` 或 `terminal:<window id>:<tty>`）。 */
  id: string;
  kind: SessionKind;
  status: Status;
  /** 所屬工作區；`null` 為未歸類。 */
  workspaceId: string | null;
  /** 前景程式的工作目錄；無法取得時為 `null`。 */
  cwd: string | null;
  tty: string;
  /** 使用者自訂標籤。 */
  label: string | null;
  /** 類型名稱（`claude`、`gemini`、`zsh` 或執行檔名）。 */
  defaultName: string;
  /** 「視窗 N · 分頁 M」或「視窗 N」，只用於顯示。 */
  locationHint: string;
  /** 終端機回報的視窗 ID，整個終端機執行期間穩定。 */
  windowId: number;
  tabIndex: number;
  /** 最近一次探索是否仍存在。 */
  alive: boolean;
}

/** 工作區。 */
export interface Workspace {
  id: string;
  path: string;
  name: string;
  collapsed: boolean;
}

/** 後端支援的操作，前端依此停用不支援的功能。 */
export interface Capabilities {
  split: boolean;
  newTab: boolean;
  typeTextNoEnter: boolean;
  rename: boolean;
  closePane: boolean;
  preciseTabs: boolean;
}

export type BackendId = "iterm2" | "terminal";

/** 沒有採用設定檔指定後端的原因。 */
export type FallbackReason = "iterm2NotInstalled";

export interface BackendInfo {
  id: BackendId;
  capabilities: Capabilities;
  installed: { iterm2: boolean; terminalApp: boolean };
  fallback: FallbackReason | null;
}

/** 前端需要的完整狀態（`get_snapshot` 回傳值與 `sessions-updated` 事件內容）。 */
export interface Snapshot {
  /** 依設定檔順序的工作區。 */
  workspaces: Workspace[];
  /** 所有節點（含已結束）。 */
  sessions: SessionNode[];
  /** 「未歸類」區要顯示的節點 id（已排除忽略清單與已結束的節點）。 */
  unclassified: string[];
  backend: BackendInfo;
}

/** Rust `CueError::kind()` 的固定代碼；`unknown` 為前端無法辨識的錯誤。 */
export type CueErrorKind =
  | "automation_denied"
  | "app_not_running"
  | "object_not_found"
  | "timeout"
  | "io"
  | "script"
  | "unsupported"
  | "session_exited"
  | "workspace_not_found"
  | "invalid_workspace"
  | "unsafe_path"
  | "invalid_label"
  | "unknown";

export interface CueError {
  kind: CueErrorKind;
  /** 可直接顯示給使用者的訊息（純文字）。 */
  message: string;
}
