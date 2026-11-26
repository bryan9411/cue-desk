# slice-4-2 主視窗外殼

> 階段：slice-4 外觀、面板與效率（對應規格 M3）
> 上一個子切片：slice-4-1 主題引擎｜下一個子切片：slice-4-3 偏好設定視窗

## 目標

完成主視窗的原生外殼：Overlay 標題列與 Cue 標誌、完整細線圖示集、geometric 狀態燈、窄版 ↔ 寬版雙型態（編輯室舞台先放靜態卡片）、底部控制列、視窗置頂、原生選單、毛玻璃與透明度、自動化權限被拒橫幅。

## 必讀

- `docs/PROMPT.md`「四、GUI 佈局…」第 1、2、5 點
- `docs/PROMPT.md`「五、…」第 2 點最後兩項（切換行為、毛玻璃）

## 完成條件

- 標題列保留紅黃綠燈空間、可拖曳；標題字型與分隔線隨主題變化。
- 一鍵（或拖拉寬度）切換窄版／寬版，位置與大小分別持久化。
- 原生選單「顯示 › 主題」可切換主題且目前主題打勾；`Cmd + C/V/X/A/Z` 在所有輸入框可用；`Cmd + Q` 直接結束、不關閉任何終端機工作階段。
- 拒絕自動化權限時出現橫幅，按鈕開啟「隱私權與安全性 › 自動化」。

---

## Commits

### 4-2-01 feat: Overlay 標題列、標誌與細線圖示集
- **UI**：是
- **預計檔案**（4）：`src-tauri/tauri.conf.json`、`src/components/shell/TitleBar.tsx`、`src/components/icons/Icons.tsx`、`src/App.tsx`
- **實作重點**：
  - 主視窗：`titleBarStyle: "Overlay"`、`hiddenTitle: true`；`app.macOSPrivateApi: true`、主視窗 `transparent: true`（4-2-06 毛玻璃需要）。
  - `TitleBar`：左側預留紅黃綠燈寬度（約 72px），整列 `data-tauri-drag-region`；極簡斜線指揮棒 SVG 標誌＋「Cue」字樣（`--font-heading`、`--heading-weight`、`--title-size`、`--heading-tracking`）；下方以 `--header-rule`／`--header-rule-height` 繪製分隔；背景套 `--texture`。
  - `Icons.tsx` 補齊：`Warning`、`Branch`、`Sound`、`SoundOff`、`Settings`、`Pin`、`Columns`（並排終端）、`Tabs`（分頁整理）、`Expand`／`Collapse`（寬窄切換）、`Search`、`Check`、`Close`、`Monitor`、`Finder`。全部細線、`currentColor`、**不使用 emoji**。
- **驗證**：`npm run build`；本機切換數個主題觀察標題列。

### 4-2-02 feat: 狀態燈支援 geometric 形狀
- **UI**：是
- **預計檔案**（2）：`src/components/StatusLight.tsx`、`src/hooks/useCssVar.ts`
- **實作重點**：
  - `useCssVar('--status-shape')` 讀取目前主題值（主題變更時更新）。
  - `dot`：實心圓（idle 空心、exited 虛線空心）。
  - `geometric`：running 方形、waiting 菱形、done 圓形、idle 空心圓、exited 虛線空心圓。
  - 以 SVG 繪製，`fill`／`stroke` 用 `var(--status-*)`；加 `aria-label`（「執行中」「等你拍板」「完成」「閒置」「已關閉」）。
- **驗證**：切到 `bauhaus` 看到幾何形狀，其他主題為圓點。

### 4-2-03 feat: 窄版與寬版雙型態與視窗狀態持久化
- **UI**：是
- **預計檔案**（7）：`src/components/shell/Layout.tsx`、`src/components/newsroom/Stage.tsx`、`src/components/newsroom/DeskCard.tsx`、`src-tauri/src/window_state.rs`、`src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`、`src/store/useCue.ts`
- **實作重點**：
  - **窄版側邊停靠**（預設約 `320 × 850`）：只有側邊欄。**寬版「雜誌編輯室」**（約 `860 × 620`）：左側工作區目錄欄＋右側舞台。
  - 一鍵切換（標題列按鈕與 `toggle_layout` command）；使用者拖拉寬度時響應式切換（寬度 ≥ 640 CSS px 為寬版）。
  - `window_state.rs`：監聽 `Moved`／`Resized`（debounce 500ms），依目前型態分別保存位置與大小到 `config.window.narrow`／`config.window.wide`；啟動時還原，若保存位置已不在任何螢幕內則置中。切換型態時套用該型態的保存值或預設尺寸。
  - `Stage`：依工作區分組排列 `DeskCard`；`DeskCard`（靜態）：標籤／名稱、類型、`StatusLight`、位置提示、卡片樣式全用主題變數（`--bg-surface`、`--radius-lg`、`--border-*`、`--shadow-panel`）；點擊聚焦（`done` 由後端聚焦時自動回 `idle`，slice-5 起生效）；加 `data-drop-target="session:<id>"` 支援拖檔派工。角色插畫留給 slice-6。
  - 兩種型態共用同一份 store 與操作邏輯，切換時不遺失任何狀態。
- **驗證**：切換、拖拉、重開 Cue 後兩種型態各自的位置大小都還原。

### 4-2-04 feat: 底部控制列、視窗置頂與音效開關
- **UI**：是
- **預計檔案**（5）：`src/components/sidebar/BottomBar.tsx`、`src/components/sidebar/Sidebar.tsx`、`src-tauri/src/commands.rs`、`src-tauri/src/config.rs`、`src/store/useCue.ts`
- **實作重點**：
  - 上排兩顆文字按鈕：「並排終端」「分頁整理」（功能於 7-1 接上；本 Commit 先停用並以 `title` 說明「將於雙螢幕功能完成後提供」）。
  - 下排左側：終端機狀態點＋文字（`iTerm2`／`終端機`），置頂啟用時附低調「置頂」標記；右側兩顆線條圖示鈕：「音效開關」（`Sound`／`SoundOff`，`aria-pressed`）與「設定」（開偏好設定視窗，4-3-01 接上）。
  - `set_always_on_top(bool)`、`set_sound_enabled(bool)` command，持久化於 `preferences.alwaysOnTop`（預設 false）、`preferences.soundEnabled`（預設 true）；啟動時套用置頂。
- **驗證**：本機切換置頂與音效，重開後保留。

### 4-2-05 feat: 原生選單（App、Edit、顯示 › 主題）
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/menu.rs`、`src-tauri/src/lib.rs`、`src-tauri/src/commands.rs`、`src/ipc/events.ts`、`src/App.tsx`
- **實作重點**：
  - **App 選單**：關於 Cue、偏好設定…（`Cmd + ,`）、隱藏、結束 Cue（`Cmd + Q`，直接結束不確認，**不關閉任何終端機工作階段**）。
  - **Edit 選單**：以 `PredefinedMenuItem` 建立 Undo／Redo／Cut／Copy／Paste／Select All，確保所有輸入框與 Modal 的快捷鍵正常。
  - **顯示選單**：切換寬／窄版、「主題」子選單——依「風格主題／經典配色／我的主題」分組，以分隔線隔開，`CheckMenuItem` 標示目前主題；最後一項「外觀設定…」。
  - 選單點擊 → emit `menu-action`（`{ action, themeId? }`）給前端，前端呼叫 `commitTheme` 等；主題清單或目前主題變更時 `rebuild_menu`（前端在 commit 後呼叫）。
  - 「偏好設定…」「外觀設定…」在 4-3-01 接上開啟偏好設定視窗（本 Commit 先 emit 事件）。
- **驗證**：本機：選單切換主題、勾選正確；在輸入框測試複製貼上與 Undo；`Cmd + Q` 後 iTerm2 工作階段仍在。

### 4-2-06 feat: 毛玻璃與背景透明度
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/vibrancy.rs`、`src-tauri/Cargo.toml`、`src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`、`src/themes/engine.ts`
- **實作重點**：
  - `window-vibrancy`：`apply_vibrancy(window, NSVisualEffectMaterial::Sidebar, …)`，需 `macOSPrivateApi: true` 與透明視窗（4-2-01 已設）。
  - 背景透明度：`preferences.opacity`（0.7～1.0，**最低不透明度下限 0.7**，預設 1.0）；engine 以 `color-mix(in srgb, var(--bg-base) <opacity>%, transparent)` 產生實際背景變數 `--bg-base-effective`、`--bg-sidebar-effective`，外殼背景使用它們。
  - 主題 `vibrancy: false`（`brutal`、`bauhaus`）時：清除毛玻璃、強制不透明（1.0），`set_vibrancy` 回傳停用原因（滑桿旁說明於 4-3-05 顯示）。
  - `set_opacity(value)` command 夾在範圍內並持久化。
- **驗證**：本機調整透明度看到桌面模糊；切到 `brutal` 立即變實色。

### 4-2-07 feat: 自動化權限被拒橫幅
- **UI**：是
- **預計檔案**（5）：`src/components/shell/PermissionBanner.tsx`、`src-tauri/src/discovery.rs`、`src-tauri/src/commands.rs`、`src/store/useCue.ts`、`src/App.tsx`
- **實作重點**：
  - 任何 osascript 呼叫回傳 `AutomationDenied` 時，後端設定旗標並 emit `automation-denied`；之後成功一次即清除（emit `automation-granted`）。
  - 面板頂部橫幅：細線 `Warning` 圖示＋「Cue 無法控制 iTerm2／終端機。請在系統設定允許自動化權限。」＋「開啟系統設定」按鈕。
  - `open_automation_settings` command：由 Rust 以 `open` 執行固定網址 `x-apple.systempreferences:com.apple.preference.security?Privacy_Automation`（opener 外掛 scope 只允許 http／https，因此不走 opener）。
- **驗證**：在系統設定撤銷 Cue 的 iTerm2 自動化權限（或 `tccutil reset AppleEvents <bundle id>` 後拒絕），橫幅出現且按鈕開啟正確頁面。
