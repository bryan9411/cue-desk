# slice-2-4 基礎側邊欄

> 階段：slice-2 探索與認領（對應規格 M1）
> 上一個子切片：slice-2-3 探索迴圈與設定｜下一個子切片：slice-3-1 安全指令層與建立工作階段

## 目標

前端以單一 Zustand store 接上探索結果，顯示工作區樹狀側邊欄、未歸類區，點擊節點聚焦終端機。完成 M1 驗收。

## 必讀

- `docs/PROMPT.md`「四、GUI 佈局…」第 2 點（側邊欄）——只需工作區節點、工作階段節點、未歸類區的顯示部分

## 完成條件（M1 驗收）

- 事先在 iTerm2 開兩個獨立視窗都 `cd` 到同一工作區，兩者都出現在該工作區下並標示不同視窗。
- 分割窗格各自成為節點。
- 用 npm 安裝與用原生安裝的 `claude` 都能被辨識。
- 在 iTerm2 關掉分頁後 3 秒內節點變為 `exited`。
- 未歸類終端機 `cd` 進工作區後 3 秒內自動歸類；再 `cd` 到另一個工作區自動改歸屬；`cd ~` 後退回「未歸類」。
- iTerm2 未執行時 Cue 不會把它打開。
- 切換至 Terminal.app 後備模式（編輯 `~/.cue/config.json` 的 `terminalBackend` 為 `"terminal"` 後重開）也能探索與聚焦。
- `pnpm verify` 通過。

---

## Commits

### 2-4-01 feat: 前端型別、IPC 包裝與 Zustand store
- **UI**：否
- **預計檔案**（4）：`src/types.ts`、`src/ipc/api.ts`、`src/ipc/events.ts`、`src/store/useCue.ts`
- **實作重點**：
  - `types.ts`：對應 Rust 的 `Snapshot`、`SessionNode`、`Workspace`、`Status`（`'idle' | 'running' | 'waiting' | 'done' | 'exited'`）、`SessionKind`、`Capabilities`、`CueError`。
  - `ipc/api.ts`：每個 Rust command 一個具型別的包裝函式（`getSnapshot`、`focusSession`、`addWorkspace`、`setWorkspaceCollapsed`）；之後新增 command 都加在這裡。
  - `ipc/events.ts`：`listen('sessions-updated')` 的訂閱與取消。
  - `store/useCue.ts`：`workspaces`、`sessions`（以 id 為 key）、`unclassified`、`backend`、`lastError`。**所有 session 狀態更新只經由 `applySnapshot()` 與（slice-5 起）`applyStatusUpdate()` 兩個函式**。提供 selector：依工作區分組並排序（工作區內依視窗序號、分頁序號）。
- **驗證**：Vitest：`applySnapshot` 分組、排序、未歸類分離；`pnpm build`。

### 2-4-02 feat: 樹狀側邊欄與點擊聚焦
- **UI**：是
- **預計檔案**（7）：`src/components/sidebar/Sidebar.tsx`、`src/components/sidebar/WorkspaceNode.tsx`、`src/components/sidebar/SessionNode.tsx`、`src/components/sidebar/UnclassifiedSection.tsx`、`src/components/StatusLight.tsx`、`src/components/icons/Icons.tsx`、`src/App.tsx`
- **實作重點**：
  - **只用主題 CSS Variables／對應的 Tailwind utility**，不寫任何硬編碼顏色、字型、圓角。
  - `Icons.tsx`：極簡細線內嵌 SVG（`stroke="currentColor"`、`stroke-width` 1.25～1.5、`fill="none"`、24 viewBox），先做 `Plus`、`ChevronRight`、`ChevronDown`、`Folder`、`Terminal`；**不使用 emoji**。
  - **工作區節點**：摺疊箭頭、工作區名稱（`--font-heading`）、子節點數；點擊箭頭摺疊並呼叫 `setWorkspaceCollapsed` 持久化。
  - **工作階段節點**：`StatusLight`（目前只做 `dot`：以 `--status-*` 著色，`idle` 空心、`exited` 虛線空心，讓色弱者也能區分）、類型標籤、名稱、位置提示「視窗 N · 分頁 M」（`--text-muted`、`--font-mono`）；`exited` 節點降低不透明度。點擊呼叫 `focusSession`，錯誤以文字顯示在節點下方。
  - **未歸類區**：置於列表底部，顯示各工作階段目前 `cwd`（以 `~` 縮寫家目錄）。「設為新工作區」與「忽略」按鈕在 3-3-01 加入。
  - 頂部 `＋`（`Plus` 圖示按鈕，有 `aria-label`）呼叫 `@tauri-apps/plugin-dialog` 的 `open({ directory: true })` 後 `addWorkspace`。
  - 所有文字以 React 純文字渲染（**禁止 `dangerouslySetInnerHTML`**）。
  - `App.tsx` 啟動時 `getSnapshot` 並訂閱事件。
- **驗證**：`pnpm build`；`pnpm tauri dev` 實際操作：加入工作區、開關 iTerm2 分頁、`cd` 進出，畫面 ≤ 3 秒更新；點擊節點 iTerm2 切到該窗格。

### 👤 2-4-H1 M1 實機驗收
- 依本文件「完成條件（M1 驗收）」逐條操作，回報每一條通過／失敗（失敗請附畫面或描述）。
- Claude 會把結果記錄在本文件末尾的「驗收紀錄」並修正失敗項目（修正以 `2-4-H1` 之後新增的 `fix` Commit 處理）。

---

## 驗收紀錄

### 2026-11-26

以 `pnpm tauri dev` 啟動 Cue，用 AppleScript 操作 iTerm2／Terminal.app，讀取 debug 版探索迴圈印出的節點變動紀錄判定。測試工作區為暫存目錄 `ws-alpha`、`ws-beta`，測完還原 `~/.cue/config.json`。

| 項目 | 結果 | 備註 |
|---|---|---|
| 兩個獨立視窗 `cd` 到同一工作區 | 通過 | 兩節點都歸到 ws-alpha，標示「視窗 1 · 分頁 1」與「視窗 2 · 分頁 1」 |
| 分割窗格各自成為節點 | 通過 | 分割出的窗格為獨立節點（與同分頁的窗格位置提示相同） |
| npm 與原生 `claude` 皆可辨識 | 通過 | 原生 `~/.local/bin/claude`、npm 全域 `…/bin/claude` 都辨識為 `claude`。本機 npm 套件已改為原生執行檔，`node …/claude` 形式只由單元測試涵蓋 |
| 關掉分頁 3 秒內變 `exited` | 通過 | 1.67 秒 |
| `cd` 自動歸類／改歸屬／`cd ~` 退回未歸類 | 通過 | 1.2～2.5 秒（含 AppleScript 送出指令的時間） |
| Terminal.app 後備模式探索 | 通過 | `terminalBackend: "terminal"` 重開後，新視窗歸到 ws-alpha，標示「視窗 1」 |
| `pnpm verify` | 通過 | |
| 聚焦（Terminal.app） | 通過 | 以 `terminal_app_discover_and_focus_local` 呼叫後端 `focus()`：iTerm2 在前景時執行，結束後 Terminal.app 為前景且前景視窗即目標視窗 |
| 聚焦（iTerm2） | 通過 | 以 `iterm2_discover_and_focus_local` 依序聚焦 3 個窗格（跨 3 個視窗）：Terminal.app 在前景時執行，結束後 iTerm2 為前景且目前窗格即最後聚焦的窗格；不存在的窗格回傳 `object_not_found` |
| 終端機未執行時不會被打開 | 通過（以 Terminal.app 驗證） | Cue 以 Terminal.app 模式執行時結束 Terminal.app，10 秒內未被重新啟動，探索每輪 0 ms。iTerm2 後端使用同一道 `is_running` 檢查，未實際結束使用者的 iTerm2 |
| 點擊節點聚焦（使用者實機） | 通過 | 使用者以 `pnpm tauri dev` 用「＋」加入工作區、在其中開 iTerm2 分頁（自動歸類到該工作區），點擊節點確認 iTerm2 切到該窗格；關閉分頁後節點變為 `exited` |

自動化部分沒有輔助使用權限，無法在 Cue 視窗內點擊，聚焦改以後端 `focus()` 驗證；實際點擊由使用者完成。

觀察（非失敗）：同一分頁內的分割窗格位置提示相同，側邊欄上只能靠類型區分。
