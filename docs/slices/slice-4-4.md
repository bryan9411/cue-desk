# slice-4-4 .itermcolors 匯入

> 階段：slice-4 外觀、面板與效率（對應規格 M3）
> 上一個子切片：slice-4-3 偏好設定視窗｜下一個子切片：slice-4-5 指令面板與首次啟動

## 目標

匯入 iTerm2 `.itermcolors`，以與經典配色相同的推導規則產生「我的主題」，存於 `~/.cue/themes/`，可重新命名與刪除。

## 必讀

- 無（規格已寫入本文件）

## 規格

- 以 Rust `plist` crate 解析；**檔案上限 1 MB**；色值分量需為 0～1 的數字，否則拒絕並說明。
- 需要的鍵：`Background Color`、`Foreground Color`、`Ansi 0 Color`～`Ansi 15 Color`；每個顏色含 `Red Component`、`Green Component`、`Blue Component`，可能有 `Alpha Component` 與 `Color Space`。
- 處理 `Color Space` 鍵：`sRGB` 直接使用；非 sRGB（含 `P3`＝Display P3、`Calibrated`＝Generic RGB，以及缺鍵時 iTerm2 預設的 Calibrated）一律轉換為 sRGB。
- 推導規則同 slice-4-1（前端 `deriveTheme`），推導後文字對比度 ≥ 4.5:1。
- 主題名稱預設取檔名（去副檔名），可重新命名、可刪除（刪除前確認）。

## 完成條件

- 匯入 Display P3 `.itermcolors` 顏色正確且文字對比度 ≥ 4.5:1。
- 匯入的主題出現在外觀分頁「我的主題」、`Cmd + P` 與選單「顯示 › 主題」中。

---

## Commits

### 4-4-01 feat: 解析 .itermcolors 並轉換色彩空間
- **UI**：否
- **預計檔案**（6）：`src-tauri/src/theme_import.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`、`src-tauri/tests/fixtures/srgb.itermcolors`、`src-tauri/tests/fixtures/p3.itermcolors`、`src-tauri/tests/theme_import.rs`
- **實作重點**：
  - `parse_itermcolors(path) -> Result<Palette { background, foreground, ansi: [Hex; 16] }>`：先檢查檔案大小 ≤ 1 MB，再以 `plist::Value::from_file` 解析；缺鍵、型別錯誤、分量不在 0～1 時回傳清楚的錯誤。
  - 色彩空間轉換：優先使用 macOS 原生 `NSColor`（`colorWithDisplayP3Red…`／`colorWithCalibratedRed…`／`colorWithSRGBRed…` → `colorUsingColorSpace(NSColorSpace.sRGBColorSpace)`）；並提供 P3 → sRGB 的純數學版本（線性化 → 3×3 矩陣 → 夾限 → gamma 編碼）供單元測試與對照。
  - Alpha 忽略（主題色一律不透明）。
- **驗證**：`cargo test --test theme_import`：sRGB 檔值不變；P3 純紅 `(1,0,0)` 轉換後落在 sRGB 範圍邊界（R=1、G≈0、B≈0 夾限後）；超過 1 MB、分量 1.5、缺 `Ansi 5 Color` 皆拒絕。

### 4-4-02 feat: 我的主題存取與管理
- **UI**：是
- **預計檔案**（7）：`src-tauri/src/custom_themes.rs`、`src-tauri/src/commands.rs`、`src/themes/registry.ts`、`src/prefs/tabs/AppearanceTab.tsx`、`src/components/theme/CustomThemeCard.tsx`、`src/ipc/api.ts`、`src-tauri/src/menu.rs`
- **實作重點**：
  - `~/.cue/themes/<id>.json`：`{ id, name, palette }`（id 以 UUID 或時間戳產生，不用使用者輸入）；目錄權限 `0700`；寫入原子化。
  - Commands：`import_itermcolors(path)`（前端以 `plugin-dialog` `open({ filters: [{ extensions: ['itermcolors'] }] })` 取得路徑）、`list_custom_themes`、`rename_custom_theme(id, name)`（長度上限、拒絕控制字元）、`delete_custom_theme(id)`（若為目前主題，先改回 `paper`）。
  - 前端 `registry.ts` 載入後以 `deriveTheme(palette, 'custom')` 產生主題，與其他主題同樣支援預覽與提交。
  - 外觀分頁「我的主題」：匯入的主題卡片（同 4-3-03 互動，另有重新命名與刪除按鈕；刪除前以 `ask` 確認），以及「匯入 .itermcolors」按鈕。名稱一律純文字渲染。
  - 清單變更後重建原生選單（`menu.rs` 的我的主題群組）。
- **驗證**：本機匯入 fixture 的 P3 檔，卡片出現、可預覽套用、重新命名、刪除；選單同步。
