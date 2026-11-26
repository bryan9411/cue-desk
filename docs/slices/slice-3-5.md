# slice-3-5 拖放與派工

> 階段：slice-3 建立、管理、防呆與拖檔派工（對應規格 M2）
> 上一個子切片：slice-3-4 Git 分支監聽｜下一個子切片：slice-4-1 主題引擎

## 目標

從 Finder 拖資料夾到側邊欄加入工作區；拖檔案到工作階段節點，把經跳脫的路徑填入該窗格輸入列（**絕不送出 Enter**），並在不安全時拒絕且顯示原因。完成 M2 驗收。

## 必讀

- `docs/PROMPT.md`「三、…」第 5 點（拖檔派工）

## 完成條件（M2 驗收）

- **注入測試**：工作區資料夾命名為 `A's 專案 $(touch /tmp/pwned) "x"` 時，建立工作階段與拖檔派工皆正確跳脫，且 `/tmp/pwned` 不會被建立。
- 「在右側補開」新窗格確實出現在右邊、「在下方補開」確實出現在下面，原 Claude 不中斷。
- 拖檔到 `waiting` 狀態、前景為 `vim` 的節點、或 Terminal.app 後備模式時皆被拒絕並顯示原因。
- 同目錄開啟 2 個 AI 時顯示防呆提醒與 SVG 警示徽章。
- `git switch` 後 1 秒內分支徽章更新；monorepo 子資料夾、worktree `.git` 檔、Detached HEAD、事後 `git init` 都顯示正確。
- 重開 Baton 後工作區與自訂標籤仍在。
- `npm run verify` 通過。

---

## Commits

### 3-5-01 feat: 拖檔派工規則與不送 Enter 的文字輸入
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/dispatch.rs`、`src-tauri/src/terminal/scripts/iterm2_type_no_enter.applescript`、`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `check_dispatch(caps, kind, status) -> Result<(), RejectReason>`，**三個條件都要符合**：
    1. `caps.type_text_no_enter` 為 true。Terminal.app 的 `do script` 一定會送出 Enter，因此後備模式一律拒絕（原因：「終端機後備模式無法只填入文字而不送出 Enter」）。
    2. 前景程式是 `claude`、`gemini` 或閒置的 shell（`zsh`）。`vim`、`less`、`npm` 等其他程式拒絕（原因：「前景是 vim，路徑字元可能被當成按鍵指令」）。
    3. 狀態**不是** `waiting`（原因：「AI 正在等你授權，路徑中的字元可能被當成選項按鍵」）。`exited` 也拒絕。
  - `build_paths_text(paths) -> Result<String>`：每個路徑以 `shell::quote` 跳脫（拒絕控制字元），以單一空白分隔，結尾**不加換行**。
  - iTerm2 腳本：argv 為 `unique ID` 與文字，執行 `write text (item 2 of argv) newline no`。
  - `dispatch_files(sessionId, paths)` command：檢查規則（以 Rust 端 registry 的最新狀態為準）→ `focus` → `type_text_no_enter`；拒絕時回傳 `RejectReason`（含給使用者看的中文原因）。
- **驗證**：單元測試：三個條件各自的拒絕、通過組合、路徑跳脫與控制字元拒絕；本機：拖入含 `'` 與 `$(…)` 的檔名，輸入列出現跳脫後路徑且未送出。

### 3-5-02 feat: 拖曳資料夾加入工作區與拖檔到節點
- **UI**：是
- **預計檔案**（7）：`src/hooks/useDragDrop.ts`、`src-tauri/src/commands.rs`、`src/components/sidebar/Sidebar.tsx`、`src/components/sidebar/SessionNode.tsx`、`src/components/sidebar/DropHint.tsx`、`src/ipc/api.ts`、`src/store/useBaton.ts`
- **實作重點**：
  - 使用 Tauri v2 `getCurrentWebview().onDragDropEvent`：`over` 時更新懸停目標、`drop` 時處理、`leave` 時清除。
  - **落點判斷**：事件座標為實體像素，先除以 `window.scaleFactor()` 換成 CSS 像素，再以 `document.elementFromPoint` 找最近的 `[data-drop-target]` 元素（工作區清單區塊為 `workspace-list`，工作階段節點為 `session:<id>`；寬版卡片於 4-2-03 加上同樣屬性）。
  - **拖到工作區清單**：呼叫 `classify_paths(paths)` command（Rust 以 `metadata().is_dir()` 判斷），資料夾逐一 `add_workspace`；混入的檔案忽略並提示。
  - **拖到工作階段節點**：懸停時先在前端依 store 狀態與 capabilities 預判，不可派工時節點顯示 `DropHint`（拒絕原因文字，主題變數著色）；放下時呼叫 `dispatch_files`，以後端結果為準並顯示原因。
  - 拖到其他地方：不做任何事。
- **驗證**：`npm run build`；本機依「完成條件」中的拖檔情境逐一測試（Retina 螢幕與外接螢幕各試一次落點）。

### 👤 3-5-H1 M2 實機驗收
- 依本文件「完成條件（M2 驗收）」逐條操作並回報通過／失敗。注入測試請先確認 `/tmp/pwned` 不存在，測試後再檢查一次。
- Claude 把結果記錄在本文件末尾「驗收紀錄」，失敗項目以新增的 `fix` Commit 修正。
