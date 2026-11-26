# slice-4-1 主題引擎

> 階段：slice-4 外觀、面板與效率（對應規格 M3）
> 上一個子切片：slice-3-5 拖放與派工｜下一個子切片：slice-4-2 主視窗外殼

## 目標

建立主題引擎：完整 CSS Variables 清單、8 套風格主題（數值原樣寫入）、8 套經典配色（統一推導規則）、OFL 字型打包、`previewTheme`／`commitTheme` 即時預覽機制，以及禁止硬編碼樣式的自動檢查與對比度測試。

## 必讀

- 無（規格已全部寫入本文件）

## 完成條件

- `src/themes/builtin.ts` 的 8 套風格主題數值與下方規格表完全一致。
- 對比度自動化測試：所有主題的 `--text-primary`、`--text-muted` 對 `--bg-base`，以及 `--on-accent` 對 `--accent`，皆 ≥ 4.5:1。
- 斷網時所有主題字型正常；`npm run lint:style` 找不到元件中的硬編碼顏色／字型／圓角。

---

## 規格

### CSS Variables 完整清單

每個主題都定義下列變數；所有元件（含 Modal、指令面板、徽章、編輯室卡片與角色）**只讀這些變數**。

- **色彩**：`--bg-base`、`--bg-sidebar`、`--bg-surface`、`--bg-selected`、`--border-subtle`、`--border-strong`、`--text-primary`、`--text-muted`、`--accent`、`--on-accent`、`--focus-ring`
- **狀態**：`--status-running`、`--status-waiting`、`--status-done`、`--status-idle`、`--status-exited`（各狀態除顏色外也要有明度差異，色弱者可辨識）
- **字型**：`--font-heading`、`--font-body`、`--font-mono`、`--heading-weight`、`--title-size`
- **造型**：`--radius-sm`、`--radius-lg`、`--border-width`、`--border-style`、`--shadow-control`（按鈕與輸入框）、`--shadow-panel`、`--texture`（背景紋理，可為 `none`）、`--header-rule`（標題列下方的分隔樣式）
- **狀態燈形狀**：`--status-shape`（`dot` 或 `geometric`；`geometric` 時 running／waiting／done 分別為方形／菱形／圓形，idle 空心圓、exited 虛線空心圓）
- **角色色板**：`--char-ink`（線條）、`--char-fill`（主色塊）、`--char-fill-2`（輔色塊）、`--char-paper`（桌面與紙張）；未指定時依序取 `--text-primary`、`--accent`、`--bg-selected`、`--bg-surface`
- **特色擴充變數**（實作用，用來表達規格表「特色」欄）：`--ws-name-font`、`--ws-name-size`、`--ws-name-weight`（工作區名稱字型）、`--branch-badge-bg`、`--branch-badge-fg`（分支徽章）、`--selected-outline`（選中列內框）、`--header-rule-height`、`--heading-tracking`（標題字距）。
- `--focus-ring` 規格表未列，一律等於 `--accent`。

### 共用字型堆疊

```
SYS_SERIF = ui-serif, "New York", "Songti TC", serif
SYS_SANS  = -apple-system, "SF Pro Text", "PingFang TC", sans-serif
SYS_MONO  = ui-monospace, "SF Mono", Menlo, monospace
```

### 8 套風格主題（原樣寫入 `src/themes/builtin.ts`）

| 項目 | 暖紙雜誌 `paper`（預設） | 深焙暗色 `roast` | 樂譜手稿 `score` | 音樂廳夜場 `concert` |
|---|---|---|---|---|
| 明暗 | 淺 | 深 | 淺 | 深 |
| `--bg-base`／`--bg-sidebar` | `#F5F1E8` | `#1D1B18` | `#F6F6F2` | `#2B0F16` |
| `--bg-surface` | `#EBE4D4` | `#282521` | `#EBEBE5` | `#3A1720` |
| `--bg-selected` | `#E2D6BE` | `#36312A` | `#E1E1DA` | `#4C1D2A` |
| `--border-subtle` | `#D3C7B0` | `#3D3731` | `#CBCBC3` | `#5E2A36` |
| `--border-strong` | `#2B2620` | `#8C8273` | `#1C1C24` | `#C9A24B` |
| `--text-primary` | `#2B2620` | `#EDE5D8` | `#1C1C24` | `#F3E6D3` |
| `--text-muted` | `#685D4E` | `#A99F90` | `#56565F` | `#CDB3A0` |
| `--accent` | `#D97757` | `#E48462` | `#7A1F2B` | `#C9A24B` |
| `--on-accent` | `#2B2620` | `#1D1B18` | `#F6F6F2` | `#2B0F16` |
| running／waiting／done | `#3A6BA5`／`#B4741A`／`#3D7A44` | `#7FA7D9`／`#E8B84A`／`#8DBF7A` | `#2F5D8C`／`#A8730C`／`#2E6B3F` | `#8FB8E8`／`#E8C15A`／`#9CCB8A` |
| idle／exited | `#8A7F70`／`#B3A893` | `#8C8273`／`#5A534A` | `#8E8E96`／`#C2C2BA` | `#A88F86`／`#6E4A52` |
| 標題字型／字重／大小 | SYS_SERIF／600／20px | SYS_SERIF／600／20px | `"Bodoni Moda", "Songti TC", serif`／700／22px | `"Cormorant Garamond", "Songti TC", serif`／700／24px |
| 內文／等寬 | SYS_SANS／SYS_MONO | SYS_SANS／SYS_MONO | SYS_SANS／SYS_MONO | SYS_SANS／SYS_MONO |
| 圓角（sm／lg） | 6／12 | 6／12 | 2／4 | 4／8 |
| 邊框 | 1px solid | 1px solid | 1px solid | 1px solid |
| 標題列分隔 | 3px 雙線（`--border-strong`） | 3px 雙線（`--border-strong`） | 五線譜：5 條 1px 線、間距 4px、透明度 55% | 1px 黃銅細線（`--accent`） |
| 紋理／陰影 | 無 | 無 | 無 | 無 |
| 特色 | 工作區名稱用標題字型 | 同暖紙雜誌 | 工作區名稱用標題字型 | 工作區名稱用標題字型、16px 粗體 |

| 項目 | 藍圖工程 `blueprint` | 包浩斯色塊 `bauhaus` | 青瓷 `celadon` | 新粗野 `brutal` |
|---|---|---|---|---|
| 明暗 | 深 | 淺 | 淺 | 淺 |
| `--bg-base`／`--bg-sidebar` | `#123B66` | `#EAE7E1` | `#E4EBE5` | `#EDE9FF` |
| `--bg-surface` | `rgba(234,242,255,.07)` | `#DDD9D1` | `#D6E0D8` | `#FFFFFF` |
| `--bg-selected` | `rgba(234,242,255,.15)` | `#F2C230` | `#C6D5CA` | `#FFE14D` |
| `--border-subtle` | `rgba(220,235,255,.38)` | `#1A1A1A` | `#B6C6BB` | `#000000` |
| `--border-strong` | `#EAF2FF` | `#1A1A1A` | `#1E2D29` | `#000000` |
| `--text-primary` | `#EAF2FF` | `#1A1A1A` | `#1E2D29` | `#000000` |
| `--text-muted` | `#AFC6E3` | `#45433F` | `#4A5D56` | `#3D3A4A` |
| `--accent` | `#FFC857` | `#D6402B` | `#2F6B5E` | `#FF5C39` |
| `--on-accent` | `#123B66` | `#FFFFFF` | `#F4F8F5` | `#000000` |
| running／waiting／done | `#8FD3FF`／`#FFC857`／`#9BE29B` | `#2450A4`／`#D6402B`／`#1A1A1A` | `#2E5E9E`／`#A0650F`／`#2F6B5E` | `#2F5BFF`／`#FF5C39`／`#0E9A63` |
| idle／exited | `#AFC6E3`／`#6F8FB6` | `#5C5A55`／`#8F8B83` | `#7F918A`／`#A9B8AF` | `#6E6A80`／`#A9A4BD` |
| 標題字型／字重／大小 | `"Barlow Condensed", "PingFang TC", sans-serif`／700／22px（字距 .04em） | `"Jost", Futura, "PingFang TC", sans-serif`／700／21px | `"Manrope", "PingFang TC", sans-serif`／700／19px | `"Bricolage Grotesque", "PingFang TC", sans-serif`／800／22px |
| 內文／等寬 | `"Barlow", "PingFang TC", sans-serif`／`"Barlow Condensed", "PingFang TC", sans-serif` | 同標題字型／同標題字型 | 同標題字型／SYS_MONO | 同標題字型／同標題字型 |
| 圓角（sm／lg） | 2／4 | 0／0 | 10／18 | 8／14 |
| 邊框 | 1px **dashed** | 2px solid | 1px solid | 2px solid |
| 標題列分隔 | 1px 虛線（`--border-subtle`） | 6px 紅／黃／藍三等分色條，上下各 2px 黑線 | 無 | 無 |
| 紋理 | 16px 製圖格線，線色 `rgba(234,242,255,.08)` | 無 | 無 | 無 |
| 陰影 | 無 | 無 | 無 | 按鈕與輸入框 `3px 3px 0 #000`；面板 `6px 6px 0 #000` |
| 狀態燈形狀 | `dot` | **`geometric`** | `dot` | `dot` |
| 特色 | — | 分支徽章反白（黑底淺字）；選中列為黃色色塊 | 最大圓角、無分隔線 | 選中列黃底加 2px 黑色內框 |

- `brutal`、`bauhaus` 為實色風格：主題定義加上 `vibrancy: false`（4-2-06 使用）。
- 包浩斯三色條的紅／黃／藍取該主題的 `#D6402B`／`#F2C230`／`#2450A4`。

### 經典配色 8 套與推導規則

主題：`Catppuccin Mocha`、`Tokyo Night`、`Dracula`、`Nord`、`Gruvbox Dark`、`One Dark`、`Rosé Pine`、`Catppuccin Latte`。只提供各自**官方**的背景、前景與 16 色 ANSI（實作時從各專案官方 repo 取值，並在程式註解附來源網址）。其餘變數一律由推導規則產生（`.itermcolors` 匯入也用這套）：

- `--bg-base`（＝`--bg-sidebar`）＝背景；`--text-primary`＝前景；`--text-muted`＝前景往背景混合 35%
- `--bg-surface`／`--bg-selected`／`--border-subtle`＝背景往前景混合 6%／12%／20%；`--border-strong`＝前景
- `--accent`＝ANSI 洋紅（5）；`--on-accent` 取黑（`#000000`）或白（`#FFFFFF`）中對比較高者
- running＝ANSI 藍（4）、waiting＝ANSI 黃（3）、done＝ANSI 綠（2）、idle＝`--text-muted`、exited＝前景往背景混合 60%
- 造型統一「簡約」版型：SYS_SANS 標題 600／19px、圓角 6／10、1px solid、無分隔線、無紋理、無陰影、`dot` 狀態燈
- 推導後檢查 `--text-primary` 與 `--text-muted` 對背景的對比度至少 4.5:1，不足時自動往亮（深色背景）或暗（淺色背景）逐步調整直到達標
- 明暗判定：背景相對亮度 < 0.5 為深色
- 主題名稱不得使用「Claude」或其他商標名稱（上列經典配色為原專案名稱，可沿用）。

---

## Commits

### 4-1-01 feat: 主題型別與 8 套風格主題
- **UI**：否
- **預計檔案**（3）：`src/themes/types.ts`、`src/themes/builtin.ts`、`src/themes/fontStacks.ts`
- **實作重點**：
  - `types.ts`：`ThemeVars`（上方完整清單的 key 皆為必填，角色色板可省略）、`ThemeDef { id, name, group: 'style' | 'classic' | 'custom', appearance: 'light' | 'dark', vibrancy: boolean, vars }`。
  - `fontStacks.ts`：`SYS_SERIF`、`SYS_SANS`、`SYS_MONO` 常數。
  - `builtin.ts`：8 套風格主題**原樣**寫入規格表數值。標題列分隔以 `--header-rule`（CSS `background` 值）＋`--header-rule-height` 表達：
    - 3px 雙線：`linear-gradient(var(--border-strong) 0 1px, transparent 1px 2px, var(--border-strong) 2px 3px)`、高 3px
    - 五線譜：5 條 1px 線、間距 4px（總高 21px），線色 `--border-strong` 以 55% 透明度（`color-mix(in srgb, var(--border-strong) 55%, transparent)`）
    - 黃銅細線：1px `--accent`
    - 藍圖虛線：1px 虛線 `--border-subtle`（`repeating-linear-gradient` 水平虛線）
    - 包浩斯：上 2px 黑、6px 紅黃藍三等分、下 2px 黑（高 10px）
    - 無：`none`、高 0
  - 藍圖紋理 `--texture`：16px 製圖格線（兩組 `linear-gradient` 橫直線，線色 `rgba(234,242,255,.08)`，`background-size: 16px 16px`）。
  - 「特色」欄轉成特色擴充變數（工作區名稱字型、分支徽章反白、選中列內框）。
- **驗證**：Vitest：每套主題含完整變數 key；抽查數值與規格表一致（每主題至少 bg-base、accent、圓角、狀態燈形狀）。

### 4-1-02 feat: 色彩工具、推導規則與 8 套經典配色
- **UI**：否
- **預計檔案**（5）：`src/themes/color.ts`、`src/themes/derive.ts`、`src/themes/classic.ts`、`src/themes/__tests__/contrast.test.ts`、`src/themes/__tests__/derive.test.ts`
- **實作重點**：
  - `color.ts`：hex／`rgba()` 解析、sRGB 混合（`mix(a, b, t)`）、WCAG 相對亮度、對比度、`adjustForContrast(fg, bg, 4.5)`。
  - `derive.ts`：`deriveTheme({ id, name, background, foreground, ansi[16] }, group)` 依上方推導規則產生 `ThemeDef`。
  - `classic.ts`：8 套官方色值＋來源網址註解，透過 `derive.ts` 產生。
  - `contrast.test.ts`（**規格要求的自動化測試**）：對所有風格主題與經典配色檢查 `--text-primary`、`--text-muted` 對 `--bg-base`、`--on-accent` 對 `--accent` ≥ 4.5:1。半透明的 `--bg-surface` 等不列入。**若規格表中的內建風格主題本身不達標，不要私自修改數值，停下回報給使用者決定。**
- **驗證**：`npm test`。

### 👤 4-1-H1 同意下載 OFL 字型檔
- Claude 會列出要下載的字型檔：Bodoni Moda（600/700）、Cormorant Garamond（600/700）、Barlow（400/500/600）、Barlow Condensed（600/700）、Jost（400/500/700）、Manrope（400/600/700）、Bricolage Grotesque（500/700/800），來源（Google Fonts 官方 GitHub `google/fonts` 的 `ofl/` 目錄或各字型官方 repo）、授權（SIL OFL 1.1）與預估大小，以及轉成 woff2 所需的工具（例如 `fonttools`）。
- 請回覆是否同意下載與安裝工具，或改由你自行提供字型檔放到 `src/assets/fonts/`。

### 4-1-03 feat: 打包 OFL 字型
- **UI**：否
- **預計檔案**（3）：`src/styles/fonts.css`、`src/assets/fonts/LICENSES.md`、`src/main.tsx`（不計入：`src/assets/fonts/*.woff2` 共 19 支）
- **實作重點**：
  - 每個字重一支 woff2（可變字型先以 instancer 固定字重），只保留拉丁字集；中文一律回退系統蘋方-繁／宋體-繁，不打包中文字型。
  - `fonts.css`：`@font-face`（`font-display: swap`，`src: url(...)` 為相對路徑，經 Vite 打包為 `'self'`）。
  - `LICENSES.md`：各字型名稱、著作權聲明、OFL 1.1 全文或連結（README 第三方授權清單在 8-2-01 引用）。
  - **禁止任何遠端字型**。
- **驗證**：`npm run build` 後 `dist/assets` 含 woff2；斷網執行 `npm run tauri dev`，DevTools Network 無對外請求，暫時把標題字型切成 Bodoni Moda 確認顯示正確後還原。

### 4-1-04 feat: 主題服務：套用、預覽與提交
- **UI**：否
- **預計檔案**（7）：`src/themes/engine.ts`、`src/themes/registry.ts`、`src/store/useBaton.ts`、`src-tauri/src/config.rs`、`src-tauri/src/commands.rs`、`src/index.css`、`src-tauri/tauri.conf.json`
- **實作重點**：
  - `registry.ts`：`allThemes()` 依「風格主題／經典配色／我的主題」分組（我的主題在 4-4-02 接上）。
  - `engine.ts`：
    - `applyVars(theme)`：以 `document.documentElement.style.setProperty` 寫入所有變數（CSSOM，不受 CSP 限制）；角色色板未指定時套用預設對應。
    - `previewTheme(id)`：只暫時套用，不寫設定；`cancelPreview()`：還原為目前已套用的主題。
    - `commitTheme(id)`：套用並呼叫 `set_theme_prefs` 持久化，emit `theme-changed` 讓其他視窗（偏好設定視窗）同步。
    - 同步呼叫 `getCurrentWindow().setTheme(appearance)` 讓紅黃綠燈、捲軸、原生選單跟隨主題明暗。
  - **不閃白**：啟動時先以 `get_theme_prefs` 取得主題後再 render；`index.css` 的 `:root` 保留 `paper` 值作為初始值；`tauri.conf.json` 設定視窗 `backgroundColor`（paper 的 `#F5F1E8`）。
  - `config.rs`：`preferences.theme`（目前主題 id，預設 `paper`）、`followSystem`（預設 false）、`lightTheme`（預設 `paper`）、`darkTheme`（預設 `roast`）、`firstRunCardShown`（預設 false）。
  - 切換主題不需重啟、不影響任何工作階段。
- **驗證**：Vitest：preview → cancel 還原、preview → commit 持久化呼叫；本機以 DevTools 呼叫切換所有主題，畫面即時改變。

### 4-1-05 test: 禁止硬編碼樣式的自動檢查
- **UI**：否
- **預計檔案**（3）：`scripts/check-hardcoded-style.mjs`、`package.json`、`scripts/verify.sh`
- **實作重點**：
  - 掃描 `src/**/*.{ts,tsx,css}`，**排除** `src/themes/`、`src/styles/fonts.css`、`src/index.css` 的 `:root` 初始值區塊與測試檔。
  - 規則：不得出現 hex 色碼（`#RGB`／`#RRGGBB`／`#RRGGBBAA`）、`rgb(`／`rgba(`／`hsl(`、具名顏色用於 `color`／`fill`／`stroke`／`background`（允許 `currentColor`、`transparent`、`none`、`inherit`）、`font-family` 字面值、`border-radius` 的數值、Tailwind 任意值顏色類別（例如 `bg-[#fff]`）、Tailwind 預設色票類別（例如 `text-gray-500`、`bg-white`）、`rounded-md` 等非變數圓角類別。
  - 違規時列出檔案與行號，exit 1。
  - `package.json` 加 `"lint:style"`；`verify.sh` 已會自動執行。
  - 若既有元件有違規，在本 Commit 一併修正（計入檔案數；超過上限就拆 `4-1-05a`／`b`）。
- **驗證**：`npm run lint:style` 通過；暫時在元件加 `#fff` 確認會失敗後還原。
