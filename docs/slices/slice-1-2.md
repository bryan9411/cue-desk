# slice-1-2 技術探針

> 階段：slice-1 技術驗證與專案骨架（風險先行）
> 上一個子切片：slice-1-1 專案骨架｜下一個子切片：slice-1-3 技術決策

## 目標

在寫正式功能前，先用實機驗證風險最高的幾件事，並把外部依賴查清楚：

1. `osascript` 以 `on run argv` 傳參數、以 `\x1f`／`\x1e` 回傳結構化資料可行（此執行器會直接沿用到正式程式）。
2. iTerm2 與 Terminal.app 的 AppleScript 能取得探索所需的所有欄位，且**不會把未執行的 App 打開**。
3. 前景程式 `cwd` 的取得方式：libproc（`PROC_PIDVNODEPATHINFO`）vs 批次 `lsof`。
4. 外部依賴的版本、授權與協定細節。

## 必讀

- `docs/PROMPT.md`「一、系統架構」第 1 點（AppleScript 執行方式、iTerm2 Socket 引擎、授權限制）
- `docs/PROMPT.md`「二、工作階段探索與認領」第 1、2 點

## 完成條件

- `cargo run --example probe_discover` 與 `cargo run --example probe_cwd` 能在實機輸出預期資料。
- `docs/design/notes-research.md` 回答下方 1-2-04 列出的所有問題，並附來源網址。
- 👤 1-2-H1 的實機輸出已記錄到 `docs/design/notes-probe.md`。

---

## Commits

### 1-2-01 feat: osascript 執行器與分隔字元解析
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/osascript.rs`、`src-tauri/src/error.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `run(script: &'static str, args: &[&str]) -> Result<String, CueError>`：以 `osascript - <arg1> <arg2> …` 執行，**腳本原文從 stdin 寫入**，變動資料只放在 argv。腳本參數型別為 `&'static str`，從型別上杜絕把執行期資料拼進腳本。
  - 逾時（預設 5 秒），逾時就 kill 子程序並回傳錯誤。
  - `parse_records(output) -> Vec<Vec<String>>`：以 `\x1e` 切紀錄、`\x1f` 切欄位，去掉結尾換行；空輸出回傳空陣列。
  - 錯誤分類：stderr 含 `-1743`（errAEEventNotPermitted，使用者拒絕自動化權限）→ `CueError::AutomationDenied`；`-600`／`-1728` 等 App 未執行或找不到物件 → 對應錯誤；其他 → `CueError::Script(stderr 摘要)`。
  - `CueError` 實作 `serde::Serialize`（給前端顯示的簡短訊息），**不得含任何機密**。
  - 單元測試：`parse_records` 的空輸出、單筆、多筆、欄位含空白與中文。整合測試（`#[ignore]`）：執行 `on run argv` 回傳 `item 1 of argv & (ASCII character 31) & item 2 of argv`，驗證含 `'`、`"`、`$(…)`、換行的參數原樣往返。
- **驗證**：`cargo test`；`cargo test -- --ignored osascript` 在本機通過。

### 1-2-02 spike: iTerm2 與 Terminal.app 探索腳本探針
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/terminal/scripts/iterm2_discover.applescript`、`src-tauri/src/terminal/scripts/terminal_app_discover.applescript`、`src-tauri/examples/probe_discover.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - 兩支腳本都以 `on run argv` 開頭，第一步先檢查 `application id "<bundle id>" is running`，未執行就回傳空字串（**不得用 `tell application` 觸發啟動**）。iTerm2 為 `com.googlecode.iterm2`，Terminal.app 為 `com.apple.Terminal`。
  - **iTerm2**：逐一走訪 windows → tabs → sessions，每個 session 輸出一筆：`window id`、視窗序號、`bounds`（四個數字）、分頁序號、`unique ID`、`tty`、`name`。
  - **Terminal.app**：逐一走訪 windows → tabs，每個 tab 輸出：`window id`、視窗序號、`bounds`、分頁序號、`tty`、`custom title`。
  - 欄位以 `(ASCII character 31)`、紀錄以 `(ASCII character 30)` 串接。
  - `probe_discover.rs`：呼叫 `cue_lib::osascript` 執行兩支腳本並以表格印出，同時記錄執行耗時（評估每 2 秒輪詢的成本）。為了讓 example 能用，`lib.rs` 需 `pub mod osascript;` 並以 `pub(crate)` 以外的方式公開腳本常數（例如 `pub mod terminal { pub mod scripts; }` 先放常數）。
  - 記錄觀察：`tty` 的格式（`/dev/ttys003` 或 `ttys003`）、視窗 ID 是否穩定、分割窗格是否各自一筆、最小化視窗與全螢幕視窗的 bounds、iTerm2 未執行時是否被啟動。
- **驗證**：`cargo run --manifest-path src-tauri/Cargo.toml --example probe_discover` 編譯成功並在本機輸出；實機情境由 👤 1-2-H1 回報。

### 1-2-03 spike: 前景程式工作目錄取得方式探針
- **UI**：否
- **預計檔案**（2）：`src-tauri/examples/probe_cwd.rs`、`src-tauri/Cargo.toml`
- **實作重點**：
  - 執行一次 `ps -A -o pid=,ppid=,pgid=,tpgid=,tty=,args=`，找出每個 tty 的前景程序群組（`pgid == tpgid`）。
  - 對這些 pid 分別用兩種方式取 cwd 並比較結果與耗時：
    - **libproc**：`proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, …)` 取 `pvi_cdir`。可用 `libproc` crate（**先確認授權為 MIT 相容**），或以 `libc`／自行 `extern "C"` 宣告。
    - **lsof**：`lsof -a -p <pid1,pid2,…> -d cwd -Fn` 批次執行並解析 `p`／`n` 行。
  - 印出：pid、tty、args 前 60 字、兩種方式的 cwd、各自耗時。特別觀察 `claude`（原生與 npm 版）、`gemini`、`zsh` 是否都能取到；非本使用者程序的錯誤處理。
- **驗證**：`cargo run --manifest-path src-tauri/Cargo.toml --example probe_cwd` 在本機輸出結果。

### 1-2-04 docs: 外部依賴與協定調查筆記
- **UI**：否
- **預計檔案**（1）：`docs/design/notes-research.md`
- **實作重點**：查閱官方文件、changelog、原始碼授權（每項附來源網址與查閱日期），回答：
  1. **`tauri-plugin-notification`**：哪個版本起在 **macOS 桌面**支援點擊通知回報（JS `onAction` 或 Rust `Notification::on_action`）？若尚無版本支援，列出替代方案（例如以 `objc2-user-notifications` 自行實作 `UNUserNotificationCenterDelegate`，或通知點擊只帶起 Cue 再由 Cue 聚焦最後一則通知的工作階段）。
  2. **Gemini CLI Hook**：啟用 Hook 的設定鍵、`settings.json` 結構、事件名稱（是否為 `BeforeAgent`／`Notification`／`AfterAgent`）、Hook 收到的 stdin JSON 欄位（`session_id`、`transcript_path`）、是否要求 stdout 為 JSON。
  3. **Claude Code Hook**：`UserPromptSubmit`／`Notification`／`Stop` 的設定格式；`Notification` 事件的 matcher（例如 `permission_prompt` 與 `idle_prompt` 的區分），以免閒置提醒誤判為 `waiting`。
  4. **iTerm2 Python API 協定**：
     - `api.proto` 的授權（iTerm2 主程式為 GPL，需確認該檔是否另有授權），能否放進 MIT 專案。
     - 連線方式：socket 路徑、WebSocket 子協定、必要的 HTTP 標頭（cookie、key、library version 等）、`request cookie and key for app named "Cue"` 的 AppleScript 用法與回傳格式。
     - 完成 M6 需要的最小訊息集合：列出工作階段／視窗／分頁、訂閱新工作階段／工作階段結束／版面變動通知、**把單一分頁移到新視窗**、**把分頁併入指定視窗**（例如 `ReorderTabsRequest`、`InvokeFunctionRequest` 或其他），各自的欄位編號。
  5. **macOS 原生 API 的 Rust 綁定**：`objc2`、`objc2-app-kit`、`objc2-foundation` 的授權與版本；`NSWorkspace URLForApplicationWithBundleIdentifier`、`NSRunningApplication runningApplicationsWithBundleIdentifier`、`NSScreen frame/visibleFrame`、`NSColor` 色彩空間轉換的對應 API。
  6. **其他依賴授權**：`window-vibrancy`、`notify`、`plist`、`tokio-tungstenite`、`prost`、`zeroize`、`libproc`（若採用）。
  7. **Tauri v2 細節**：`onDragDropEvent` 的座標單位、`window.setTheme()`、`set_badge_count`（Dock 徽章）在 macOS 的支援狀況、`bundle.macOS.minimumSystemVersion` 預設值。
- **驗證**：每個問題都有結論與來源；無法確認的項目明確標示「待決定」並列入 👤 1-3-H1。

### 👤 1-2-H1 在 iTerm2／Terminal.app 準備情境並回報探針輸出
- 請在 iTerm2 準備以下情境，然後執行兩支探針並把輸出貼給 Claude：
  1. 兩個獨立視窗都 `cd` 到同一個資料夾。
  2. 其中一個分頁做一次左右分割、一次上下分割。
  3. 若有：一個分頁執行原生安裝的 `claude`、一個執行 npm 安裝的 `claude`、一個執行 `gemini`。
  4. 一個分頁執行 `vim`。
  5. 結束 iTerm2 後再執行一次 `probe_discover`，確認 iTerm2 沒被打開。
  6. 在 Terminal.app 開兩個視窗（其中一個有兩個分頁）再執行一次。
- 指令：
  - `cargo run --manifest-path src-tauri/Cargo.toml --example probe_discover`
  - `cargo run --manifest-path src-tauri/Cargo.toml --example probe_cwd`
- Claude 收到後整理到 `docs/design/notes-probe.md`（計為本項的附帶紀錄，與 1-3-01 一起 commit）。
