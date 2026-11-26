# 001 技術決策紀錄

> 決策日期：2026-11-26
> 依據：`docs/design/notes-research.md`（外部依賴與協定調查）、`docs/design/notes-probe.md`（1-2-H1 實機探針輸出）、使用者於 1-3-H1 的決定（記於 `docs/slices/slice-1-3.md`）。
> 本文件是後續切片的單一依據；切片文件與本文件不一致時以本文件為準。標示「待實機確認」的項目不影響設計，由對應切片的驗證步驟確認。

## 總覽

| # | 決策 | 結論 | 影響切片 |
|---|---|---|---|
| 1 | cwd 取得方式 | libproc（`libc` 直接呼叫 `proc_pidinfo`） | 2-1 |
| 2 | tty 正規化 | 一律轉成 `ttysNNN`，其他格式視為無 tty | 2-1、2-2、5-1 |
| 3 | 探索腳本與前景程式判斷 | 欄位維持探針版本；腳本改為批次取值；Terminal.app 分頁＝視窗 | 2-1、2-2、2-3、3-2 |
| 4 | macOS 原生 API 綁定 | `objc2` 0.6、`objc2-app-kit`／`objc2-foundation` 0.3（跟隨 Tauri） | 2-2 |
| 5 | 通知與提示 | `tauri-plugin-notification` `=2.8.1` 的 `on_action`；`waiting` 只亮燈與 Dock 徽章 | 5-2 |
| 6 | iTerm2 Socket 協定 | 不放入 `api.proto`，手寫最小訊息結構 | 7-2 |
| 7 | Hook 設定 | Claude：`UserPromptSubmit`／`PermissionRequest`＋`Notification`／`Stop`；Gemini：`BeforeAgent`／`Notification`／`AfterAgent` | 5-1、5-2 |
| 8 | 依賴授權 | 全部為 MIT 相容授權；專案維持 MIT | 7-2、8-2 |
| 9 | 最低 macOS 版本 | `26.0` | 8-1 |

---

## 1. 前景程式 cwd 取得方式

- **選項**：(a) libproc `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO)` 讀 `pvi_cdir.vip_path`；(b) 每輪批次執行 `lsof -a -p <pids> -d cwd -Fn`。
- **結論**：採用 **(a) libproc**，以 `libc` crate（0.2，`MIT OR Apache-2.0`）直接呼叫，不引入 `libproc` crate，也不使用 `lsof`。
- **理由**：
  - 兩輪實測共 48 筆，libproc 與 `lsof` 的結果完全一致。
  - libproc 每支程序約 1 µs；`lsof` 一次批次要 5–27 ms，且需要啟動子程序。
  - `libc` 已用於探針（`examples/probe_cwd.rs`），不需要新增依賴，只需從 dev-dependency 移到 dependency。
- **實作要點**：
  - `libc::proc_vnodepathinfo` 的 `vip_path` 被 `libc` 拆成 `[[c_char; 32]; 32]`，要攤平成 1024 位元組，再以 `CStr::from_bytes_until_nul` 讀出（作法同探針的 `cwd_libproc`）。
  - 回傳值 `<= 0` 為錯誤：`EPERM`（非本使用者程序）表示「無法取得 cwd」，工作階段放進「未歸類」，不當成錯誤上報；`ESRCH`（程序已結束）是正常競態，本輪略過，下一輪再試。回傳大小不等於結構大小時也略過。
  - cwd 只取**前景程序群組領頭**（見第 3.4 節）的 cwd。
- **影響**：`slice-2-1.md` 2-1-03（`process/cwd.rs`，`Cargo.toml` 把 `libc` 移到 `[dependencies]`）。

## 2. tty 名稱正規化

- **各方格式（實測）**：
  - iTerm2 AppleScript `tty of session`：`/dev/ttys000`
  - Terminal.app AppleScript `tty of tab`：`/dev/ttys021`
  - `ps -o tty=`：`ttys000`；沒有控制終端機時為 `??`
  - Hook 腳本以 `ps -o tty= -p <pid>` 取得，格式同 `ps`
- **結論**：所有來源都經同一個 `normalize_tty(&str) -> Option<String>`：
  1. 去掉前後空白。
  2. 去掉開頭的 `/dev/`。
  3. 結果必須符合 `^ttys[0-9]+$`（位數不固定，可能超過 3 位），否則回傳 `None`（例如 `??`、`console`、空字串），代表「沒有 tty」。
- **理由**：四方中只有 `/dev/` 前綴不同；用白名單格式比對，同時能擋掉 Hook 訊息中不合法的值。
- **影響**：`slice-2-1.md`（ps 解析）、`slice-2-2.md`（兩個後端的探索解析）、`slice-5-1.md`（Hook 伺服器收到的 tty 一律正規化；`cue-notify` 送原始值即可）。

## 3. 探索腳本、輪詢與前景程式判斷

### 3.1 輸出欄位（定稿）

欄位以 `\x1f`、紀錄以 `\x1e` 分隔；序號從 1 起算。欄位順序**沿用 1-2-02 探針腳本**，正式腳本不得更動順序。

**iTerm2**（每個窗格一筆，10 欄）：

| # | 欄位 | 型別 | 用途 |
|---|---|---|---|
| 1 | window id | 整數 | **視窗身分**（同一次執行期間穩定） |
| 2 | 視窗序號 | 整數 | 只用於顯示；會隨視窗前後順序變動，**不得當身分** |
| 3–6 | left、top、right、bottom | 整數（點） | 視窗框架，左上為原點；最小化時仍是還原後的正常位置 |
| 7 | 分頁序號 | 整數 | 顯示「分頁 M」；同一分頁的分割窗格相同 |
| 8 | unique ID | 大寫 UUID | **工作階段身分**（`SessionKey::Iterm2`） |
| 9 | tty | `/dev/ttysNNN` | 經第 2 節正規化 |
| 10 | name | 字串 | 只用於顯示；由 shell 或程式頻繁改寫，**不得當身分或判斷類型** |

**Terminal.app**（每個分頁一筆，9 欄）：window id、視窗序號、left、top、right、bottom、分頁序號、tty、custom title。身分為「window id + tty」（`SessionKey::TerminalApp`）。`custom title` 預設為 `使用者@主機`，只用於顯示。

### 3.2 耗時與 2 秒輪詢

- **實測**：
  - iTerm2 每個工作階段約 85 ms，呈線性：7 個約 690 ms，13 個約 1150 ms，16 個約 1400 ms。
  - Terminal.app：2 個分頁約 310 ms，3 個約 400 ms。
  - 終端機未執行時一次約 50–70 ms。
  - `ps`（約 600 支程序）55–85 ms；libproc 可忽略。
- **結論**：維持每 2 秒輪詢，但有三個條件：
  1. 探索在 `spawn_blocking` 執行；一輪結束後才排下一輪，不重疊（`MissedTickBehavior::Skip`，slice-2-3 已是如此）。
  2. **正式腳本改為批次取值**：以 `get {id, bounds} of windows`、`get unique ID of sessions of tabs of w` 這類一次取回整批屬性的寫法，取代逐一讀取，減少 Apple Event 往返。輸出欄位與第 3.1 節完全相同。2-2-02 實作後要以 `probe_discover` 的情境重新量測，目標是 16 個工作階段時 < 500 ms。
  3. 若批次取值後仍超過 1 秒，再調整輪詢間隔；M6 的 Socket 事件訂閱啟用後，輪詢降為 10 秒的保險同步（slice-7-2）。
- **未執行不啟動**：iTerm2、Terminal.app 都已實測，`application id … is running` 的前置檢查有效，結束 iTerm2 後執行探索不會把它打開。

### 3.3 Terminal.app 的分頁行為

- **實測**：以 Cmd+T 開的分頁，在 AppleScript 中**被回報為獨立的視窗**（macOS 原生視窗分頁），分頁序號永遠是 1。同一分頁群組的視窗 `bounds` 相同，但不同群組也可能剛好同位置，不能據此判斷群組。
- **結論**：
  - 身分仍用「window id + tty」，每個分頁都唯一。
  - 位置提示：Terminal.app 只顯示「視窗 N」，不顯示分頁序號；iTerm2 維持「視窗 N · 分頁 M」。
  - **關閉工作階段**：每個分頁本身就是一個 AppleScript 視窗，`close` 該視窗只會關閉這一個分頁，因此 `capabilities().close_pane` 為 `true`。防線：關閉前確認該視窗只有 1 個分頁；不只 1 個時（非原生分頁的情況）不關閉，回傳「Terminal.app 無法單獨關閉分頁」。**待實機確認**（3-2-02）：`close` 只關閉該分頁，不影響同群組的其他分頁。
- **影響**：`slice-2-2.md` 2-2-03、`slice-2-3.md`（位置提示）、`slice-3-2.md`（關閉）。

### 3.4 前景程式與類型判斷

- `ps -A -ww -o pid=,ppid=,pgid=,tpgid=,tty=,args=` 一輪只執行一次；`-ww` 避免 `args` 被欄寬截斷。
- **前景程式＝前景程序群組的領頭**（`pgid == tpgid` 且 `pid == pgid`）。`claude` 會在同一群組啟動 MCP 伺服器等子程序，部分子程序的 cwd 甚至不同，所以不能把群組內每支程序都當成前景程式；cwd 也只取領頭的 cwd。找不到領頭時（領頭已結束），取群組中 pid 最小者。
- **類型判斷**：先看領頭；領頭是包裝程式（`npm exec …`、`npx …`、`node` 但腳本不是 claude／gemini）時，再往**同一群組**的子孫程序找 `claude`／`gemini`。
  - `Claude`：執行檔 basename 為 `claude`（原生版；npm 版 2.1.332 起也是套件內附的原生執行檔，`ps` 顯示為 `claude`）；或 `node`／`bun` 執行的腳本路徑含 `@anthropic-ai/claude-code`（舊版 npm 的 `cli.js` 形式，仍要支援）；或 `npm exec @anthropic-ai/claude-code`。
  - `Gemini`：腳本檔名為 `gemini`（例如 `node …/.bin/gemini`，以及它以 `node --max-old-space-size=8192 …/.bin/gemini` 重新啟動自己的形式）；或 `npm exec @google/gemini-cli`。
  - `Zsh`：shell 本身是前景（`zsh`、`bash`、`fish`、`sh`）。比對前去掉登入 shell 開頭的 `-`（`-zsh`），並接受完整路徑（`/opt/homebrew/bin/zsh -l`）。
  - 其他：`Other(basename)`，例如 `vim`。
- **工作階段只來自 `discover()`**：`ps` 也會看到其他程式的 tty（例如 Claude 桌面 App 內建的終端機面板），只用來查詢 `discover()` 回報的 tty 的前景程式，不得自行產生工作階段。並排除 Cue 自己的子程序（例如 `ps`、`osascript`）。
- **影響**：`slice-2-1.md` 2-1-02。

## 4. macOS 原生 API 綁定

- **結論**：使用 `objc2` 0.6.5（MIT）、`objc2-foundation` 0.3.2（MIT）、`objc2-app-kit` 0.3.2（`Zlib OR Apache-2.0 OR MIT`）。版本與 Tauri 2.12.7 已依賴的版本相同（`Cargo.lock` 中已有），不會重複編譯。`Cargo.toml` 寫 `objc2 = "0.6"`、`objc2-app-kit = "0.3"`、`objc2-foundation = "0.3"`，並關閉預設 feature，只開需要的類別。
- **需要的 feature**：`objc2-app-kit`：`NSWorkspace`、`NSRunningApplication`（2-2-01）、`NSScreen`（7-1）、`NSColor`、`NSColorSpace`（主題取色時）；`objc2-foundation`：`NSString`、`NSURL`、`NSArray`、`NSGeometry`。
- **對應 API**：
  - 是否安裝：`NSWorkspace::sharedWorkspace().URLForApplicationWithBundleIdentifier(&NSString)`
  - 是否執行中：`NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString)` 非空，且 `!isTerminated()`
  - 帶到前景：`activateWithOptions`
  - 螢幕：`NSScreen::screens(mtm)`，須在主執行緒取得 `MainThreadMarker`；原點在主螢幕左下、y 向上，與 AppleScript `bounds`（左上原點）需要換算
- **影響**：`slice-2-2.md` 2-2-01。

## 5. 通知、提示音與 Dock 徽章

- **選項**：(a) `tauri-plugin-notification` ≥ 2.6 的 `on_action`；(b) 以 `objc2-user-notifications` 自行實作 `UNUserNotificationCenterDelegate`；(c) 點擊只帶起 Cue；(d) 不支援點擊聚焦。
- **結論**：採用 **(a)**，Rust 與前端都**鎖定 `=2.8.1`**（`tauri-plugin-notification`、`@tauri-apps/plugin-notification`，`Apache-2.0 OR MIT`）。
  - 通知 `extra` 帶 sessionId；Rust 端註冊 `Notification::on_action`，收到 `actionId == "tap"` 時走與 `focus_session` 相同的聚焦流程（含 `done` → `idle`）。必須註冊 `on_action`，外掛才會追蹤點擊。
  - 開發模式（`tauri::is_dev()`）下通知會以 Terminal.app 的身分送出，點擊會帶起 Terminal.app；**點擊行為只能用 `pnpm tauri build` 打包後的 App 測試**。
  - 外掛無法移除已顯示的通知（macOS 的 `close` 為空實作），Cue 不依賴撤回通知。
  - **備案**：打包後在 macOS 26 收不到 `tap` 時，改用 (b)。`objc2-user-notifications` 0.3.2 授權為 `Zlib OR Apache-2.0 OR MIT`。
  - **待實機確認**：5-2-H1 驗收時以打包後的 App 確認 macOS 26 收得到 `tap`。
- **各狀態的提示方式（使用者決定：`waiting` 不干擾當下操作）**：

  | 狀態轉變 | 燈號 | Dock 徽章 | 提示音 | macOS 通知 |
  |---|---|---|---|---|
  | → `waiting` | 亮 | 計入數量 | **不播放** | **不發送** |
  | → `done` | 亮 | — | 播放（依音效開關） | Cue 不在前景時發送（依通知開關） |

  - 任何狀態都**不搶焦點**：不把 Cue 或終端機帶到前景，也不自動切換工作階段。
- **Dock 徽章**：Tauri 的 `Window::set_badge_count(Option<i64>)` 支援 macOS（設定 `NSApp.dockTile` 的 badge），0 時傳 `None`。只從 Rust 端呼叫，不需要新增前端 capability。
- **理由**：(a) 程式量最少，也已確認 2.6.0 起桌面支援點擊回報；`waiting` 由使用者決定只需燈號提示。
- **影響**：`slice-5-2.md` 5-2-01、5-2-02、5-2-03。

## 6. iTerm2 Socket 協定（M6）

### 6.1 `api.proto` 的處理方式

- **選項**：(a) 不放入原檔，手寫最小訊息結構；(b) 詢問作者是否可另以寬鬆授權使用；(c) M6 不做 Socket 引擎。
- **結論（使用者決定）**：採用 **(a)**。
  - **不放入 `api.proto`**，不使用 `prost-build`、`protoc`，也不借用官方 Python 函式庫產生的 `api_pb2.py`（同為 GPL）。
  - 以 `prost` 的 `#[derive(prost::Message)]`／`#[derive(prost::Oneof)]`／`#[derive(prost::Enumeration)]` 手寫，**只描述協定互通所需的事實資訊**：欄位編號、wire 型別、`optional`／`repeated`（原協定為 proto2，純量欄位用 `optional`）。
  - **不複製原檔的註解、說明文字或整段內容**。Rust 型別名稱、欄位名稱、模組分組與檔案結構都以 Cue 自己的方式撰寫，不照抄原版的命名與排列；Protobuf 傳輸只依欄位編號與型別，名稱不影響相容。
  - **只實作 M6 實際用到的訊息**（見 6.3）；之後的功能需要時再逐一加入，不預先寫完整協定。沒有用到的欄位可以省略不寫，解碼時 `prost` 會略過不認得的欄位。
  - 檔頭註明「協定格式參考 iTerm2 Python API 公開文件（https://iterm2.com/python-api/）」，不附 GPL，不調整專案授權。
- **理由**：`api.proto` 沒有授權標頭，適用 iTerm2 儲存庫的 GPL-2.0；手寫只描述通訊格式的事實資訊，不引入 GPL 內容，專案授權維持不變。

### 6.2 連線方式

- Socket：`~/Library/Application Support/iTerm2/private/socket`（Unix Domain Socket）。**不使用** Python 函式庫在 socket 不存在時改連的 `ws://localhost:1912` TCP 後備。
- WebSocket：在 Unix socket 上以 `tokio::net::UnixStream::connect` 連線後，用 `tokio_tungstenite::client_async(request, stream)` 升級。請求 URI 為 `ws://localhost/`，子協定 `Sec-WebSocket-Protocol: api.iterm2.com`，不送 ping。
- 標頭：`origin: ws://localhost/`、`x-iterm2-library-version: cue <版本>`、`x-iterm2-cookie`、`x-iterm2-key`、`x-iterm2-advisory-name: Cue`、`x-iterm2-disable-auth-ui: true`。
- Cookie 與 Key：先以 `application id "com.googlecode.iterm2" is running` 檢查，執行中才以固定腳本 `request cookie and key for app named "Cue"` 取得。回傳為以單一空白分隔的 `"<cookie> <key>"`；**只存在記憶體**。
- 訊息：每則都是二進位 frame，用戶端送 `ClientOriginatedMessage`，伺服器回 `ServerOriginatedMessage`，以 `id` 配對；主動推送的通知不帶 `id`。
- **待實機確認**（7-2-02、7-2-03）：
  - iTerm2 兩種 Python API 授權模式下帶 cookie 連線是否都不額外詢問。
  - AppleScript 的 session `unique ID` 是否等於 API 的 `unique_identifier`。
  - AppleScript 視窗 `id`（整數）與 API `window_id`（字串）如何對應。

### 6.3 最小訊息集合

欄位編號與型別以 `notes-research.md` 4.3 節為準（7-2-01 的往返測試要逐一對照）。M6 只實作下列用途：

| 用途 | 協定訊息（括號內為外框 oneof 編號） |
|---|---|
| 外框 | `ClientOriginatedMessage`、`ServerOriginatedMessage`（`id`=1、`error`=2、推送 `notification`=1000） |
| 列出視窗／分頁／窗格 | `ListSessionsRequest`／`Response`（106），以及 `Window`、`Tab`、`SplitTreeNode`、`SplitTreeLink`、`SessionSummary`、`Frame`、`Point`、`Size` |
| 訂閱通知 | `NotificationRequest`／`Response`（103）、`Notification`、`NewSessionNotification`、`TerminateSessionNotification`、`LayoutChangedNotification`；`NotificationType` 只用 `NEW_SESSION`=6、`TERMINATE_SESSION`=7、`LAYOUT_CHANGE`=8 |
| 單一分頁移到新視窗 | `InvokeFunctionRequest`／`Response`（132）：`context` 設為該分頁，呼叫內建函式 `iterm2.move_tab_to_window()`，成功時 `json_result` 為新視窗 ID；原視窗必須有 2 個以上分頁 |
| 分頁併入指定視窗 | `ReorderTabsRequest`／`Response`（125）：`assignments` 給定目標視窗與分頁清單，分頁可來自任何視窗；被搬空的視窗會自動關閉 |

合計約 24 個訊息、4 個列舉。`buried_sessions`、`grid_size` 等用不到的欄位不寫。

- **影響**：`slice-7-2.md` 全部 Commit。

## 7. Claude Code 與 Gemini CLI Hook

### 7.1 Claude Code（`~/.claude/settings.json`，使用者層）

| 事件 | matcher | 狀態 |
|---|---|---|
| `UserPromptSubmit` | （無） | `running` |
| `PermissionRequest` | 省略（所有工具） | `waiting` |
| `Notification` | `elicitation_dialog\|elicitation_url_dialog` | `waiting` |
| `Stop` | （無） | `done` |
| `StopFailure` | （無） | `idle` |

- **使用者決定**：`waiting` 即時觸發，改用 `PermissionRequest`。它在 Claude 即將詢問權限時立刻觸發，`Notification` 的 `permission_prompt` 則要等約 6 秒，使用者在終端機打字時還會延後。`cue-notify` 不輸出任何內容並 `exit 0`，只是觀察，不影響權限決定。
- `Notification` 只精確比對需要使用者輸入的對話框，**排除 `idle_prompt`**（回應結束約 60 秒後的閒置提醒，不代表等待）；也不列 `permission_prompt`，避免與 `PermissionRequest` 重複。
- `StopFailure`（API 錯誤結束）回到 `idle`，避免燈號停在 `running`；不播放提示音，也不發送通知。使用者按 Esc 中斷時兩者都不觸發，`running` 可能停留，屬 Hook 驅動模型的已知限制（規格禁止以閒置計時推測）。
- `timeout` 單位為**秒**，設 `5`。
- **已知限制**：若使用者另有會自動核准的 `PermissionRequest` Hook，可能短暫亮 `waiting`。

### 7.2 Gemini CLI（`~/.gemini/settings.json`，使用者層）

| 事件 | matcher | 狀態 |
|---|---|---|
| `BeforeAgent` | `""` | `running` |
| `Notification` | `""`（`notification_type` 目前只有 `ToolPermission`） | `waiting` |
| `AfterAgent` | `""` | `done` |

- 啟用鍵 `hooksConfig.enabled` 預設為 `true`，只要寫入 `hooks` 即生效，不需要另外開啟。handler 需有 `name: "cue-notify"`、`type: "command"`。
- `timeout` 單位為**毫秒**，設 `5000`（與 Claude Code 不同）。

### 7.3 `cue-notify` 的額外要求

- **stdout 與 stderr 都必須靜默**：Gemini CLI 在 stdout 為空、stderr 有內容且 `exit 0` 時，會把 stderr 當成 `systemMessage` 顯示給使用者。腳本開頭除了 `exec 1>/dev/null`，非 `CUE_DEBUG=1` 時也要 `exec 2>/dev/null`。
- 絕不 `exit 2`（Gemini 會因此放棄該回合）；規格的「任何錯誤都靜默 `exit 0`」已涵蓋。
- tty 取得：先 `ps -o tty= -p $$`；結果為 `??` 或空時，再試 `ps -o tty= -p $PPID`（CLI 本身有控制終端機）。**待實機確認**（5-2-H1）：兩個 CLI 啟動的 Hook 程序是否繼承控制終端機。
- `cue-notify` 送原始 tty，伺服器端依第 2 節正規化。
- **影響**：`slice-5-1.md` 5-1-01、5-1-02；`slice-5-2.md` 5-2-04。

## 8. 依賴授權

專案維持 MIT。以下依賴全部與 MIT 相容，**不得引入 GPL／AGPL 依賴**（`iterm2-client` crate 為 GPL-2.0-only，不可用）。

| 依賴 | 版本 | 授權 | 使用切片 |
|---|---|---|---|
| `tauri` | 2.12 | Apache-2.0 OR MIT | 全部 |
| `libc` | 0.2 | MIT OR Apache-2.0 | 2-1 |
| `objc2` | 0.6.5 | MIT | 2-2、7-1 |
| `objc2-foundation` | 0.3.2 | MIT | 2-2、7-1 |
| `objc2-app-kit` | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 2-2、7-1 |
| `tauri-plugin-notification`／`@tauri-apps/plugin-notification` | =2.8.1 | Apache-2.0 OR MIT | 5-2 |
| `notify-rust`、`mac-notification-sys`（通知外掛的間接依賴） | 4.19／0.6 | MIT OR Apache-2.0 | 5-2 |
| `objc2-user-notifications`（僅備案） | 0.3.2 | Zlib OR Apache-2.0 OR MIT | 5-2 |
| `window-vibrancy` | 0.8 | Apache-2.0 OR MIT | 依規格 |
| `notify` | 9.0 | CC0-1.0 | 依規格 |
| `plist` | 1.10 | MIT | 依規格 |
| `tokio-tungstenite`（含 `tungstenite`） | 0.30 | MIT | 7-2 |
| `prost` | 0.14 | Apache-2.0 | 7-2（不使用 `prost-build`） |
| `zeroize` | 1.9 | Apache-2.0 OR MIT | 7-2 |
| 介面字型 | — | OFL-1.1 | 依規格 |

- `prost` 為單一 Apache-2.0 授權：散布打包後的 App 時，第三方授權清單需附 Apache-2.0 全文（8-2 的 `THIRD_PARTY_LICENSES.md`）。
- 新增依賴前確認授權；8-2 發佈前以 `cargo-about` 或 `cargo-deny` 檢查依賴樹中沒有 GPL。
- **影響**：`slice-7-2.md`（依賴授權說明）、`slice-8-2.md`（第三方授權清單；該文件已引用本節，不需修改）。

## 9. 最低 macOS 版本

- **選項**：Tauri 預設 `10.13`；`13.3`（內建 Safari 16.4，Tailwind CSS v4 的最低需求）；`26.0`（實測版本）。
- **結論（使用者決定）**：`bundle.macOS.minimumSystemVersion = "26.0"`，`install.sh` 使用同一個值檢查。
- **理由**：使用者與朋友都使用新系統；只支援實測過的版本，避免在未測試的系統上出現功能異常。之後有需要時，再放寬並以實機測試。
- **影響**：`slice-8-1.md`。
