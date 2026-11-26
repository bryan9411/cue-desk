# slice-6-2 狀態動畫

> 階段：slice-6 雜誌編輯室角色（對應規格 M5）
> 上一個子切片：slice-6-1 角色造型｜下一個子切片：slice-7-1 螢幕座標與整窗調度

> 前提：👤 6-1-H1 已確認角色風格。

## 目標

依 5 種 Hook 狀態完成角色姿勢與微動畫、狀態切換過場、視窗隱藏時暫停、遵守 `prefers-reduced-motion`。完成 M5 驗收。

## 必讀

- `docs/PROMPT.md`「四、…」第 3 點
- 6-1 已完成的 `src/components/newsroom/characters/` 原始碼（了解部件與 class 命名）

## 規格：5 種狀態

- `running`（拼命撰寫中）：身體前傾搖擺、皺眉專注、雙手交替快速寫字／打字、速度線閃動、草稿紙張飛出、桌燈發光與底部進度流光。
- `waiting`（等你拍板）：高舉手臂大幅揮手、歪頭望向你、琥珀色（`--status-waiting`）波紋擴散＋跳動的「!」氣泡（**SVG 繪製，不用 emoji 或文字字元**）。
- `done`（大功告成）：雙手高舉歡呼跳躍、彎月笑眼、星芒與紙花飄升。
- `idle`（喝咖啡小憩）：靠椅悠閒呼吸、定時舉起陶杯啜飲、緩緩飄出熱氣。
- `exited`（已關閉）：空座位與闔上的筆電，點擊可重新開啟。
- 動畫一律以樣式表中的 CSS `@keyframes`（或 SVG 原生動畫）實作；`document.hidden` 時暫停；`prefers-reduced-motion: reduce` 時只保留靜態姿勢。
- 點擊卡片立即聚焦對應 iTerm2 窗格（`done` 自動回 `idle`）；支援拖檔到角色卡片派工（規則同 3-5）。

## 完成條件（M5 驗收）

- 5 種狀態動畫流暢且隨 Hook 即時切換。
- 視窗隱藏時暫停。
- 窄版與寬版切換不遺失狀態。
- 每個主題下角色配色正確。
- `npm run verify` 通過。

---

## Commits

### 6-2-01 feat: running 與 waiting 姿勢與動畫
- **UI**：是
- **預計檔案**（4）：`src/components/newsroom/characters/poses/Running.tsx`、`src/components/newsroom/characters/poses/Waiting.tsx`、`src/styles/newsroom.css`、`src/components/newsroom/characters/Character.tsx`
- **實作重點**：
  - 依上方規格繪製姿勢與附屬元素（速度線、紙張、桌燈光暈、進度流光；波紋、「!」氣泡）。
  - `@keyframes` 只動 `transform` 與 `opacity`（避免 layout）；設 `transform-box: fill-box` 與適當 `transform-origin` 讓部件以關節旋轉。
  - 多張卡片同時 running 時，以 `animation-delay` 依 sessionId 雜湊錯開，避免整齊劃一。
  - 光暈、流光色彩用 `--status-running`、`--accent`、`--char-*`。
- **驗證**：本機以 `baton-notify running|waiting` 觀察；`npm run lint:style`。

### 6-2-02 feat: done、idle 動畫與 exited 空座位
- **UI**：是
- **預計檔案**（5）：`src/components/newsroom/characters/poses/Done.tsx`、`src/components/newsroom/characters/poses/Idle.tsx`、`src/components/newsroom/characters/poses/Exited.tsx`、`src/styles/newsroom.css`、`src/components/newsroom/characters/Character.tsx`
- **實作重點**：
  - `done`：歡呼跳躍、彎月笑眼、星芒與紙花飄升（`--status-done`、`--accent`）。
  - `idle`：在 6-1 靜態造型上加呼吸（軀幹微幅縮放）、每 6～8 秒舉杯啜飲一次、熱氣緩緩上飄淡出。
  - `exited`：空椅子與闔上的筆電，整體降低飽和（以 `opacity` 與 `--status-exited`），無動畫；卡片顯示「點擊重新開啟」。
- **驗證**：本機觀察三種狀態。

### 6-2-03 feat: 狀態過場、隱藏暫停與減少動態
- **UI**：是
- **預計檔案**（4）：`src/hooks/useDocumentVisibility.ts`、`src/styles/newsroom.css`、`src/components/newsroom/characters/Character.tsx`、`src/components/newsroom/DeskCard.tsx`
- **實作重點**：
  - **過場**：狀態改變時舊姿勢與新姿勢交叉淡化（約 250ms，`opacity`＋輕微位移），利用 store 記錄的前一狀態；快速連續變化時只保留最後一個。
  - **隱藏暫停**：`visibilitychange` 時在根元素切換 `is-paused` class，CSS 以 `animation-play-state: paused` 暫停所有角色動畫。
  - **減少動態**：`@media (prefers-reduced-motion: reduce)` 下 `animation: none`、取消過場，只保留各狀態的靜態姿勢。
  - `DeskCard`：點擊 `exited` 卡片呼叫 `reopen_session`，其他狀態呼叫 `focus_session`；保留 `data-drop-target` 與拒絕原因顯示。
  - 窄版 ↔ 寬版切換時狀態不遺失（狀態在 store，切回寬版直接以目前狀態渲染，不播放過場）。
- **驗證**：本機：切到別的 App 讓 Baton 隱藏後回來（動畫有暫停）；系統設定開啟「減少動態」確認只剩靜態姿勢。

### 👤 6-2-H1 M5 實機驗收
- 依本文件「完成條件（M5 驗收）」逐條操作並回報通過／失敗（可搭配真實 Claude Code／Gemini CLI Hook）。
- Claude 把結果記錄在本文件末尾「驗收紀錄」，失敗項目以新增的 `fix` Commit 修正。
