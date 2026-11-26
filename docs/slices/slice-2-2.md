# slice-2-2 終端機後端

> 階段：slice-2 探索與認領（對應規格 M1）
> 上一個子切片：slice-2-1 領域模型與程序解析｜下一個子切片：slice-2-3 探索迴圈與設定

> ⚠️ 探索腳本欄位、tty 正規化與 objc2 crate 版本會在 slice-1-3 定案後更新本文件。

## 目標

實作 `ITerm2Backend` 與 `TerminalAppBackend` 的 `discover()`、`focus()`、`capabilities()`，以及啟動時的終端機偵測。

## 必讀

- `docs/design/001-tech-decisions.md` 第 2～4 節
- `docs/design/notes-probe.md`（探索腳本的實機輸出）

## 完成條件

- 兩個後端都能列出所有工作階段（iTerm2 分割窗格各自一筆），並能聚焦到指定的視窗／分頁／窗格。
- 終端機未執行時 `discover()` 回傳空清單且**不會啟動該 App**；`focus()` 會啟動 App。

---

## Commits

### 2-2-01 feat: 偵測已安裝的終端機與執行狀態
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/terminal/detect.rs`、`src-tauri/src/terminal/mod.rs`、`src-tauri/Cargo.toml`
- **實作重點**：
  - 以 Launch Services 依 Bundle ID 判斷是否安裝：`NSWorkspace.sharedWorkspace.URLForApplicationWithBundleIdentifier`（`objc2-app-kit`）。iTerm2 為 `com.googlecode.iterm2`，Terminal.app 為 `com.apple.Terminal`。
  - 判斷是否執行中：`NSRunningApplication.runningApplicationsWithBundleIdentifier` 非空。**不使用會觸發啟動的 AppleScript `tell application`**。
  - `select_backend(pref: BackendPref) -> Arc<dyn TerminalBackend>`：`BackendPref` 為 `Auto`｜`Iterm2`｜`TerminalApp`；`Auto` 時有 iTerm2 用 iTerm2，否則 Terminal.app；手動指定 iTerm2 但未安裝時退回 Terminal.app 並回報原因。
- **驗證**：`#[ignore]` 測試在本機回報兩者安裝與執行狀態；`cargo clippy`。

### 2-2-02 feat: 實作 ITerm2Backend 的探索與聚焦
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/scripts/iterm2_discover.applescript`、`src-tauri/src/terminal/scripts/iterm2_focus.applescript`、`src-tauri/src/terminal/mod.rs`
- **實作重點**：
  - `discover()`：先以 2-2-01 的執行狀態檢查，未執行直接回傳空清單；執行中才跑 `iterm2_discover.applescript`（依 001 定稿的欄位），以 `osascript::parse_records` 解析成 `DiscoveredSession`。腳本本身仍保留 `is running` 防線。
  - `focus(key)`：`iterm2_focus.applescript` 以 `on run argv` 接收 `unique ID`，`activate` iTerm2 → 走訪找到該 session → 視窗 `select`（帶到最前）→ 分頁 `select` → session `select`。找不到時回傳 `NotFound`。
  - `capabilities()`：`split: true, new_tab: true, type_text_no_enter: true, rename: true, close_pane: true, precise_tabs: false`（M6 依 Socket 引擎狀態改為動態）。
  - 腳本以 `include_str!` 載入成 `&'static str`，**變動資料只透過 argv**。
- **驗證**：`cargo test`（解析測試使用 notes-probe 的範例輸出）；本機手動：`cargo test -- --ignored iterm2` 能聚焦指定窗格。

### 2-2-03 feat: 實作 TerminalAppBackend 的探索與聚焦
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/terminal/scripts/terminal_app_discover.applescript`、`src-tauri/src/terminal/scripts/terminal_app_focus.applescript`、`src-tauri/src/terminal/mod.rs`
- **實作重點**：
  - `discover()`：同 iTerm2 的未執行防線；每個分頁一筆，識別碼為「視窗 ID + tty」（`SessionKey::TerminalApp`）。
  - `focus(key)`：以視窗 ID 找到視窗，設為最前（`set index of w to 1`、`activate`），再以 tty 找到分頁設為 `selected tab`。
  - `capabilities()`：`split: false, new_tab: false, type_text_no_enter: false, rename: false, close_pane:`（依 001 結論）`, precise_tabs: false`。
- **驗證**：解析單元測試；本機手動 `cargo test -- --ignored terminal_app`。
