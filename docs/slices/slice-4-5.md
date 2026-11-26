# slice-4-5 指令面板與首次啟動

> 階段：slice-4 外觀、面板與效率（對應規格 M3）
> 上一個子切片：slice-4-4 .itermcolors 匯入｜下一個子切片：slice-5-1 Hook 通道

## 目標

完成 `Cmd + P` 快速指令面板（模糊搜尋、方向鍵操作、主題即時預覽）與首次啟動「挑選你的外觀」卡片。完成 M3 驗收。

## 必讀

- `docs/PROMPT.md`「四、…」第 4 點（快速指令面板）
- `docs/PROMPT.md`「五、…」第 3 點的「入口 2」「入口 4」

## 完成條件（M3 驗收）

- 8 套風格主題的顏色、字型、圓角、邊框、分隔線、紋理、陰影、狀態燈形狀與規格表一致；每個主題切換後所有元件（含 Modal、指令面板、徽章、偏好設定視窗）都正確換色、換字型與造型；`pnpm lint:style` 找不到元件中的硬編碼顏色。
- 所有主題的 `--text-primary`、`--text-muted` 對背景，以及 `--on-accent` 對 `--accent`，對比度皆 ≥ 4.5:1（自動化測試）。
- 在偏好設定移到卡片上即時預覽、移開或按 Esc 還原、點擊才套用；`Cmd + P` 與選單列「顯示 › 主題」都能切換；首次啟動卡片只出現一次。
- 開啟「跟隨系統深淺色」後，在系統設定切換深淺色，Cue 在 1 秒內換成對應主題。
- 斷網狀態下所有主題字型都正常顯示，開發者工具中看不到任何對外的字型請求。
- 匯入 Display P3 `.itermcolors` 顏色正確且文字對比度 ≥ 4.5:1。
- 拒絕自動化權限時出現橫幅且按鈕能開啟正確系統設定頁。
- `pnpm verify` 通過。

---

## Commits

### 4-5-01 feat: 模糊搜尋與指令註冊表
- **UI**：否
- **預計檔案**（4）：`src/lib/fuzzy.ts`、`src/lib/__tests__/fuzzy.test.ts`、`src/commands/registry.ts`、`src/commands/__tests__/registry.test.ts`
- **實作重點**：
  - `fuzzy.ts`：子序列比對＋分數（連續、字首、單字開頭加分），支援中文（逐字元比對）；回傳命中位置供高亮。
  - `registry.ts`：`buildCommands(state) -> Command[]`，每個指令 `{ id, group, title, keywords, run(), preview?(), disabledReason? }`：
    - 跳轉聚焦任意工作階段或工作區（工作區＝聚焦其最近使用的工作階段）
    - 新增工作區（資料夾對話框）、建立工作階段（可選版面：新視窗／新分頁／AI 上・Zsh 下／AI 左・Zsh 右，依 capabilities 停用）
    - 並排終端、分頁整理、送往螢幕 2／收回螢幕 1（7-1-05 接上前顯示停用原因）
    - 切換寬／窄版、開啟設定、在 Finder 開啟目前工作區、切換靜音
    - 主題：輸入「主題」或主題名稱即列出所有主題（依風格主題／經典配色／我的主題分組），`preview` 呼叫 `previewTheme`
- **驗證**：Vitest：模糊排序、中文比對、主題指令分組、停用原因。

### 4-5-02 feat: Cmd+P 指令面板
- **UI**：是
- **預計檔案**（4）：`src/components/CommandPalette.tsx`、`src/hooks/useHotkeys.ts`、`src/App.tsx`、`src-tauri/src/menu.rs`
- **實作重點**：
  - 浮動面板（`<dialog>`）：搜尋框＋結果清單（分組標題、命中字元高亮——以多個 `<span>` 純文字渲染，**不用 innerHTML**）。
  - `Cmd + P` 開啟（並加入「顯示」選單項目以顯示快捷鍵）；上下方向鍵移動、Enter 執行、Esc 關閉。
  - 主題項目：方向鍵移動到時即時 `previewTheme`；Enter `commitTheme`；Esc 或關閉時 `cancelPreview` 還原。
  - 停用項目顯示原因，Enter 不執行。
  - 樣式全用主題變數（含 `--shadow-panel`、`--radius-lg`）。
- **驗證**：本機逐項執行指令；主題預覽／還原。

### 4-5-03 feat: 首次啟動外觀卡片
- **UI**：是
- **預計檔案**（3）：`src/components/FirstRunCard.tsx`、`src/App.tsx`、`src/ipc/api.ts`
- **實作重點**：
  - `firstRunCardShown` 為 false 時，在面板上顯示一次「挑選你的外觀」卡片：列出 8 套風格主題縮圖（重用 `ThemeThumbnail`，hover 即時預覽），點選即 `commitTheme`；「之後再說」略過（使用預設 `暖紙雜誌`）。
  - 任何一種關閉方式都設定 `firstRunCardShown = true`，之後不再出現。
  - 窄版時縮圖改為兩欄。
- **驗證**：刪除 `~/.cue/config.json` 的 `firstRunCardShown` 後啟動，卡片出現；選擇或略過後重開不再出現。

### 👤 4-5-H1 M3 實機驗收
- 依本文件「完成條件（M3 驗收）」逐條操作並回報通過／失敗。斷網測試請關閉 Wi-Fi 後執行 `pnpm tauri dev`，並在 DevTools Network 分頁確認。
- Claude 把結果記錄在本文件末尾「驗收紀錄」，失敗項目以新增的 `fix` Commit 修正。
