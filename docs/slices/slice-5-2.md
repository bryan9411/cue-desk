# slice-5-2 提示與通知

> 階段：slice-5 Hook 與通知（對應規格 M4）
> 上一個子切片：slice-5-1 Hook 通道｜下一個子切片：slice-6-1 角色造型

> ⚠️ 通知外掛版本與點擊回報方案、Claude Code／Gemini CLI 的 Hook 設定鍵與事件名稱，會在 slice-1-3 依調查結果更新本文件。

## 目標

狀態變為 `waiting`／`done` 時播放提示音、在 Baton 不在前景時發送 macOS 原生通知（點擊即聚焦該工作階段）、Dock 圖示顯示 `waiting` 數量；並提供 Hook 設定範例。完成 M4 驗收。

## 必讀

- `docs/PROMPT.md`「五、…」第 6 點（提示）
- `docs/design/001-tech-decisions.md` 第 5、7 節

## 完成條件（M4 驗收）

- 在 Baton 啟動**前**就開好的 iTerm2 分頁中執行 `~/.baton/bin/baton-notify waiting`，對應節點變為 waiting 燈號並響提示音。
- Baton 在背景時跳出原生通知，點擊後 iTerm2 切到該窗格。
- Baton 未執行時執行同一指令會在 1 秒內靜默結束（`exit 0`），且 stdout 為空。
- 以 `kill -STOP` 暫停 Baton 時執行同一指令，1 秒內逾時結束、不卡住。
- 實際在 Claude Code 與 Gemini CLI 設定 Hook 後，送出提示、等待授權、完成三種狀態都正確顯示。
- `npm run verify` 通過。

---

## Commits

### 5-2-01 feat: Web Audio 雙頻提示音
- **UI**：否
- **預計檔案**（3）：`src/audio/chime.ts`、`src/store/useBaton.ts`、`src/App.tsx`
- **實作重點**：
  - `chime.ts`：單一 `AudioContext`；使用者第一次互動（`pointerdown`／`keydown`）後 `resume()`。雙頻提示音：兩個 `OscillatorNode`（例如 `waiting` 為 660 Hz → 880 Hz 上行、`done` 為 880 Hz＋1320 Hz 和音），各約 120ms，以 `GainNode` 做短 attack／release 避免爆音，音量適中。
  - 狀態**轉變為** `waiting` 或 `done` 時（前一狀態不同）且音效總開關開啟才播放；同一瞬間多個節點只響一次（節流 300ms）。
- **驗證**：本機以 `baton-notify` 觸發聽到提示音；靜音時不響。

### 5-2-02 feat: macOS 原生通知與點擊聚焦
- **UI**：否
- **預計檔案**（6）：`src-tauri/Cargo.toml`、`package.json`、`src-tauri/src/notify.rs`、`src-tauri/capabilities/default.json`、`src-tauri/src/lib.rs`、`src-tauri/src/registry.rs`
- **實作重點**：
  - 加入 `tauri-plugin-notification`，**Rust 與 npm 兩端都鎖定 001 決策中確認支援 macOS 桌面點擊回報的精確版本**（`=x.y.z`）；capability 只加通知所需的最小權限。
  - 狀態轉為 `waiting` 或 `done`、通知開關開啟、且主視窗**不在前景**（`is_focused()` 為 false）時發送：標題＝工作區名稱（未歸類時為「未歸類」）、內容＝「<標籤或名稱>：等你拍板」／「<標籤或名稱>：大功告成」；附帶 sessionId 作為識別。
  - 點擊通知（`Notification::on_action` 或 001 決定的替代方案）→ 呼叫與 `focus_session` 相同的聚焦流程（含 `done` → `idle`）。
  - 首次發送前請求通知授權；被拒時不重複詢問。
- **驗證**：本機讓 Baton 退到背景後觸發 `waiting`，出現通知，點擊後 iTerm2 切到該窗格。

### 5-2-03 feat: Dock 待決數量徽章
- **UI**：否
- **預計檔案**（2）：`src-tauri/src/registry.rs`、`src-tauri/src/hook/server.rs`
- **實作重點**：
  - 任何狀態變動後計算 `waiting` 節點數，以主視窗 `set_badge_count(Some(n))`（0 時 `None`）更新 Dock 徽章（若 001 結論顯示 Tauri API 不支援，改以 objc2 `NSApp.dockTile.setBadgeLabel`，檔案改為新增 `src-tauri/src/dock.rs`）。
- **驗證**：本機兩個分頁 `waiting` 時 Dock 顯示 2；聚焦並送 `running` 後數字遞減。

### 5-2-04 docs: Claude Code 與 Gemini CLI Hook 設定範例
- **UI**：否
- **預計檔案**（3）：`README.md`、`docs/hooks/claude-settings.example.json`、`docs/hooks/gemini-settings.example.json`
- **實作重點**：
  - **只提供範例，不自動修改使用者的設定檔**。
  - **Claude Code**（`~/.claude/settings.json`）：`UserPromptSubmit` → `~/.baton/bin/baton-notify running`、`Notification` → `waiting`（依 001 結論設定 matcher，避免閒置提醒誤判）、`Stop` → `done`。
  - **Gemini CLI**（`~/.gemini/settings.json`）：依 001 查證的官方 Hook 文件寫出啟用 Hook 所需的設定鍵；`BeforeAgent` → `running`、`Notification` → `waiting`、`AfterAgent` → `done`。
  - README 新增「Hook 設定」段落（錨點固定，例如 `#hook-設定`，供 `install.sh` 印出網址），說明合併到既有設定的方式與「為什麼 0 額外 Token」。README 其他段落在 8-2-01 完成。
- **驗證**：兩個範例 JSON 可通過 `python3 -m json.tool`；實際設定由 👤 5-2-H1 驗證。

### 👤 5-2-H1 M4 實機驗收
- 依 `docs/hooks/` 範例在你自己的 `~/.claude/settings.json` 與 `~/.gemini/settings.json` 加入 Hook（Claude 不會修改你的設定檔）。
- 依本文件「完成條件（M4 驗收）」逐條操作並回報通過／失敗。`kill -STOP` 測試：`kill -STOP $(pgrep -x baton)`（或 dev 模式的程序名稱），執行 `time ~/.baton/bin/baton-notify waiting`，完成後 `kill -CONT`。
- Claude 把結果記錄在本文件末尾「驗收紀錄」，失敗項目以新增的 `fix` Commit 修正。
