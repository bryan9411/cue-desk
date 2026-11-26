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
- `npm run verify` 通過。

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
- **驗證**：Vitest：`applySnapshot` 分組、排序、未歸類分離；`npm run build`。

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
- **驗證**：`npm run build`；`npm run tauri dev` 實際操作：加入工作區、開關 iTerm2 分頁、`cd` 進出，畫面 ≤ 3 秒更新；點擊節點 iTerm2 切到該窗格。

### 👤 2-4-H1 M1 實機驗收
- 依本文件「完成條件（M1 驗收）」逐條操作，回報每一條通過／失敗（失敗請附畫面或描述）。
- Claude 會把結果記錄在本文件末尾的「驗收紀錄」並修正失敗項目（修正以 `2-4-H1` 之後新增的 `fix` Commit 處理）。
