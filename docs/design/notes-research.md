# 外部依賴與協定調查筆記

> 查閱日期：2026-11-26（以下所有來源皆為此日查閱）
> 查閱環境：macOS 26.6.2、iTerm2 3.7.8、Claude Code 2.1.332；專案目前鎖定 `tauri` 2.12.7、`@tauri-apps/api` 2.12.5、`tauri-utils` 2.10.4、`tao` 0.37.3。
> 標示「**待決定**」的項目已列入文末，交由 👤 1-3-H1 決定；標示「**待實機確認**」的項目留待對應切片以實機驗證。

---

## 1. `tauri-plugin-notification`：macOS 點擊通知回報

**結論：2.6.0 起 macOS 桌面支援點擊回報。** 目前最新穩定版 2.8.1（2026-11-10，授權 `Apache-2.0 OR MIT`）。

- 2.6.0 的變更（PR #3671）：桌面平台點擊通知本體（`actionId == "tap"`）或其動作按鈕，會送到 JS `onAction` 與新的 Rust `Notification::on_action`。同版本起 `registerActionTypes` 在桌面不再回「command not found」。
- `onAction` 的回呼參數型別改為 `ActionPerformed`（原本是 `Options`），既有程式需跟著改型別。
- 回報條件：必須先註冊 `on_action` 或存在 JS `onAction` 監聽器，否則外掛不會追蹤該通知、也不顯示動作按鈕。
- macOS 實作細節（讀 2.8.1 `src/desktop.rs`）：
  - 透過 `notify-rust` 4.18+ → `mac-notification-sys` 0.6.x，底層是已棄用的 `NSUserNotificationCenter`（非 `UNUserNotificationCenter`）。
  - 每則被追蹤的通知會開一條執行緒阻塞等待使用者回應（`wait_for_response`）；關閉通知不回報任何事件。
  - 兩個以上動作會收進「Options」選單；動作以標題區分。
  - **開發模式（`tauri::is_dev()`）下通知會以 `com.apple.Terminal` 的身分送出**，點擊通知會帶起 Terminal.app；只有打包後才用 Cue 自己的 bundle identifier。開發期間測試點擊行為要用 `pnpm tauri build` 後的 App。
  - 外掛無法移除已顯示的通知（macOS 的 `close` 為空實作）。
- 依此，Cue 可用「通知 `extra` 帶工作階段 ID → `on_action` 收到 `tap` → 聚焦該工作階段」的流程，不需自行實作 `UNUserNotificationCenterDelegate`。
- 替代方案（若實機發現 `NSUserNotification` 在 macOS 26 失效）：以 `objc2-user-notifications` 0.3.2（`Zlib OR Apache-2.0 OR MIT`）自行實作 `UNUserNotificationCenterDelegate`；或退而求其次，通知點擊只帶起 Cue，再由 Cue 聚焦最後一則通知的工作階段。
- **待實機確認**：打包後的 Cue 在 macOS 26 上點擊通知能否收到 `tap`（`NSUserNotification` 已棄用多年）。

來源：
- https://crates.io/crates/tauri-plugin-notification
- https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/CHANGELOG.md
- https://github.com/tauri-apps/plugins-workspace/pull/3671
- https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/notification/src/desktop.rs
- https://crates.io/crates/notify-rust 、 https://crates.io/crates/mac-notification-sys

---

## 2. Gemini CLI Hook

最新版 `@google/gemini-cli` 0.71.1（2026-11-24，`Apache-2.0`）。

- **啟用設定鍵**：`hooksConfig.enabled`（boolean，**預設 `true`**，變更需重新啟動）。另有 `hooksConfig.disabled`（停用的 hook 名稱陣列）、`hooksConfig.notifications`（執行時是否顯示提示，預設 `true`）。不需額外打開開關，只要在 `hooks` 寫入設定即可生效。
- **設定檔層級**（優先順序高→低）：專案 `.gemini/settings.json`、使用者 `~/.gemini/settings.json`、系統 `/etc/gemini-cli/settings.json`、擴充套件。Cue 安裝時寫入使用者層 `~/.gemini/settings.json`。
- **結構**（與 Claude Code 同型）：

  ```json
  {
    "hooks": {
      "BeforeAgent": [
        { "matcher": "", "hooks": [ { "name": "cue-notify", "type": "command", "command": "~/.cue/bin/cue-notify gemini running", "timeout": 5000 } ] }
      ]
    }
  }
  ```

  - `type` 目前只支援 `"command"`；`name` 可用於 `/hooks enable|disable <name>`。
  - **`timeout` 單位是毫秒**（預設 60000），與 Claude Code 的秒不同。
  - 定義群組可加 `sequential`。
- **事件名稱**：確認為 `BeforeAgent`（使用者送出 prompt 後、規劃前）、`AfterAgent`（每回合最終回應產生後）、`Notification`（系統提示，例如工具權限）。另有 `SessionStart`／`SessionEnd`／`BeforeModel`／`AfterModel`／`BeforeToolSelection`／`BeforeTool`／`AfterTool`／`PreCompress`。
- **Matcher**：工具事件（`BeforeTool`／`AfterTool`）為正規表示式；生命週期事件為精確字串；`"*"` 或 `""` 全部符合。
- **stdin JSON 共同欄位**：`session_id`、`transcript_path`、`cwd`、`hook_event_name`、`timestamp`（ISO 8601）。
  - `Notification` 另有 `notification_type`（目前唯一值 `"ToolPermission"`）、`message`、`details`（物件）。
  - `BeforeAgent` 有 `prompt`；`AfterAgent` 有 `prompt`、`prompt_response`、`stop_hook_active`。
- **環境變數**：`GEMINI_PROJECT_DIR`、`GEMINI_PLANS_DIR`、`GEMINI_SESSION_ID`、`GEMINI_CWD`、`CLAUDE_PROJECT_DIR`（相容別名）；環境會被淨化。
- **stdout 規則**：文件要求 stdout 只能是 JSON。讀 `hookRunner.ts` 確認實作為 `stdout.trim() || stderr.trim()`：
  - stdout 與 stderr 都空 → 不產生任何輸出，正常放行。**`cue-notify` 完全不輸出是安全的。**
  - **stdout 空但 stderr 有字、exit 0 → stderr 會被當成 `systemMessage` 顯示給使用者。** 因此 `cue-notify` 除了 stdout 靜默，**stderr 也必須靜默**（腳本開頭 `exec 2>/dev/null` 或各指令導向）。
  - exit 2 會阻擋該動作（`BeforeAgent` 為放棄這回合），`cue-notify` 絕不可回 2；規格的「任何錯誤靜默 `exit 0`」正好符合。
- 專案層 hook 有指紋機制，名稱或指令改變會被視為新的未信任 hook；Cue 只寫使用者層，不受影響。

來源：
- https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/index.md
- https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/reference.md
- https://github.com/google-gemini/gemini-cli/blob/main/docs/reference/configuration.md（`hooksConfig` 章節）
- https://github.com/google-gemini/gemini-cli/blob/main/packages/core/src/hooks/hookRunner.ts
- https://www.npmjs.com/package/@google/gemini-cli

---

## 3. Claude Code Hook

- **設定位置**：`~/.claude/settings.json`（使用者層，所有專案）、`.claude/settings.json`、`.claude/settings.local.json`、受管政策、外掛 `hooks/hooks.json` 等。Cue 寫入使用者層。
- **結構**：事件 → matcher 群組 → handler 陣列：

  ```json
  {
    "hooks": {
      "UserPromptSubmit": [ { "hooks": [ { "type": "command", "command": "~/.cue/bin/cue-notify claude running", "timeout": 5 } ] } ],
      "Notification":     [ { "matcher": "permission_prompt|elicitation_dialog", "hooks": [ { "type": "command", "command": "~/.cue/bin/cue-notify claude waiting", "timeout": 5 } ] } ],
      "Stop":             [ { "hooks": [ { "type": "command", "command": "~/.cue/bin/cue-notify claude done", "timeout": 5 } ] } ]
    }
  }
  ```

  - `timeout` 單位為**秒**；`command` 預設 600 秒，但 `UserPromptSubmit` 降為 30 秒。
  - `UserPromptSubmit`、`Stop` 沒有 matcher（省略即可）。
- **Matcher 規則**：只含字母、數字、`_`、`-`、空白、`,`、`|` 時為精確比對（可用 `|` 或 `,` 分隔多值）；含其他字元則當成不錨定的 JS 正規表示式。
- **`Notification` 的 `notification_type`**（matcher 比對此欄位）：`permission_prompt`、`idle_prompt`、`auth_success`、`auth_storage_failure`、`elicitation_dialog`、`elicitation_url_dialog`、`elicitation_complete`、`elicitation_response`、`agent_needs_input`、`agent_completed`、`quota_auto_resume_*`、`plugin_notification`。
  - `permission_prompt`：Claude 要求核准工具使用，且提示**已等待約 6 秒**（終端機內每次按鍵會延後計時）。
  - `idle_prompt`：Claude 回應結束約 **60 秒**後使用者仍未輸入。**這是閒置提醒，不能當成 `waiting`**；只把 `permission_prompt`（以及需要使用者輸入的 `elicitation_dialog`、`elicitation_url_dialog`）視為 `waiting`，matcher 寫成精確值清單即可排除 `idle_prompt`。
  - 輸入欄位：共同欄位 + `message`、選用 `title`、`notification_type`。
- **另有 `PermissionRequest` 事件**：在 Claude 即將詢問權限時**立即**觸發（不像 `permission_prompt` 有 6 秒延遲）。不輸出任何內容、exit 0 時不影響權限決定，只是觀察。**待決定**：`waiting` 要用 `PermissionRequest`（即時）或 `Notification` 的 `permission_prompt`（延遲 6 秒、使用者在打字時會延後）。
- **共同 stdin 欄位**：`session_id`、`prompt_id`、`transcript_path`、`cwd`、`permission_mode`、`hook_event_name` 等；`Stop` 另有 `stop_hook_active`、`last_assistant_message`、`background_tasks`、`session_crons`。
- **stdout 注意**：`UserPromptSubmit`（以及 `SessionStart` 等）的純文字 stdout 會被加入 Claude 的上下文，`cue-notify` 必須完全不輸出（規格已要求）。
- `UserPromptSubmit` 不只在使用者打字時觸發，也會在排程任務、背景子代理回報、跨工作階段訊息時觸發——都代表 Claude 開始工作，標為 `running` 仍正確。
- `Stop` 在使用者中斷（Esc）時**不會**觸發；API 錯誤改觸發 `StopFailure`。因此 `running` 可能因中斷而停留，屬 Hook 驅動模型的已知限制（規格禁止以閒置計時推測，不另外處理；聚焦等使用者操作可再研議）。

來源：
- https://code.claude.com/docs/en/hooks（Configuration、Matcher patterns、Common input fields、UserPromptSubmit、PermissionRequest、Notification、Stop 章節）

---

## 4. iTerm2 Python API 協定

### 4.1 `api.proto` 授權 — **待決定**

- 檔案位置：iTerm2 主程式庫 `proto/api.proto`（1739 行，`syntax = "proto2"`，`package iterm2`）。
- **檔案本身沒有任何授權或著作權標頭**，`proto/` 目錄也沒有另外的 LICENSE；因此適用整個儲存庫的 `LICENSE`，即 **GPL-2.0**。
- 官方 Python 函式庫 `iterm2`（PyPI 2.30）同為 **GPLv2+**，其中產生的 `api_pb2.py` 也不能借用。
- 結論：**無法確認 `api.proto` 可以放進 MIT 專案**，依規格須暫停並交由使用者決定。可選方案：
  1. **不放入 `api.proto`**，以 `prost` 的 `#[derive(Message)]` 手寫 Cue 實際用到的最小訊息子集（只寫欄位編號與型別，不複製原檔註解與結構全文），視為協定互通所需的事實資訊。
  2. 向 iTerm2 作者詢問 `api.proto` 是否可另以寬鬆授權使用，取得書面同意前不實作 M6。
  3. 放棄 M6 iTerm2 Socket 引擎，只保留 AppleScript 整窗操作。

### 4.2 連線方式

- **Socket 路徑**：`~/Library/Application Support/iTerm2/private/socket`（Unix Domain Socket；Python 函式庫在 socket 不存在時才退回 `ws://localhost:1912` TCP，**Cue 不得使用 TCP 後備**）。
- **WebSocket**：在 Unix socket 上做 HTTP Upgrade，請求 URI `ws://localhost/`，**子協定 `api.iterm2.com`**（`Sec-WebSocket-Protocol`）。Python 函式庫關閉 ping（`ping_interval=None`）。
- **必要 HTTP 標頭**：
  - `origin: ws://localhost/`
  - `x-iterm2-library-version: <名稱> <版本>`（Python 送 `python <版本>`；Cue 可送 `cue <版本>`）
  - `x-iterm2-cookie: <cookie>`
  - `x-iterm2-key: <key>`
  - `x-iterm2-advisory-name: <腳本名稱>`（顯示用，送 `Cue`）
  - `x-iterm2-disable-auth-ui: true`（Python 函式庫一律送出，避免跳出授權 UI）
- **申請 Cookie 與 Key**：AppleScript `tell application "iTerm2" to request cookie and key for app named "Cue"`，回傳**以單一空白分隔**的字串 `"<cookie> <key>"`。
  - Python 函式庫先檢查 iTerm2 是否執行中，未執行就放棄，不會啟動 iTerm2；Cue 腳本同樣以 `application id "com.googlecode.iterm2" is running` 先檢查。
  - App 名稱固定為 `"Cue"`，寫在腳本原文即可，不需經 argv。
  - 每則訊息是二進位 WebSocket frame，內容為序列化的 `ClientOriginatedMessage`；回應為 `ServerOriginatedMessage`，以 `id` 對應請求。
- 以 `tokio-tungstenite` 實作：`tokio::net::UnixStream::connect` 後用 `client_async(request, stream)`，`request` 自行帶上述標頭。
- **待實機確認**：iTerm2 Python API 設定中「需 Automation 權限」與「允許所有 App 連線」兩種模式下，帶 cookie 連線是否都不跳出額外詢問；AppleScript 的 session `unique ID` 是否等於 API 的 `unique_identifier`；AppleScript 的視窗 `id`（整數）與 API 的 `window_id`（字串）如何對應。

### 4.3 M6 所需最小訊息集合與欄位編號

外層包裝：

| 訊息 | 欄位 |
|---|---|
| `ClientOriginatedMessage` | `id`=1（int64）；oneof `submessage`：`notification_request`=103、`list_sessions_request`=106、`reorder_tabs_request`=125、`invoke_function_request`=132 |
| `ServerOriginatedMessage` | `id`=1；oneof：`error`=2（string）、`notification_response`=103、`list_sessions_response`=106、`reorder_tabs_response`=125、`invoke_function_response`=132、`notification`=1000（主動推送，不帶 `id`） |

列出工作階段／視窗／分頁：

| 訊息 | 欄位 |
|---|---|
| `ListSessionsRequest` | （無欄位） |
| `ListSessionsResponse` | `windows`=1（repeated Window）、`buried_sessions`=2 |
| `ListSessionsResponse.Window` | `tabs`=1、`window_id`=2、`frame`=3、`number`=4、`selected_tab_id`=5 |
| `ListSessionsResponse.Tab` | `tab_id`=2、`root`=3（SplitTreeNode）、`minimized_sessions`=6、`active_session_id`=7 |
| `SplitTreeNode` | `vertical`=1、`links`=2；`SplitTreeLink` oneof `child`：`session`=1、`node`=2 |
| `SessionSummary` | `unique_identifier`=1、`frame`=2、`grid_size`=3、`title`=4 |
| `Frame` / `Point` / `Size` | `origin`=1、`size`=2 / `x`=1、`y`=2 / `width`=1、`height`=2 |

訂閱通知：

| 訊息 | 欄位 |
|---|---|
| `NotificationRequest` | `session`=1、`subscribe`=2、`notification_type`=3 |
| `NotificationType`（enum） | `NOTIFY_ON_NEW_SESSION`=6、`NOTIFY_ON_TERMINATE_SESSION`=7、`NOTIFY_ON_LAYOUT_CHANGE`=8（這三種忽略 `session` 參數，對所有工作階段推送） |
| `NotificationResponse` | `status`=1（`OK`=0、`ALREADY_SUBSCRIBED`=4 等） |
| `Notification` | `new_session_notification`=6、`terminate_session_notification`=7、`layout_changed_notification`=8 |
| `NewSessionNotification` / `TerminateSessionNotification` | `session_id`=1 |
| `LayoutChangedNotification` | `list_sessions_response`=1（直接附完整版面，可省一次查詢） |

把單一分頁移到新視窗：

- 使用 `InvokeFunctionRequest`，`context` 設為 `tab`、呼叫內建函式 `iterm2.move_tab_to_window()`，成功時 `json_result` 為新視窗 ID（Python `Tab.async_move_to_window` 即如此實作；原分頁所在視窗須有兩個以上分頁）。
- `InvokeFunctionRequest`：oneof `context`：`tab`=1（`tab_id`=1）、`session`=2、`window`=3、`app`=4、`method`=7；`invocation`=5（string）、`timeout`=6（double，預設 -1）。
- `InvokeFunctionResponse`：oneof `disposition`：`error`=1（`status`=1、`error_reason`=2）、`success`=2（`json_result`=1）。

把分頁併入指定視窗：

- 使用 `ReorderTabsRequest`：`assignments`=3（repeated Assignment：`window_id`=1、`tab_ids`=2）。給定的分頁可來自任何視窗，會被搬到目標視窗並依序排在最前面；被搬空的視窗會自動關閉（Python `Window.async_set_tabs`）。
- `ReorderTabsResponse`：`status`=4（`OK`=0、`INVALID_ASSIGNMENT`=1、`INVALID_WINDOW_ID`=2、`INVALID_TAB_ID`=3）。

來源：
- https://github.com/gnachman/iTerm2/blob/master/proto/api.proto（最後修改 commit `744e524d4e`，2026-11-14）
- https://github.com/gnachman/iTerm2/blob/master/LICENSE
- https://pypi.org/project/iterm2/
- https://github.com/gnachman/iTerm2/blob/master/api/library/python/iterm2/iterm2/connection.py（socket 路徑、標頭、子協定）
- https://github.com/gnachman/iTerm2/blob/master/api/library/python/iterm2/iterm2/auth.py（`request cookie and key`）
- https://github.com/gnachman/iTerm2/blob/master/api/library/python/iterm2/iterm2/tab.py（`async_move_to_window`）
- https://github.com/gnachman/iTerm2/blob/master/api/library/python/iterm2/iterm2/window.py（`async_set_tabs`）
- https://iterm2.com/python-api/

---

## 5. macOS 原生 API 的 Rust 綁定

| Crate | 最新穩定版 | 授權 |
|---|---|---|
| `objc2` | 0.6.5（2026-10-05） | MIT |
| `objc2-foundation` | 0.3.2（2025-10-04） | MIT |
| `objc2-app-kit` | 0.3.2（2025-10-04） | Zlib OR Apache-2.0 OR MIT |
| `objc2-user-notifications`（備案用） | 0.3.2 | Zlib OR Apache-2.0 OR MIT |

Tauri 2.12 本身已依賴 `objc2` 0.6／`objc2-app-kit` 0.3／`objc2-foundation` 0.3，Cue 跟隨同一版本即可避免重複編譯。`objc2-app-kit` 的 Cargo feature 名稱等於類別檔名，需要的類別都要開對應 feature（例如 `NSWorkspace`、`NSRunningApplication`、`NSScreen`、`NSColor`、`NSColorSpace`，Foundation 端 `NSString`、`NSURL`、`NSArray`、`NSGeometry`）。

| 需求 | 對應 API（`objc2-app-kit` 0.3.2） |
|---|---|
| 偵測是否安裝 iTerm2 | `NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&NSString)` → `Option<Retained<NSURL>>` |
| 偵測是否執行中 | `NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString)` → `Retained<NSArray<NSRunningApplication>>`；可再看 `isTerminated()`、`isFinishedLaunching()` |
| 帶到前景 | `NSRunningApplication::activateWithOptions(NSApplicationActivationOptions)` |
| 螢幕座標 | `NSScreen::screens(mtm)`、`NSScreen::mainScreen(mtm)`、`frame()`、`visibleFrame()`、`backingScaleFactor()`、`localizedName()`（需 `MainThreadMarker`，須在主執行緒呼叫） |
| 色彩空間轉換 | `NSColor::colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())` 後讀 `redComponent()`／`greenComponent()`／`blueComponent()`／`alphaComponent()` |

注意：`NSScreen` 的座標原點在主螢幕左下、y 向上；AppleScript `bounds` 為左上原點、y 向下，`screen.rs` 需換算。

來源：
- https://crates.io/crates/objc2 、 https://crates.io/crates/objc2-foundation 、 https://crates.io/crates/objc2-app-kit 、 https://crates.io/crates/objc2-user-notifications
- https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSWorkspace.html
- https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSRunningApplication.html
- https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSScreen.html
- https://docs.rs/objc2-app-kit/0.3.2/objc2_app_kit/struct.NSColor.html
- （另以本機 Cargo registry 中 `objc2-app-kit-0.3.2/src/generated/*.rs` 核對方法簽名）

---

## 6. 其他依賴授權

| Crate | 最新穩定版 | 授權 | 與 MIT 相容 |
|---|---|---|---|
| `window-vibrancy` | 0.8.1 | Apache-2.0 OR MIT | 是 |
| `notify` | 9.0.0 | CC0-1.0 | 是（公眾領域） |
| `plist` | 1.10.4 | MIT | 是 |
| `tokio-tungstenite` | 0.30.0 | MIT | 是 |
| `tungstenite`（間接） | 0.30.0 | MIT OR Apache-2.0 | 是 |
| `prost` / `prost-build` | 0.14.6 | **Apache-2.0**（單一授權） | 是；散布二進位時須附 Apache-2.0 授權全文與 NOTICE（若有） |
| `zeroize` | 1.9.2 | Apache-2.0 OR MIT | 是 |
| `libproc` | 0.15.0 | MIT | 是（目前探針改用 `libc` 直接呼叫 `proc_pidinfo`，未採用此 crate） |
| `notify-rust`（通知外掛間接依賴） | 4.19.0 | MIT OR Apache-2.0 | 是 |
| `mac-notification-sys`（間接） | 0.6.15 | MIT/Apache-2.0 | 是 |

- 若採用 `prost-build` 就需要 `.proto` 檔與 `protoc`；依 4.1 方案 1 改用 `#[derive(prost::Message)]` 手寫時只需 `prost`，不必 `prost-build` 與 `protoc`。
- 打包發佈前建議以 `cargo-about` 或 `cargo-deny` 產生第三方授權清單並檢查 GPL（列入發佈切片考量）。

來源：
- https://crates.io/crates/window-vibrancy 、 https://crates.io/crates/notify 、 https://crates.io/crates/plist 、 https://crates.io/crates/tokio-tungstenite 、 https://crates.io/crates/tungstenite 、 https://crates.io/crates/prost 、 https://crates.io/crates/prost-build 、 https://crates.io/crates/zeroize 、 https://crates.io/crates/libproc 、 https://crates.io/crates/notify-rust 、 https://crates.io/crates/mac-notification-sys
- （版本與授權取自 crates.io API `https://crates.io/api/v1/crates/<name>` 的最新穩定版紀錄）

---

## 7. Tauri v2 細節

- **`onDragDropEvent` 座標單位**：`position` 為 **`PhysicalPosition`（實體像素）**，相對於 **webview 左上角**。與 DOM 的 CSS 像素比對時要除以 `window.devicePixelRatio`（或 `scaleFactor()`），再用 `document.elementFromPoint` 判斷放在哪個節點上。事件種類：`enter`（含 `paths`）、`over`、`drop`（含 `paths`）、`leave`。
- **`window.setTheme(theme?: 'light' | 'dark' | null)`**：v2.0.0 起提供；傳 `null` 回到跟隨系統。**macOS 上為整個 App 生效**，非單一視窗。需 capability `core:window:allow-set-theme`（不在 `core:window:default` 內）。讀取用 `theme()`，監聽用 `onThemeChanged`。
- **Dock 徽章**：
  - `setBadgeCount(count?: number)`（Rust `Window::set_badge_count(Option<i64>)`）：macOS 支援、全 App 生效；`undefined`／`None` 移除徽章。macOS 實作是設定 `NSApp.dockTile` 的 badge label（`tao` 0.37.3 `platform_impl/macos/badge.rs`）。
  - `setBadgeLabel(label?: string)`：**僅 macOS**，可顯示任意短字串。
  - 權限 `core:window:allow-set-badge-count`、`core:window:allow-set-badge-label`，都不在 `core:window:default` 內，需在 capabilities 另外加入（屬最小權限下的必要放寬，啟用時要在對應切片明列）。若只從 Rust 端呼叫則不需要前端權限。
- **`bundle.macOS.minimumSystemVersion` 預設值**：`"10.13"`（`tauri-utils` 2.10.4 `macos_minimum_system_version()`；設為空字串也會回到此預設）。Cue 使用的 `NSScreen`、`NSRunningApplication` 等 API 都早於 10.13，但 WKWebView 的能力跟隨系統安裝的 Safari 版本，而 Tailwind CSS v4 需要 Safari 16.4 以上，**待決定**是否把最低版本提高（例如 `"13.0"`），以免在舊系統安裝後畫面損壞。

來源：
- https://v2.tauri.app/reference/javascript/api/namespacewebview/#dragdropevent
- https://v2.tauri.app/reference/javascript/api/namespacewindow/#settheme
- https://v2.tauri.app/reference/javascript/api/namespacewindow/#setbadgecount
- https://docs.rs/tauri/2.12.7/tauri/window/struct.Window.html#method.set_badge_count
- https://v2.tauri.app/reference/config/#minimumsystemversion
- https://tailwindcss.com/docs/compatibility（瀏覽器需求：Safari 16.4+）
- （另以本機 `node_modules/@tauri-apps/api/{webview,window}.d.ts` 2.12.5、Cargo registry `tauri-2.12.7`、`tauri-utils-2.10.4/src/config.rs`、`tao-0.37.3` 核對）

---

## 待決定（交由 👤 1-3-H1）

1. **`api.proto` 處理方式**（見 4.1）：方案 1 手寫最小子集／方案 2 詢問作者／方案 3 放棄 M6。
2. **通知點擊**：採用 `tauri-plugin-notification` ≥ 2.6 的 `on_action`（建議），實機若失效再改用 `objc2-user-notifications` 自行實作 delegate。
3. **Claude Code `waiting` 的觸發事件**：`PermissionRequest`（即時）或 `Notification` 的 `permission_prompt`／`elicitation_dialog`（約 6 秒延遲）。兩者都排除 `idle_prompt`。
4. **`bundle.macOS.minimumSystemVersion`**：維持預設 `"10.13"` 或提高到實際測試過的版本。

## 待實機確認（各自的實作切片處理）

- 打包後 Cue 在 macOS 26 點擊通知能否收到 `tap`（第 1 節）。
- iTerm2 Python API 兩種授權模式下帶 cookie 連線的行為；AppleScript `unique ID`／視窗 `id` 與 API ID 的對應（第 4.2 節）。
