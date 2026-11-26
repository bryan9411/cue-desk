# slice-4-3 偏好設定視窗

> 階段：slice-4 外觀、面板與效率（對應規格 M3）
> 上一個子切片：slice-4-2 主視窗外殼｜下一個子切片：slice-4-4 .itermcolors 匯入

## 目標

建立獨立的偏好設定視窗（一般、終端機、外觀、通知、進階），外觀分頁提供以主題變數即時渲染的縮圖、hover／方向鍵即時預覽、跟隨系統深淺色與透明度滑桿。

## 必讀

- `docs/PROMPT.md`「五、…」第 3 點（主題挑選介面，入口 1）

## 完成條件

- 移到卡片上即時預覽（主面板與偏好設定視窗同時變）、移開或按 Esc 還原、點擊或 Enter／空白鍵才套用。
- 開啟「跟隨系統深淺色」後，在系統設定切換深淺色，Baton 在 1 秒內換成對應主題。
- 偏好設定視窗位置與所有偏好持久化。

---

## Commits

### 4-3-01 feat: 偏好設定獨立視窗骨架
- **UI**：是
- **預計檔案**（7）：`prefs.html`、`src/prefs/main.tsx`、`src/prefs/PrefsApp.tsx`、`vite.config.ts`、`src-tauri/src/windows.rs`、`src-tauri/src/commands.rs`、`src/components/sidebar/BottomBar.tsx`
- **實作重點**：
  - Vite 多頁輸入（`index.html`、`prefs.html`）。
  - `open_prefs(tab?)` command：以 `WebviewWindowBuilder` 建立 label **`prefs`**（capability 已涵蓋）的視窗，約 `760 × 640`、可調整大小、標題「偏好設定」；已開啟則聚焦並切換分頁。位置持久化於 `config.window.prefs`。
  - `PrefsApp`：左側分頁「一般、終端機、外觀、通知、進階」，鍵盤可操作（`role="tablist"`）。啟動時套用目前主題並訂閱 `theme-changed`／預覽事件，使兩個視窗同步。
  - 底部「設定」按鈕、App 選單「偏好設定…」（`Cmd + ,`）、「顯示 › 主題 › 外觀設定…」都開啟此視窗（後者直接切到外觀分頁）。
- **驗證**：三個入口都能開啟；關閉後重開位置保留。

### 4-3-02 feat: 一般、終端機、通知、進階分頁
- **UI**：是
- **預計檔案**（6）：`src/prefs/tabs/GeneralTab.tsx`、`src/prefs/tabs/TerminalTab.tsx`、`src/prefs/tabs/NotificationTab.tsx`、`src/prefs/tabs/AdvancedTab.tsx`、`src/ipc/api.ts`、`src-tauri/src/commands.rs`
- **實作重點**：
  - **一般**：新工作階段預設開啟方式（`defaultOpenMode`；選項「記住上次選擇」＝不設定）、視窗置頂。
  - **終端機**：終端機後端（自動偵測／iTerm2／終端機），切換後立即生效（`set_terminal_backend` 重新選擇後端並重新探索）；未安裝 iTerm2 時該選項停用並說明。iTerm2 Socket 精準分頁開關與開啟引導（「於 iTerm2 勾選 `Settings › General › Magic › Enable Python API`，無須安裝 Python 或開啟輔助使用權限；首次連線 iTerm2 可能詢問一次」）——開關本身在 7-2-05 生效，本 Commit 先顯示為停用並註明「即將推出」。
  - **通知**：音效開關、macOS 通知開關（`preferences.notificationsEnabled`，預設 true；5-2-02 使用）。
  - **進階**：在 Finder 開啟 `~/.baton`、顯示 Hook 腳本路徑 `~/.baton/bin/baton-notify`。
- **驗證**：每個設定變更後重開仍保留，主視窗即時反映。

### 4-3-03 feat: 外觀分頁的主題縮圖與即時預覽
- **UI**：是
- **預計檔案**（5）：`src/prefs/tabs/AppearanceTab.tsx`、`src/components/theme/ThemeThumbnail.tsx`、`src/components/theme/ClassicSwatch.tsx`、`src/components/theme/useThemePreview.ts`、`src/themes/engine.ts`
- **實作重點**：
  - 外觀分頁由上到下：1. 跟隨系統深淺色（4-3-04 加入）；2. **風格主題**（每列 4 張縮圖卡片）；3. **經典配色**（較小色票卡片：背景色塊＋4 個 ANSI 色點＋名稱，每列 4 張）；4. **我的主題**（4-4-02 加入）；5. 背景透明度滑桿（4-3-05 加入）。
  - **縮圖是以該主題變數即時渲染的迷你面板**（不是圖片）：容器以 React `style` 設定該主題的全部 CSS 變數（CSSOM，CSP 允許），內含標題、分隔線、三列含狀態燈的工作階段、一個強調色按鈕；卡片下方顯示主題名稱與「淺色／深色」標籤。
  - **互動**：卡片是真正的 `<button>`，有可見的鍵盤焦點（`--focus-ring`）。`mouseenter`／`focus` → `previewTheme`（卡片顯示「預覽中」），`mouseleave`／`blur`／Esc → `cancelPreview`，點擊／Enter／空白鍵 → `commitTheme`。方向鍵在格狀卡片間移動焦點。目前主題卡片顯示勾選圖示與強調色外框。
  - 預覽跨視窗：engine 的預覽透過事件 `theme-preview`（`{ id | null }`）同步主視窗；關閉偏好設定視窗時自動取消預覽。
- **驗證**：本機逐一 hover、方向鍵、Esc、點擊；兩個視窗同步。

### 4-3-04 feat: 跟隨系統深淺色
- **UI**：是
- **預計檔案**（5）：`src-tauri/src/appearance.rs`、`src-tauri/src/lib.rs`、`src/themes/followSystem.ts`、`src/components/theme/FollowSystemControl.tsx`、`src/prefs/tabs/AppearanceTab.tsx`
- **實作重點**：
  - **注意**：4-1-04 會以 `window.setTheme()` 強制視窗明暗，這會讓 WebView 的 `prefers-color-scheme` 與 Tauri 的 `onThemeChanged` 跟著視窗而非系統變化。因此系統外觀改由 Rust 偵測：監聽 `NSDistributedNotificationCenter` 的 `AppleInterfaceThemeChangedNotification`，讀取 `NSUserDefaults` 的 `AppleInterfaceStyle`（`Dark` 或不存在），emit `system-appearance-changed`（`'light' | 'dark'`）；並提供 `get_system_appearance` command。
  - `FollowSystemControl`：「跟隨系統深淺色」開關（預設關閉，固定 `暖紙雜誌`）；開啟時顯示「淺色時」「深色時」兩個下拉選單，**只列出對應明暗的主題**，預設 `暖紙雜誌`／`深焙暗色`。
  - `followSystem.ts`：開啟時依系統外觀 `commitTheme` 對應主題；關閉時固定使用使用者選定的主題。
- **驗證**：本機在系統設定切換深淺色，≤ 1 秒換成對應主題。

### 4-3-05 feat: 背景透明度滑桿
- **UI**：是
- **預計檔案**（2）：`src/components/theme/OpacitySlider.tsx`、`src/prefs/tabs/AppearanceTab.tsx`
- **實作重點**：
  - `<input type="range">` 70%～100%，即時呼叫 `set_opacity`（節流）。
  - 目前主題為 `brutal` 或 `bauhaus` 時滑桿停用，旁邊說明「此主題為實色風格，已停用毛玻璃與透明度」。
- **驗證**：本機調整與主題切換。
