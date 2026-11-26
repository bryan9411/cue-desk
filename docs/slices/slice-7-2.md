# slice-7-2 iTerm2 Socket 引擎

> 階段：slice-7 雙螢幕與 iTerm2 Socket 精準分頁（對應規格 M6）
> 上一個子切片：slice-7-1 螢幕座標與整窗調度｜下一個子切片：slice-8-1 安裝與發佈腳本

## 目標

在使用者勾選 iTerm2 `Settings › General › Magic › Enable Python API` 時，以自行手寫的最小協定子集直接連線 iTerm2 本機 Unix Domain Socket（WebSocket + Protobuf），解鎖精準分頁操作與事件驅動探索；未勾選時平滑降級為整窗操作。完成 M6 驗收。

## 必讀

- `docs/PROMPT.md`「一、系統架構」第 1 點的「iTerm2 本機 Socket 精準分頁引擎」
- `docs/design/001-tech-decisions.md` 第 6 節
- `docs/design/notes-research.md` 第 4.3 節（欄位編號與型別，7-2-01 逐一對照）

## 規格摘要

- 連線：Rust 後端透過既有 AppleScript 自動化權限，以 `request cookie and key for app named "Cue"` 取得 Cookie 與 Key，連線至 `~/Library/Application Support/iTerm2/private/socket`（WebSocket + Protobuf）——無須 Python、無須輔助使用權限、零網路 Port。首次連線 iTerm2 可能顯示授權詢問，屬正常流程。
- **授權限制（強制）**：禁止 GPL 依賴（`iterm2-client` crate 不可用）。**不放入 `api.proto`**，不使用 `prost-build`、`protoc`，也不借用官方 Python 函式庫的 `api_pb2.py`；以 `prost` 的 derive 巨集手寫最小子集（001 第 6.1 節）。
- **Cookie 與 Key 只保存在記憶體**，不得寫入設定檔、log、錯誤訊息或前端狀態；斷線重連時重新申請。
- 啟用後解鎖：從 `Cmd + T` 混裝視窗「單獨抽出單一分頁移至螢幕 2」、「只抽回單一工作區分頁並合併進同一個工作區視窗」、「一鍵依工作區自動分家（1 工作區 = 1 專屬視窗）」；改用事件訂閱（新工作階段、工作階段結束、版面變動）即時更新，輪詢降為每 10 秒一次的保險同步。

## 完成條件（M6 驗收）

- 在「Retina 筆電 + 一般解析度外接螢幕」配置下，並排終端與送到螢幕 2 位置正確、不被 Dock 或選單列遮住。
- 啟用 Python API 後可從混裝視窗單獨抽出一個分頁到螢幕 2，並可一鍵按工作區分家；停用後自動降級為整窗操作。
- 原始碼與設定檔中搜尋不到 Cookie 或 Key 的任何持久化寫入。
- 專案中沒有 `api.proto`、`api_pb2.py` 或其內容。
- `pnpm verify` 通過。

---

## Commits

### 7-2-01 feat: iTerm2 API 最小協定子集
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/iterm2_api/mod.rs`、`src-tauri/src/iterm2_api/proto.rs`、`src-tauri/Cargo.toml`
- **實作重點**（001 第 6.1、6.3 節）：
  - 以 `#[derive(prost::Message)]`／`#[derive(prost::Oneof)]`／`#[derive(prost::Enumeration)]` 手寫，**只描述協定互通所需的事實資訊**：欄位編號、wire 型別、`optional`／`repeated`（原協定為 proto2，純量欄位用 `optional`）。
  - **不複製原檔的註解、說明文字或整段內容**。Rust 型別名稱、欄位名稱、模組分組與檔案結構都以 Cue 自己的方式撰寫，不照抄原版的命名與排列。
  - **只實作 M6 用到的訊息**（約 24 個訊息、4 個列舉），用不到的欄位（例如 `buried_sessions`、`grid_size`）不寫：

    | 用途 | 協定訊息（括號內為外框 oneof 編號） |
    |---|---|
    | 外框 | 用戶端訊息、伺服器訊息（`id`=1、`error`=2、推送通知=1000） |
    | 列出視窗／分頁／窗格 | 列出工作階段請求／回應（106），以及視窗、分頁、分割樹節點與連結、窗格摘要、框架、點、尺寸 |
    | 訂閱通知 | 通知請求／回應（103）、通知、新工作階段、工作階段結束、版面變動；通知類型只用 6、7、8 |
    | 單一分頁移到新視窗 | 呼叫函式請求／回應（132） |
    | 分頁併入指定視窗 | 重排分頁請求／回應（125） |

  - `proto.rs` 檔頭註明「協定格式參考 iTerm2 Python API 公開文件（https://iterm2.com/python-api/）」；**不附 GPL，不調整專案授權**。
  - `Cargo.toml` 加入 `prost = "0.14"`（Apache-2.0；8-2 的第三方授權清單需附 Apache-2.0 全文）。**不加入 `prost-build`，不使用 `build.rs` 產生程式碼**。
- **驗證**：單元測試：每個訊息編碼後再解碼往返；以手工組出的位元組（依 `notes-research.md` 4.3 節的欄位編號與 wire 型別）解碼，確認外框 oneof 編號（103、106、125、132、1000）與各欄位編號一致；含未知欄位的位元組能被略過。

### 7-2-02 feat: 取得 Cookie／Key 並建立 Socket 連線
- **UI**：否
- **預計檔案**（7）：`src-tauri/src/iterm2_api/auth.rs`、`src-tauri/src/iterm2_api/conn.rs`、`src-tauri/src/terminal/scripts/iterm2_request_cookie.applescript`、`src-tauri/src/iterm2_api/mod.rs`、`src-tauri/Cargo.toml`、`src-tauri/src/lib.rs`、`docs/design/notes-probe.md`
- **實作重點**（001 第 6.2 節）：
  - `iterm2_request_cookie.applescript`：先以 `application id "com.googlecode.iterm2" is running` 檢查，**未執行就放棄、不啟動 iTerm2**；執行中才 `request cookie and key for app named "Cue"`（App 名稱為腳本內固定字串，不經 argv）。
  - `auth.rs`：回傳為以單一空白分隔的 `"<cookie> <key>"`，只在此函式內解析；結果包成 `Secret` 型別：**不實作 `Debug`／`Display`／`Serialize`／`Clone`**，以 `zeroize` 於 drop 時清除；錯誤訊息不得包含原始輸出。
  - `conn.rs`：
    - `tokio::net::UnixStream::connect("~/Library/Application Support/iTerm2/private/socket")` 後，以 `tokio_tungstenite::client_async(request, stream)` 升級。**不使用 `ws://localhost:1912` 的 TCP 後備**；socket 不存在即為 `NotEnabledInIterm`。
    - 請求 URI `ws://localhost/`；`Sec-WebSocket-Protocol: api.iterm2.com`；標頭 `origin: ws://localhost/`、`x-iterm2-library-version: cue <版本>`、`x-iterm2-cookie`、`x-iterm2-key`、`x-iterm2-advisory-name: Cue`、`x-iterm2-disable-auth-ui: true`；不送 ping。
    - 每則訊息為二進位 frame：送出用戶端訊息、接收伺服器訊息；請求／回應以 `id` 配對（`oneshot`），不帶 `id` 的推送通知經 `broadcast` 分送。
  - 斷線或驗證失敗時丟棄舊 Secret、重新申請後重連（指數退避，上限 30 秒）；使用者關閉開關時中斷並清除。
  - 狀態列舉：`Disabled`｜`NotEnabledInIterm`（socket 不存在）｜`Connecting`｜`Connected`｜`Unauthorized`，emit `iterm2-api-status`。
  - `Cargo.toml` 加入 `tokio-tungstenite = "0.30"`（MIT）、`zeroize = "1.9"`（Apache-2.0 OR MIT）；不開啟 TLS 相關 feature。
- **驗證**：單元測試：`Secret` 不可被格式化（以編譯期檢查或測試確認未實作 `Debug`）；回傳字串解析（正常、缺少空白、多餘空白）；本機啟用 Python API 後連線成功；`grep -rniE 'cookie|key' src-tauri/src` 檢查沒有寫檔／log 的路徑。
  - **待實機確認**，結果記錄到 `docs/design/notes-probe.md`：iTerm2「需 Automation 權限」與「允許所有 App 連線」兩種模式下帶 cookie 連線是否都不額外詢問。

### 7-2-03 feat: 透過 Socket 列出工作階段與事件訂閱
- **UI**：否
- **預計檔案**（6）：`src-tauri/src/iterm2_api/requests.rs`、`src-tauri/src/iterm2_api/events.rs`、`src-tauri/src/discovery.rs`、`src-tauri/src/config.rs`、`src-tauri/src/terminal/iterm2.rs`、`docs/design/notes-probe.md`
- **實作重點**：
  - `preferences.iterm2SocketEngine`（預設 false）。啟用且連線成功時：訂閱新工作階段（6）、工作階段結束（7）、版面變動（8）三種通知（這三種對所有工作階段推送，請求中的 session 欄位不必填）→ 收到即 `trigger_discovery()`；輪詢間隔改為 10 秒保險同步。斷線時恢復 2 秒輪詢。訂閱回應為「已訂閱」時視同成功。
  - `ITerm2Backend::capabilities()` 改為動態：連線中 `precise_tabs: true`。
  - 探索資料仍以 AppleScript 腳本為主（欄位一致、已驗證）；Socket 的列出結果（版面變動通知也直接附上完整版面）用於取得分頁／視窗 id 對應，供 7-2-04 使用。
  - **待實機確認**（先寫最小的對照探針或 `#[ignore]` 測試，結果記錄到 `docs/design/notes-probe.md`）：AppleScript 的 session `unique ID` 是否等於 API 的窗格 `unique_identifier`；AppleScript 視窗 `id`（整數）與 API `window_id`（字串）如何對應。對應方式以實測結果為準；無法對應時以 session 身分經 API 的分割樹反查所在分頁與視窗。
- **驗證**：本機：啟用後開關分頁，節點即時（< 1 秒）更新；關閉 iTerm2 Python API 後自動回到輪詢。

### 7-2-04 feat: 精準分頁操作
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/iterm2_api/requests.rs`、`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/arrange.rs`、`src-tauri/src/commands.rs`、`src-tauri/src/layout.rs`
- **實作重點**（001 第 6.3 節）：
  - **單獨抽出分頁到螢幕 2**：以呼叫函式請求（132），`context` 設為該分頁，呼叫內建函式 `iterm2.move_tab_to_window()`；成功時 `json_result` 為新視窗 ID。原視窗必須有 2 個以上分頁，只有 1 個分頁時不呼叫，直接以 7-1 的整窗移動處理。之後以 7-1 的 `set bounds` 滿版放到螢幕 2。
  - **只抽回單一工作區分頁並合併**：以重排分頁請求（125），`assignments` 給定該工作區的專屬視窗與分頁清單（分頁可來自任何視窗）；沒有專屬視窗時先以上一項把第一個分頁移到新視窗。被搬空的視窗會由 iTerm2 自動關閉。完成後放回螢幕 1。回應狀態非 OK 時回報錯誤，不重試。
  - **一鍵依工作區分家**：`plan_split_by_workspace(nodes) -> Vec<Move>` 純函式：一個分頁的**所有**窗格都屬於同一工作區才歸該工作區；含多個工作區或未歸類窗格的分頁不移動（回報給使用者）；每個工作區最後只有一個專屬視窗。
  - `tidy_tabs()` 在 `precise_tabs` 時改走分家流程，否則維持 7-1-04 的整窗降級。
  - Commands：`extract_tab_to_screen2(sessionId)`、`merge_workspace_tabs(workspaceId)`。
- **驗證**：單元測試 `plan_split_by_workspace`（純工作區分頁、混合分頁不動、未歸類不動、已是專屬視窗不重複移動）；本機用 `Cmd + T` 開混裝視窗實測。

### 7-2-05 feat: Socket 引擎設定與降級提示
- **UI**：是
- **預計檔案**（6）：`src/prefs/tabs/TerminalTab.tsx`、`src/components/sidebar/BottomBar.tsx`、`src/components/sidebar/SessionNode.tsx`、`src/commands/registry.ts`、`src/ipc/api.ts`、`src/store/useCue.ts`
- **實作重點**：
  - 偏好設定「終端機」分頁：「iTerm2 Socket 精準分頁」開關生效；顯示連線狀態（未啟用／iTerm2 尚未勾選 Python API／連線中／已連線／未授權），未勾選時顯示開啟引導：「於 iTerm2 勾選 `Settings › General › Magic › Enable Python API`。無須安裝 Python、無須開啟輔助使用權限；首次連線 iTerm2 可能詢問一次。」
  - `precise_tabs` 時：工作階段選單「送到螢幕 2 對照」改為只移動該分頁（不再顯示「連同其他分頁」提示）；工作區選單加「抽回此工作區分頁並合併」；「分頁整理」改為「依工作區分家」。未啟用時維持整窗行為。
  - `Cmd + P` 指令依 `precise_tabs` 調整文字與可用性。
- **驗證**：本機切換 iTerm2 Python API 開／關，UI 與行為自動切換。

### 👤 7-2-H1 M6 實機驗收（雙螢幕）
- 準備「Retina 筆電 + 一般解析度外接螢幕」配置（Dock 放在主螢幕下方或側邊皆可，各試一次更好）。
- 依本文件「完成條件（M6 驗收）」逐條操作並回報通過／失敗。最後一條可執行：`grep -rniE 'cookie|key' ~/.cue src-tauri/src` 並把結果貼給 Claude 判讀。
- Claude 把結果記錄在本文件末尾「驗收紀錄」，失敗項目以新增的 `fix` Commit 修正。
