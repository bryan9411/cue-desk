# slice-2-2 終端機後端

> 階段：slice-2 探索與認領（對應規格 M1）
> 上一個子切片：slice-2-1 領域模型與程序解析｜下一個子切片：slice-2-3 探索迴圈與設定

## 目標

實作 `ITerm2Backend` 與 `TerminalAppBackend` 的 `discover()`、`focus()`、`capabilities()`，以及啟動時的終端機偵測。

## 必讀

- `docs/design/001-tech-decisions.md` 第 2～4 節
- `docs/design/notes-probe.md`（探索腳本的實機輸出）

## 完成條件

- 兩個後端都能列出所有工作階段（iTerm2 分割窗格各自一筆），並能聚焦到指定的視窗／分頁／窗格。
- 終端機未執行時 `discover()` 回傳空清單且**不會啟動該 App**；`focus()` 會啟動 App。
- iTerm2 16 個工作階段時，一次探索 < 500 ms（批次取值的目標，001 第 3.2 節）。

---

## Commits

### 2-2-01 feat: 偵測已安裝的終端機與執行狀態
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/terminal/detect.rs`、`src-tauri/src/terminal/mod.rs`、`src-tauri/Cargo.toml`
- **實作重點**（001 第 4 節）：
  - `Cargo.toml` 加入（版本與 Tauri 已依賴的相同，不會重複編譯；關閉預設 feature，只開需要的類別）：
    - `objc2 = "0.6"`（MIT）
    - `objc2-foundation = "0.3"`（MIT）：`NSString`、`NSURL`、`NSArray`
    - `objc2-app-kit = "0.3"`（Zlib OR Apache-2.0 OR MIT）：`NSWorkspace`、`NSRunningApplication`
    - `NSScreen`、`NSGeometry`、`NSColor` 等 feature 由使用到的切片（7-1 等）再開啟。
  - 是否安裝：`NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&NSString)` 非 `None`。iTerm2 為 `com.googlecode.iterm2`，Terminal.app 為 `com.apple.Terminal`。
  - 是否執行中：`NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString)` 中有任一個 `!isTerminated()`。**不使用會觸發啟動的 AppleScript `tell application`**。
  - `select_backend(pref: BackendPref) -> Arc<dyn TerminalBackend>`：`BackendPref` 為 `Auto`｜`Iterm2`｜`TerminalApp`；`Auto` 時有 iTerm2 用 iTerm2，否則 Terminal.app；手動指定 iTerm2 但未安裝時退回 Terminal.app 並回報原因。
- **驗證**：`#[ignore]` 測試在本機回報兩者安裝與執行狀態；`cargo clippy`。

### 2-2-02 feat: 實作 ITerm2Backend 的探索與聚焦
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/scripts/iterm2_discover.applescript`、`src-tauri/src/terminal/scripts/iterm2_focus.applescript`、`src-tauri/src/terminal/mod.rs`、`docs/design/notes-probe.md`（記錄批次取值後的耗時）
- **實作重點**：
  - `iterm2_discover.applescript`（沿用 1-2-02 探針腳本並改寫）：
    - **輸出欄位維持 001 第 3.1 節定稿的 10 欄與順序**：window id、視窗序號、left、top、right、bottom、分頁序號、unique ID、tty、name。
    - **改為批次取值**：以 `get {id, bounds} of windows`、`get unique ID of sessions of tabs of w` 這類一次取回整批屬性的寫法取代逐一讀取，減少 Apple Event 往返。
    - 保留 `application id "com.googlecode.iterm2" is running` 的前置防線。
  - `discover()`：先以 2-2-01 的執行狀態檢查，未執行直接回傳空清單；執行中才跑腳本，以 `osascript::parse_records` 解析成 `DiscoveredSession`。
    - 身分：`unique ID` → `SessionKey::Iterm2`；視窗身分用 window id。**視窗序號與 name 只用於顯示，不得當身分或判斷類型**。
    - tty 經 `model::normalize_tty`；結果為 `None` 的窗格略過。
  - `focus(key)`：`iterm2_focus.applescript` 以 `on run argv` 接收 `unique ID`，`activate` iTerm2 → 走訪找到該 session → 視窗 `select`（帶到最前）→ 分頁 `select` → session `select`。找不到時回傳 `NotFound`。
  - `capabilities()`：`split: true, new_tab: true, type_text_no_enter: true, rename: true, close_pane: true, precise_tabs: false`（M6 依 Socket 引擎狀態改為動態）。
  - 腳本以 `include_str!` 載入成 `&'static str`，**變動資料只透過 argv**。
- **驗證**：`cargo test`（解析測試使用 notes-probe 的範例輸出）；本機手動：`cargo test -- --ignored iterm2` 能聚焦指定窗格；以 `probe_discover` 的情境重新量測耗時，記錄到 `docs/design/notes-probe.md`（16 個工作階段目標 < 500 ms；批次後仍超過 1 秒時回報，再調整輪詢間隔）。

### 2-2-03 feat: 實作 TerminalAppBackend 的探索與聚焦
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/terminal/scripts/terminal_app_discover.applescript`、`src-tauri/src/terminal/scripts/terminal_app_focus.applescript`、`src-tauri/src/terminal/mod.rs`
- **實作重點**：
  - `terminal_app_discover.applescript`（沿用探針腳本，同樣改為批次取值）：**9 欄**，順序為 window id、視窗序號、left、top、right、bottom、分頁序號、tty、custom title（001 第 3.1 節）。`custom title` 只用於顯示。
  - `discover()`：同 iTerm2 的未執行防線；每個分頁一筆，識別碼為「視窗 ID + 正規化後 tty」（`SessionKey::TerminalApp`）。
  - **分頁行為**（001 第 3.3 節）：以 Cmd+T 開的分頁在 AppleScript 中被回報為**獨立的視窗**，分頁序號永遠是 1；同群組的視窗 `bounds` 相同，但不能據此判斷群組。
  - `focus(key)`：以視窗 ID 找到視窗，設為最前（`set index of w to 1`、`activate`），再以 tty 找到分頁設為 `selected tab`。
  - `capabilities()`：`split: false, new_tab: false, type_text_no_enter: false, rename: false, close_pane: true, precise_tabs: false`（關閉方式見 slice-3-2 的 3-2-02）。
- **驗證**：解析單元測試；本機手動 `cargo test -- --ignored terminal_app`（含以 Cmd+T 開的分頁能各自聚焦）。
