# slice-6-1 角色造型

> 階段：slice-6 雜誌編輯室角色（對應規格 M5）
> 上一個子切片：slice-5-2 提示與通知｜下一個子切片：slice-6-2 狀態動畫

## 目標

先交付**一個角色的 idle 靜態造型**供使用者確認風格；同意後才在 slice-6-2 製作其餘狀態與動畫。

## 必讀

- `docs/PROMPT.md`「四、…」第 3 點（寬版雜誌編輯室活體視覺模式）

## 規格摘要

- 每個工作階段是一張工作桌卡片，內含**手繪墨線＋柔和色塊的非像素純向量 SVG 角色**：零外部圖檔、全部內嵌 SVG。
- 色板只吃主題 CSS Variables：`--char-ink`（線條）、`--char-fill`（主色塊）、`--char-fill-2`（輔色塊）、`--char-paper`（桌面與紙張）；未指定時依序取 `--text-primary`、`--accent`、`--bg-selected`、`--bg-surface`。
- idle（喝咖啡小憩）姿勢：靠椅悠閒、手持陶杯、杯口熱氣（本子切片只做靜態，不動）。

## 完成條件

- 寬版編輯室的卡片顯示 idle 角色，切換 16 套主題配色都協調、線條清楚。
- 使用者確認風格（👤 6-1-H1）。

---

## Commits

### 6-1-01 feat: 角色 SVG 基礎與 idle 靜態造型
- **UI**：是
- **預計檔案**（5）：`src/components/newsroom/characters/Character.tsx`、`src/components/newsroom/characters/parts.tsx`、`src/components/newsroom/characters/poses/Idle.tsx`、`src/styles/newsroom.css`、`src/components/newsroom/DeskCard.tsx`
- **實作重點**：
  - 角色變數的回退已由主題引擎處理（4-1-04 的 `applyVars` 在主題未指定時把 `--char-*` 設為對應變數），角色元件直接使用 `var(--char-*)`。
  - `newsroom.css`：角色相關 class（線條 `stroke-linecap: round`、`stroke-linejoin: round`、線寬 1.75～2.25 的輕微變化營造手繪感）。
  - `parts.tsx`：可重用的部件（頭、臉部表情、軀幹、手臂、桌子、椅子、筆電、陶杯、紙張），每個部件以 `<g>` 分組並預留 `className` 供 6-2 動畫掛載（例如 `char-arm-left`、`char-head`、`char-steam`）。路徑以略帶不規則的曲線繪製，柔和色塊稍微錯位於墨線（手繪套色感）。
  - `Character.tsx`：`<Character status=... />`，viewBox 固定（例如 `0 0 160 120`），`role="img"` 加 `aria-label`；本 Commit 只實作 `idle`，其他狀態暫時也顯示 idle。
  - `DeskCard` 放入角色（卡片上半部），下方保留標籤、類型、燈號、位置提示。
  - **不得**出現任何硬編碼顏色（`npm run lint:style` 必須通過）；不使用點陣圖、像素風或 emoji。
- **驗證**：`npm run verify`；本機寬版下切換 `paper`、`roast`、`blueprint`、`brutal`、`Catppuccin Latte` 等主題截圖給使用者。

### 👤 6-1-H1 確認角色風格
- Claude 會提供 idle 角色在數個主題下的截圖。請確認風格（線條、比例、表情、色塊感），或提出修改意見。
- 修改意見由 Claude 以 `6-1-01` 之後新增的 `fix`／`style` Commit 處理，直到你同意。
- **你同意後才會開始 slice-6-2**。
