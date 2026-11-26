# slice-3-1 安全指令層與建立工作階段

> 階段：slice-3 建立、管理、防呆與拖檔派工（對應規格 M2）
> 上一個子切片：slice-2-4 基礎側邊欄｜下一個子切片：slice-3-2 工作階段管理

## 目標

建立「送進終端機的文字」的唯一安全出口（shell 跳脫、控制字元拒絕、指令列舉），並實作四種開啟方式的建立工作階段與「專屬視窗」判定。

## 必讀

- `docs/PROMPT.md`「三、建立、聚焦、拖檔派工與雙螢幕版面」第 1 點與第 4 點

## 完成條件

- 注入測試：工作區資料夾 `A's 專案 $(touch /tmp/pwned) "x"` 建立工作階段時正確跳脫，`/tmp/pwned` 不會被建立。
- iTerm2 四種開啟方式都正確；Terminal.app 一律開新視窗，其他選項回傳 `Unsupported`。

---

## Commits

### 3-1-01 feat: shell 單引號跳脫與 AI 指令列舉
- **UI**：否
- **預計檔案**（2）：`src-tauri/src/shell.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `quote(s: &str) -> Result<String, CueError>`：以單引號包住，內部 `'` 轉為 `'\''`；字串含換行（`\n`、`\r`）或任何其他控制字元（`char::is_control`，含 `\0`、`\x1b`、`\x7f`）時回傳 `CueError::UnsafePath`。
  - `enum AiCommand { Claude, Gemini }`，`as_str()` 只回傳 `"claude"`／`"gemini"`。**不接受任意字串，不提供附加參數的介面**；`--dangerously-skip-permissions`、`--yolo` 等永遠不會出現在程式中。
  - `launch_command(kind: SessionKindToCreate, workspace: &Path) -> Result<String>`：`Claude` → `cd -- '<路徑>' && claude`；`Gemini` → `cd -- '<路徑>' && gemini`；`Zsh` → `cd -- '<路徑>'`。
  - 這是送入終端機文字的**唯一建構點**；其他模組不得自行組字串。
- **驗證**：單元測試：一般路徑、含 `'`、含 `$(…)`、含 `"`、含反引號、含中文與空白、含換行（拒絕）、含 `\x1b`（拒絕）；把產生的指令丟給 `/bin/zsh -c 'cd -- … && pwd'`（以 tempdir 建立名為 `A's 專案 $(touch /tmp/pwned) "x"` 的資料夾）驗證輸出正確且 `/tmp/pwned` 不存在（測試前先確認並刪除）。

### 3-1-02 feat: 開啟方式與專屬視窗判定
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/layout.rs`、`src-tauri/src/registry.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `registry.rs` 新增「最近聚焦時間」：`focus_session` 成功時記錄該節點所在視窗的聚焦時間（`window_id → Instant`）。
  - `layout.rs`：`dedicated_window(workspace_id, nodes, focus_history) -> Option<WindowId>`：
    - 專屬視窗＝該視窗內**所有**存活窗格都歸屬於此工作區（含未歸類或其他工作區窗格的混裝視窗不算）。
    - 多個符合時選最近聚焦過的；都沒聚焦過時選視窗序號最小者。
  - `resolve_new_tab_target(...)`：有專屬視窗就用它，否則回傳「最前視窗」（`None` 交給腳本取 `current window`；沒有任何視窗時改開新視窗）。
- **驗證**：單元測試：純工作區視窗、混裝視窗排除、未歸類窗格使其不算、多個候選取最近聚焦、無候選。

### 3-1-03 feat: iTerm2 建立工作階段（四種開啟方式）
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/scripts/iterm2_create_window.applescript`、`src-tauri/src/terminal/scripts/iterm2_create_tab.applescript`、`src-tauri/src/terminal/scripts/iterm2_create_split.applescript`、`src-tauri/src/terminal/mod.rs`
- **實作重點**：
  - `CreateSpec { workspace_path, kind, label, mode, target_window: Option<WindowId> }`。
  - 所有腳本以 `on run argv` 接收：要送的指令（3-1-01 產生）、目標視窗 ID（可為空字串）、zsh 窗格指令等；**腳本原文不含任何變動資料**。
  - **新視窗**：`create window with default profile` → `write text (item 1 of argv)` 到 `current session`。
  - **新分頁**：目標視窗 ID 非空時在該視窗 `create tab with default profile`，空字串時用 `current window`（無視窗則建新視窗）。
  - **AI 上／Zsh 下**：先新分頁跑 AI，再對該 session `split horizontally with default profile`（水平分隔線，新窗格在下），新窗格送 `cd -- '<路徑>'`。
  - **AI 左／Zsh 右**：同上，但 `split vertically with default profile`（垂直分隔線，新窗格在右）。
  - 腳本回傳新 AI 窗格的 `unique ID`（以及 zsh 窗格的 ID），供設定標籤。
  - 呼叫 iTerm2 前若未執行會自動啟動（使用者主動建立，允許）。
- **驗證**：`cargo clippy`；`#[ignore]` 本機整合測試：四種方式各建一次並檢查 discover 結果中新窗格的位置（下／右）。

### 3-1-04 feat: Terminal.app 建立工作階段與 create 指令
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/terminal/scripts/terminal_app_create.applescript`、`src-tauri/src/commands.rs`、`src-tauri/src/config.rs`
- **實作重點**：
  - Terminal.app：`do script (item 1 of argv)`（不指定 `in`）一律開**新視窗**，回傳新視窗 ID 與 tty；其他 `OpenMode` 回傳 `Unsupported`。
  - `create_session(workspaceId, kind, label, mode)` command：
    - `kind` 只接受 `"claude" | "gemini" | "zsh"`（反序列化到列舉，其他值直接錯誤）。
    - 依 `capabilities()` 檢查 `mode`；`NewTab` 時用 3-1-02 判定目標視窗。
    - 建立成功後立即把 `label`（若有）綁到新 SessionKey 寫入 `config.nodes`，並觸發一次探索。
    - 記住上次使用的開啟方式：`preferences.lastOpenMode`；`preferences.defaultOpenMode`（設定面板於 4-3-02 提供）若有值則作為 Modal 預設，否則用 `lastOpenMode`。
- **驗證**：`cargo test`；本機以 Terminal.app 後備模式建立 zsh 工作階段，確認新視窗開在正確目錄。
