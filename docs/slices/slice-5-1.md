# slice-5-1 Hook 通道

> 階段：slice-5 Hook 與通知（對應規格 M4）
> 上一個子切片：slice-4-5 指令面板與首次啟動｜下一個子切片：slice-5-2 提示與通知

> ⚠️ tty 正規化規則與 Hook stdin 欄位以 `docs/design/001-tech-decisions.md` 為準（slice-1-3 會回寫本文件）。

## 目標

建立 `~/.cue/run/cue.sock` Unix Domain Socket Hook 伺服器與 `~/.cue/bin/cue-notify` 輔助腳本，讓 Claude Code／Gemini CLI 的 Hook 以 0 額外 Token 驅動狀態燈號。

## 必讀

- `docs/PROMPT.md`「五、…」第 4、5 點（狀態型別、Hook 通道）
- `docs/design/001-tech-decisions.md` 第 2、7 節

## 完成條件

- 在 Cue 啟動**前**就開好的 iTerm2 分頁執行 `~/.cue/bin/cue-notify waiting`，對應節點變為 waiting 燈號。
- Cue 未執行時同一指令 1 秒內靜默 `exit 0`、stdout 為空；以 `kill -STOP` 暫停 Cue 時 1 秒內逾時結束、不卡住。

---

## Commits

### 5-1-01 feat: Unix Domain Socket Hook 伺服器
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/hook/mod.rs`、`src-tauri/src/hook/server.rs`、`src-tauri/src/hook/protocol.rs`、`src-tauri/src/registry.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - 路徑 `~/.cue/run/cue.sock`；目錄 `~/.cue/run` 權限 `0700`、socket 檔 `0600`（bind 後立即 `set_permissions`）。啟動時刪除殘留的舊 socket；App 結束（`RunEvent::Exit`）時刪除。
  - `tokio::net::UnixListener`；**不得監聽任何 TCP/UDP Port**。每次連線讀一行 JSON，**上限 4 KB**、讀取逾時 1 秒，超過即丟棄並關閉連線。
  - `protocol.rs`：`HookMessage { tty: String, status: String, cli_session_id: Option<String>, transcript_path: Option<String> }`（JSON 欄位 `tty`、`status`、`cliSessionId`、`transcriptPath`）。`status` 必須是 `idle`｜`running`｜`waiting`｜`done`｜`exited` 之一，否則丟棄；tty 依 001 規則正規化。
  - `registry.set_status_by_tty(tty, status, extra)`：以 tty 對應**目前探索到**的存活工作階段（多個同 tty 不應發生，若有則全部更新）；找不到就丟棄。`cliSessionId`、`transcriptPath` 只儲存在節點上，本版不使用。合法則 emit `session-status-update`（`{ sessionId, status }`）。
  - 狀態規則：`focus_session` 成功時若狀態為 `done` 自動回到 `idle` 並 emit 更新；`exited` 只由探索設定。**嚴禁解析終端機輸出或以閒置計時推測狀態**。
- **驗證**：單元測試：合法訊息、非法 status、超過 4 KB、缺 tty、未知 tty；整合測試：在 tempdir 啟動伺服器，以 `UnixStream` 送訊息收到事件、權限為 0600。

### 5-1-02 feat: cue-notify 輔助腳本
- **UI**：否
- **預計檔案**（4）：`src-tauri/resources/cue-notify.sh`、`src-tauri/src/hook/install_script.rs`、`src-tauri/src/lib.rs`、`scripts/test-cue-notify.sh`
- **實作重點**：
  - 腳本以 `include_str!` 內嵌；App **每次啟動**時寫入（覆蓋）`~/.cue/bin/cue-notify` 並設為 `0755`（目錄 `0700`）。
  - 用法：`cue-notify <status>`。同一支腳本供 Claude Code 與 Gemini CLI 使用。腳本規格（`#!/bin/bash`，相容 macOS 內建 bash 3.2）：
    - 第一行就 `exec 1>/dev/null`——**stdout 完全不輸出任何內容**（Gemini CLI 會把 Hook 的 stdout 當成 JSON 解析）。除錯訊息只寫 stderr，且只在 `CUE_DEBUG=1` 時輸出（預設關閉）。
    - 不使用 `set -e`；任何錯誤都靜默 `exit 0`，**絕不阻擋 CLI**。
    - status 不在五種之內 → `exit 0`。
    - socket（`$HOME/.cue/run/cue.sock`）不存在（Cue 沒在執行）→ 直接 `exit 0`。
    - 以 `ps -o tty= -p $$` 取得自己所在的 tty（Hook 程序繼承 AI CLI 的控制終端）；取不到或為 `??` → `exit 0`。
    - stdin 非終端機時，以 `read -t 1` 讀取 CLI 傳入的 Hook JSON（**讀取也要有逾時**），用 `sed` 擷取 `session_id`、`transcript_path`；值只允許 `[A-Za-z0-9._/~-]` 字元，含其他字元就略過該欄位（避免 JSON 注入）。讀不到就略過。
    - 以 `printf '%s\n' "$json" | /usr/bin/nc -U -w 1 "$sock"` 送出（**必須有 1 秒逾時**：兩個 CLI 都會同步等待 Hook，Cue 卡住時不可連帶卡住 AI）。
  - 因為以 tty 辨識，Cue 啟動前就開好的工作階段也能正常顯示狀態。
  - `scripts/test-cue-notify.sh`：驗證 (1) 無 socket 時 `exit 0`、stdout 為空、耗時 < 1 秒；(2) 非法 status 立即結束；(3) 以 `python3` 或 `nc -lU` 建立不回應的 socket 並讓伺服器 `kill -STOP`，腳本 ≤ 1.5 秒結束（`/usr/bin/time` 量測）；(4) stdin 給 Hook JSON 時正確帶出欄位（以 `nc -lU` 接收比對）。
- **驗證**：`bash scripts/test-cue-notify.sh`；`shellcheck`（若已安裝）無錯誤。

### 5-1-03 feat: 前端狀態燈號連動
- **UI**：是
- **預計檔案**（4）：`src/ipc/events.ts`、`src/store/useCue.ts`、`src/components/sidebar/SessionNode.tsx`、`src/components/newsroom/DeskCard.tsx`
- **實作重點**：
  - 訂閱 `session-status-update`，經 store 統一函式 `applyStatusUpdate(sessionId, status)` 更新（並記錄上一個狀態，供 5-2 的提示音與 slice-6 的過場判斷）。
  - 側邊欄節點與編輯室卡片的燈號即時改變；`waiting` 節點加上低調強調（例如 `--status-waiting` 左側細線）。
  - 點擊節點聚焦後，後端會把 `done` 改回 `idle` 並 emit，前端不自行改狀態。
- **驗證**：Vitest：`applyStatusUpdate` 記錄前一狀態；本機在既有 iTerm2 分頁執行 `~/.cue/bin/cue-notify running|waiting|done`，燈號即時改變，點擊 `done` 節點回到 `idle`。
