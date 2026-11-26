# slice-5-2 提示與通知

> 階段：slice-5 Hook 與通知（對應規格 M4）
> 上一個子切片：slice-5-1 Hook 通道｜下一個子切片：slice-6-1 角色造型

## 目標

依 001 第 5 節的提示方式：狀態變為 `done` 時播放提示音、在 Cue 不在前景時發送 macOS 原生通知（點擊即聚焦該工作階段）；`waiting` **只亮燈並計入 Dock 徽章**，不播放提示音、不發送通知。任何狀態都不搶焦點。並提供 Hook 設定範例，完成 M4 驗收。

| 狀態轉變 | 燈號 | Dock 徽章 | 提示音 | macOS 通知 |
|---|---|---|---|---|
| → `waiting` | 亮 | 計入數量 | 不播放 | 不發送 |
| → `done` | 亮 | — | 播放（依音效開關） | Cue 不在前景時發送（依通知開關） |

## 必讀

- `docs/PROMPT.md`「五、…」第 6 點（提示）
- `docs/design/001-tech-decisions.md` 第 5、7 節

## 完成條件（M4 驗收）

- 在 Cue 啟動**前**就開好的 iTerm2 分頁中執行 `~/.cue/bin/cue-notify waiting`，對應節點變為 waiting 燈號、Dock 徽章加 1，**不響提示音、不跳通知、不搶焦點**。
- 同一分頁執行 `~/.cue/bin/cue-notify done`：響提示音；Cue 在背景時跳出原生通知，點擊後 iTerm2 切到該窗格（以 `pnpm tauri build` 打包後的 App 測試）。
- Cue 未執行時執行同一指令會在 1 秒內靜默結束（`exit 0`），且 stdout、stderr 皆為空。
- 以 `kill -STOP` 暫停 Cue 時執行同一指令，1 秒內逾時結束、不卡住。
- 實際在 Claude Code 與 Gemini CLI 設定 Hook 後，送出提示、等待授權（含權限詢問與需要輸入的對話框）、完成三種狀態都正確顯示；Claude 的權限詢問出現時燈號即時變為 `waiting`。
- `pnpm verify` 通過。

---

## Commits

### 5-2-01 feat: Web Audio 雙頻提示音
- **UI**：否
- **預計檔案**（3）：`src/audio/chime.ts`、`src/store/useCue.ts`、`src/App.tsx`
- **實作重點**：
  - `chime.ts`：單一 `AudioContext`；使用者第一次互動（`pointerdown`／`keydown`）後 `resume()`。`done` 提示音為雙頻和音：兩個 `OscillatorNode`（例如 880 Hz＋1320 Hz），約 120ms，以 `GainNode` 做短 attack／release 避免爆音，音量適中。
  - 只在狀態**轉變為** `done` 時（前一狀態不同）且音效總開關開啟才播放；同一瞬間多個節點只響一次（節流 300ms）。**`waiting` 不播放**（使用者決定：`waiting` 只亮燈，不干擾當下操作）。
- **驗證**：本機以 `cue-notify done` 觸發聽到提示音；`cue-notify waiting` 不響；靜音時不響。

### 5-2-02 feat: macOS 原生通知與點擊聚焦
- **UI**：否
- **預計檔案**（4）：`src-tauri/Cargo.toml`、`src-tauri/src/notify.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/registry.rs`
- **實作重點**（001 第 5 節）：
  - 加入 `tauri-plugin-notification = "=2.8.1"`（Apache-2.0 OR MIT）。通知只由 Rust 端發送；前端不呼叫通知 API，因此**不加** `@tauri-apps/plugin-notification` 與前端 capability（最小權限），`package.json` 與 `capabilities/default.json` 不修改。日後前端若需要套件，同樣鎖定 `=2.8.1`，capability 只加所需的最小權限。
  - 只有狀態轉為 **`done`**、通知開關開啟、且主視窗**不在前景**（`is_focused()` 為 false）時發送：標題＝工作區名稱（未歸類時為「未歸類」）、內容＝「<標籤或名稱>：大功告成」；`extra` 帶 sessionId。**`waiting` 不發送通知**。
  - 點擊：在 Rust 端註冊 `Notification::on_action`（必須註冊，外掛才會追蹤點擊）；收到 `actionId == "tap"` 時，以 `extra` 的 sessionId 走與 `focus_session` 相同的聚焦流程（含 `done` → `idle`）。
  - 外掛無法撤回已顯示的通知（macOS 的 `close` 為空實作），Cue 不依賴撤回。
  - 首次發送前請求通知授權；被拒時不重複詢問。
  - **不搶焦點**：發送通知時不把 Cue 或終端機帶到前景，也不自動切換工作階段；只有使用者點擊通知才聚焦。
  - 開發模式（`tauri::is_dev()`）下通知以 Terminal.app 的身分送出，點擊會帶起 Terminal.app；**點擊行為只能用 `pnpm tauri build` 打包後的 App 測試**。
  - **備案**：打包後在 macOS 26 收不到 `tap` 時，改以 `objc2-user-notifications` 0.3.2（Zlib OR Apache-2.0 OR MIT）自行實作 `UNUserNotificationCenterDelegate`，以新增的 `fix` Commit 處理。
- **驗證**：本機以打包後的 App 退到背景後觸發 `done`，出現通知，點擊後 iTerm2 切到該窗格；觸發 `waiting` 不出現通知。

### 5-2-03 feat: Dock 待決數量徽章
- **UI**：否
- **預計檔案**（2）：`src-tauri/src/registry.rs`、`src-tauri/src/hook/server.rs`
- **實作重點**：
  - 任何狀態變動後計算 `waiting` 節點數，以主視窗 `Window::set_badge_count(Some(n))`（0 時 `None`）更新 Dock 徽章。Tauri 在 macOS 已支援（設定 `NSApp.dockTile` 的 badge）；只從 Rust 端呼叫，不需要新增前端 capability。
  - 更新徽章時不改變視窗焦點、不彈跳 Dock 圖示（不呼叫 `request_user_attention`）。
- **驗證**：本機兩個分頁 `waiting` 時 Dock 顯示 2；聚焦並送 `running` 後數字遞減；歸零時徽章消失。

### 5-2-04 docs: Claude Code 與 Gemini CLI Hook 設定範例
- **UI**：否
- **預計檔案**（3）：`README.md`、`docs/hooks/claude-settings.example.json`、`docs/hooks/gemini-settings.example.json`
- **實作重點**（001 第 7 節）：
  - **只提供範例，不自動修改使用者的設定檔**。
  - **Claude Code**（`~/.claude/settings.json`，使用者層），`timeout` 單位為**秒**，設 `5`：

    | 事件 | matcher | 指令 |
    |---|---|---|
    | `UserPromptSubmit` | （無） | `~/.cue/bin/cue-notify running` |
    | `PermissionRequest` | 省略（所有工具） | `~/.cue/bin/cue-notify waiting` |
    | `Notification` | `elicitation_dialog\|elicitation_url_dialog` | `~/.cue/bin/cue-notify waiting` |
    | `Stop` | （無） | `~/.cue/bin/cue-notify done` |
    | `StopFailure` | （無） | `~/.cue/bin/cue-notify idle` |

    - `PermissionRequest` 讓 `waiting` 即時亮起；`cue-notify` 不輸出任何內容並 `exit 0`，只是觀察，不影響權限決定。
    - `Notification` 只精確比對需要使用者輸入的對話框，**不列 `idle_prompt`**（閒置提醒）與 `permission_prompt`（與 `PermissionRequest` 重複）。
  - **Gemini CLI**（`~/.gemini/settings.json`，使用者層），`timeout` 單位為**毫秒**，設 `5000`；每個 handler 需有 `name: "cue-notify"`、`type: "command"`；matcher 皆為 `""`：`BeforeAgent` → `running`、`Notification` → `waiting`、`AfterAgent` → `done`。啟用鍵 `hooksConfig.enabled` 預設為 `true`，只要寫入 `hooks` 即生效，範例不需另外寫。
  - README 新增「Hook 設定」段落（錨點固定，例如 `#hook-設定`，供 `install.sh` 印出網址），說明：合併到既有設定的方式、「為什麼 0 額外 Token」、兩者 `timeout` 單位不同、已知限制（使用者按 Esc 中斷時 `running` 可能停留；另有自動核准的 `PermissionRequest` Hook 時可能短暫亮 `waiting`）。README 其他段落在 8-2-01 完成。
- **驗證**：兩個範例 JSON 可通過 `python3 -m json.tool`；實際設定由 👤 5-2-H1 驗證。

### 👤 5-2-H1 M4 實機驗收
- 依 `docs/hooks/` 範例在你自己的 `~/.claude/settings.json` 與 `~/.gemini/settings.json` 加入 Hook（Claude 不會修改你的設定檔）。
- 依本文件「完成條件（M4 驗收）」逐條操作並回報通過／失敗。`kill -STOP` 測試：`kill -STOP $(pgrep -x cue)`（或 dev 模式的程序名稱），執行 `time ~/.cue/bin/cue-notify waiting`，完成後 `kill -CONT`。
- 一併確認 001 標示的待實機確認項目：
  1. 打包後的 App 在 macOS 26 點擊通知時收得到 `tap`（收不到則改用 5-2-02 的備案）。
  2. Claude Code 與 Gemini CLI 啟動的 Hook 程序是否繼承控制終端（燈號有亮即代表 `cue-notify` 取得了 tty）。
- Claude 把結果記錄在本文件末尾「驗收紀錄」，失敗項目以新增的 `fix` Commit 修正。
