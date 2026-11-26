# slice-3-2 工作階段管理

> 階段：slice-3 建立、管理、防呆與拖檔派工（對應規格 M2）
> 上一個子切片：slice-3-1 安全指令層與建立工作階段｜下一個子切片：slice-3-3 選單與建立 Modal

## 目標

後端完成既有工作階段的操作：就地補開同工作區 Zsh、重新命名（同步 iTerm2 標題）、關閉、重新開啟、移除，以及未歸類區的「設為新工作區」與「忽略」、移除工作區。

## 必讀

- `docs/PROMPT.md`「三、…」第 2 點（就地補開 Zsh）與第 6 點最後的「關閉工作階段」
- `docs/PROMPT.md`「二、…」第 3 點（未歸類區的兩個操作）、第 4 點（命名）、第 5 點（生命週期）

## 完成條件

- 「在右側補開」新窗格確實出現在右邊、「在下方補開」確實出現在下面，原 AI 對話不中斷。
- 重新命名後 iTerm2 分頁標題同步；重開 Baton 標籤仍在。
- `exited` 節點可重新開啟（相同類型、標籤、cwd）或移除。

---

## Commits

### 3-2-01 feat: 就地補開同工作區 Zsh
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/scripts/iterm2_split_zsh.applescript`、`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/commands.rs`
- **實作重點**：
  - **方向對應（注意 iTerm2 命名）**：「在右側」＝ `split vertically with default profile`（垂直分隔線，新窗格在右）；「在下方」＝ `split horizontally with default profile`（水平分隔線，新窗格在下）。
  - 腳本 argv：目標 `unique ID`、方向字串（`"vertical"`／`"horizontal"`，腳本內以 `if` 分支選擇固定語句）、`cd -- '<工作區路徑>'` 指令（由 `shell::launch_command(Zsh, …)` 產生）。
  - 只對新窗格 `write text`，**原窗格完全不送任何輸入**。
  - Terminal.app 後備：改開同工作區新視窗（`do script` 送 cd），回傳提示「後備模式改以新視窗並排」。
  - `split_zsh(sessionId, direction)` command：工作區路徑取該節點的歸屬工作區根目錄；未歸類節點則用其 cwd。
- **驗證**：本機手動：對執行中的 `claude` 窗格各執行一次右側／下方，確認位置正確且 claude 未中斷。

### 3-2-02 feat: 重新命名與關閉工作階段
- **UI**：否
- **預計檔案**（6）：`src-tauri/src/terminal/scripts/iterm2_rename.applescript`、`src-tauri/src/terminal/scripts/iterm2_close.applescript`、`src-tauri/src/terminal/scripts/terminal_app_close.applescript`、`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/commands.rs`
- **實作重點**：
  - `rename_session(id, label)`：標籤寫入 `config.nodes[key].label`（空字串＝清除標籤，回到預設名稱）；iTerm2 後端同步 `set name of s to (item 2 of argv)`。若使用者的 shell 會覆寫標題，以 shell 為準，不另外處理。
  - 3-1-04 的 `create_session` 若帶標籤，建立後也呼叫一次 `rename`。
  - `close_session(id)`：iTerm2 對該 session 送 `close`；若程式仍在執行，交由 iTerm2 自己的確認機制。Terminal.app 依 001 結論（關閉分頁或其所在視窗；多分頁時回傳提示）。
  - 關閉後觸發一次探索，節點依探索結果變為 `exited`。
  - 標籤長度上限（例如 64 字元），拒絕控制字元。
- **驗證**：本機手動：重新命名後 iTerm2 分頁標題變更；關閉後 ≤ 3 秒節點變 `exited`。

### 3-2-03 feat: 重新開啟、移除與未歸類操作
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/commands.rs`、`src-tauri/src/registry.rs`、`src-tauri/src/config.rs`
- **實作重點**：
  - `reopen_session(id)`：僅 `exited` 節點可用；以相同類型（`claude`／`gemini`／`zsh`；`Other` 類型以 `zsh` 開啟）、標籤、最後 cwd 建立新工作階段（開啟方式用預設值）；新節點繼承標籤，舊節點移除。
  - `remove_node(id)`：移除 `exited` 節點與其 `config.nodes` 紀錄。
  - `set_as_workspace(sessionId)`：以該未歸類工作階段的 cwd 呼叫 `add_workspace`，下一輪探索即依規則歸入。
  - `ignore_session(sessionId)`：加入 `config.ignored`；registry 不再把它放進未歸類清單。
  - `remove_workspace(id)`：只從清單移除（**不刪除任何檔案**），其下工作階段依規則重新歸屬。
  - **不提供**任何把工作階段手動歸入工作區的指令。
- **驗證**：`cargo test`（registry 的 ignore 過濾、reopen 參數推導）；本機手動流程。
