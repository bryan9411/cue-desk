# slice-7-2 iTerm2 Socket 引擎

> 階段：slice-7 雙螢幕與 iTerm2 Socket 精準分頁（對應規格 M6）
> 上一個子切片：slice-7-1 螢幕座標與整窗調度｜下一個子切片：slice-8-1 安裝與發佈腳本

> ⚠️ 協定細節（`api.proto` 處理方式、連線標頭、訊息欄位編號、移動分頁所用的 RPC）會在 slice-1-3 依調查結果與使用者決定更新本文件。若 1-3-H1 決定「M6 不做 Socket 引擎」，本子切片改為只保留 7-2-05 的引導文案並移除其餘 Commit。

## 目標

在使用者勾選 iTerm2 `Settings › General › Magic › Enable Python API` 時，以自行實作的最小協定子集直接連線 iTerm2 本機 Unix Domain Socket（WebSocket + Protobuf），解鎖精準分頁操作與事件驅動探索；未勾選時平滑降級為整窗操作。完成 M6 驗收。

## 必讀

- `docs/PROMPT.md`「一、系統架構」第 1 點的「iTerm2 本機 Socket 精準分頁引擎」
- `docs/design/001-tech-decisions.md` 第 6 節

## 規格摘要

- 連線：Rust 後端透過既有 AppleScript 自動化權限，以 `request cookie and key for app named "Cue"` 取得 Cookie 與 Key，連線至 `~/Library/Application Support/iTerm2/private/socket`（WebSocket + Protobuf）——無須 Python、無須輔助使用權限、零網路 Port。首次連線 iTerm2 可能顯示授權詢問，屬正常流程。
- **授權限制（強制）**：禁止 GPL 依賴（`iterm2-client` crate 不可用）；以 `tokio-tungstenite` + `prost` 自行實作最小子集；`api.proto` 依 001 決定處理。
- **Cookie 與 Key 只保存在記憶體**，不得寫入設定檔、log、錯誤訊息或前端狀態；斷線重連時重新申請。
- 啟用後解鎖：從 `Cmd + T` 混裝視窗「單獨抽出單一分頁移至螢幕 2」、「只抽回單一工作區分頁並合併進同一個工作區視窗」、「一鍵依工作區自動分家（1 工作區 = 1 專屬視窗）」；改用事件訂閱（新工作階段、工作階段結束、版面變動）即時更新，輪詢降為每 10 秒一次的保險同步。

## 完成條件（M6 驗收）

- 在「Retina 筆電 + 一般解析度外接螢幕」配置下，並排終端與送到螢幕 2 位置正確、不被 Dock 或選單列遮住。
- 啟用 Python API 後可從混裝視窗單獨抽出一個分頁到螢幕 2，並可一鍵按工作區分家；停用後自動降級為整窗操作。
- 原始碼與設定檔中搜尋不到 Cookie 或 Key 的任何持久化寫入。
- `pnpm verify` 通過。

---

## Commits

### 7-2-01 feat: iTerm2 API 最小協定子集
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/iterm2_api/mod.rs`、`src-tauri/src/iterm2_api/proto.rs`、`src-tauri/Cargo.toml`
- **實作重點**：
  - 依 001 決定（預設：手寫 `#[derive(prost::Message)]` 結構，不放入 `api.proto` 原檔）定義最小訊息集合：`ClientOriginatedMessage`／`ServerOriginatedMessage` 外框與 id、列出工作階段、通知訂閱（新工作階段、工作階段結束、版面變動）、001 確認的分頁移動／重排 RPC。檔案標頭註明協定來源與授權說明。
  - 加入 `prost`（MIT／Apache-2.0）。
- **驗證**：單元測試：編碼後再解碼往返；與 001 記錄的欄位編號一致。

### 7-2-02 feat: 取得 Cookie／Key 並建立 Socket 連線
- **UI**：否
- **預計檔案**（6）：`src-tauri/src/iterm2_api/auth.rs`、`src-tauri/src/iterm2_api/conn.rs`、`src-tauri/src/terminal/scripts/iterm2_request_cookie.applescript`、`src-tauri/src/iterm2_api/mod.rs`、`src-tauri/Cargo.toml`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `auth.rs`：以固定腳本執行 `request cookie and key for app named "Cue"`（App 名稱為腳本內固定字串）；結果包成 `Secret` 型別：**不實作 `Debug`／`Display`／`Serialize`／`Clone`**，以 `zeroize` 於 drop 時清除；osascript 的 stdout 只在此函式內解析，錯誤訊息不得包含原始輸出。
  - `conn.rs`：`tokio::net::UnixStream` 連 iTerm2 socket → `tokio_tungstenite::client_async`（依 001 的請求 URL、子協定與標頭）；請求／回應以 id 配對（`oneshot`），通知經 `broadcast` 分送。
  - 斷線或驗證失敗時丟棄舊 Secret、重新申請後重連（指數退避，上限 30 秒）；使用者關閉開關時中斷並清除。
  - 狀態列舉：`Disabled`｜`NotEnabledInIterm`（socket 不存在）｜`Connecting`｜`Connected`｜`Unauthorized`，emit `iterm2-api-status`。
  - 加入 `tokio-tungstenite`、`zeroize`（皆 MIT／Apache-2.0）。
- **驗證**：單元測試：`Secret` 不可被格式化（以編譯期檢查或測試確認未實作 `Debug`）；本機啟用 Python API 後連線成功；`grep -rniE 'cookie|key' src-tauri/src` 檢查沒有寫檔／log 的路徑。

### 7-2-03 feat: 透過 Socket 列出工作階段與事件訂閱
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/iterm2_api/requests.rs`、`src-tauri/src/iterm2_api/events.rs`、`src-tauri/src/discovery.rs`、`src-tauri/src/config.rs`、`src-tauri/src/terminal/iterm2.rs`
- **實作重點**：
  - `preferences.iterm2SocketEngine`（預設 false）。啟用且連線成功時：訂閱新工作階段、工作階段結束、版面變動通知 → 收到即 `trigger_discovery()`；輪詢間隔改為 10 秒保險同步。斷線時恢復 2 秒輪詢。
  - `ITerm2Backend::capabilities()` 改為動態：連線中 `precise_tabs: true`。
  - 探索資料仍以 AppleScript 腳本為主（欄位一致、已驗證）；Socket 的列出結果用於取得分頁／視窗 id 對應，供 7-2-04 使用。
- **驗證**：本機：啟用後開關分頁，節點即時（< 1 秒）更新；關閉 iTerm2 Python API 後自動回到輪詢。

### 7-2-04 feat: 精準分頁操作
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/iterm2_api/requests.rs`、`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/arrange.rs`、`src-tauri/src/commands.rs`、`src-tauri/src/layout.rs`
- **實作重點**：
  - **單獨抽出分頁到螢幕 2**：把該工作階段所在分頁移到新視窗（001 確認的 RPC），再以 7-1 的 `set bounds` 滿版放到螢幕 2。
  - **只抽回單一工作區分頁並合併**：把該工作區散落在混裝視窗的分頁移入該工作區的專屬視窗（沒有則新建一個），放回螢幕 1。
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
