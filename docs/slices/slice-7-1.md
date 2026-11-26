# slice-7-1 螢幕座標與整窗調度

> 階段：slice-7 雙螢幕與 iTerm2 Socket 精準分頁（對應規格 M6）
> 上一個子切片：slice-6-2 狀態動畫｜下一個子切片：slice-7-2 iTerm2 Socket 引擎

## 目標

正確換算多螢幕座標，完成「並排終端」「送到螢幕 2 對照／收回螢幕 1」「只收回此工作區回螢幕 1」與「分頁整理」的整窗（AppleScript `set bounds`）版本，並接上前端入口。

## 必讀

- `docs/PROMPT.md`「三、…」第 6 點（聚焦、雙螢幕調度與關閉）

## 規格：螢幕座標換算（強制）

- Tauri 回報的螢幕位置與尺寸為**實體像素**；AppleScript 的 `bounds` 使用**點（points）**，原點為**主螢幕左上角**、y 向下。
- 由 Rust 以 `NSScreen` 取得每個螢幕的 `frame` 與 `visibleFrame`（點座標；`visibleFrame` 避開 Dock 與選單列）。Cocoa 座標原點在主螢幕**左下角**、y 向上，需轉換：`as_top = primary.frame.height - (rect.y + rect.height)`，`as_left = rect.x`。
- 若使用 Tauri 的視窗／螢幕資訊，必須先除以各自的 `scaleFactor` 再轉換。
- 驗收需涵蓋「Retina 筆電 + 一般解析度外接螢幕」的混合配置。

## 完成條件

- 在混合配置下，並排終端與送到螢幕 2 的位置正確、不被 Dock 或選單列遮住。
- 未啟用 Socket 引擎時只能移動整個視窗；該視窗還有其他分頁時提示「將連同同視窗的其他分頁一起移動」。

---

## Commits

### 7-1-01 feat: 螢幕資訊與座標換算
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/screen.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`、`src-tauri/src/commands.rs`
- **實作重點**：
  - `screens() -> Vec<ScreenInfo { index, is_primary, frame_as, visible_as, scale }>`（皆為 AppleScript 座標系的點）；主螢幕 index 0（＝「螢幕 1」），其他依 x 座標排序。NSScreen 須在主執行緒呼叫（以 `run_on_main_thread` 取得）。
  - 純函式 `cocoa_to_applescript(rect, primary_height)`、`tauri_physical_to_points(pos, size, scale)`，有單元測試。
  - `cue_window_rect_as()`：取 Cue 主視窗的外框（實體像素 ÷ scaleFactor → 點），判斷所在螢幕。
  - 監聽螢幕變化（`NSApplicationDidChangeScreenParametersNotification` 或每次調度時重新查詢），emit `screens-changed`（螢幕數量）。
  - `get_screens` command（前端用來決定是否顯示雙螢幕選項）。
- **驗證**：單元測試：Retina 主螢幕（1512×982 點、scale 2）＋外接 1920×1080（scale 1，位於右側／上方／左側）三種排列的轉換結果；本機 `#[ignore]` 測試印出實際螢幕資訊。

### 7-1-02 feat: 並排終端
- **UI**：否
- **預計檔案**（6）：`src-tauri/src/arrange.rs`、`src-tauri/src/terminal/scripts/iterm2_set_bounds.applescript`、`src-tauri/src/terminal/scripts/terminal_app_set_bounds.applescript`、`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/commands.rs`
- **實作重點**：
  - `arrange.rs` 純函式 `beside_rect(cue_window, visible) -> Bounds`：left＝Cue 右緣、top＝Cue 上緣、right＝該螢幕 `visibleFrame` 右緣、bottom＝Cue 下緣（與 Cue 同高），並夾限在 `visibleFrame` 內。
  - set bounds 腳本：argv 為視窗 ID（空字串＝最前視窗）與 `left top right bottom` 四個數字字串（腳本內轉為 integer）；`{left, top, right, bottom}` 為 AppleScript 座標。
  - `arrange_beside` 實作：iTerm2／Terminal.app 將**最前面的視窗**移到 Cue 右側。
  - `arrange_beside_terminal` command。
- **驗證**：單元測試（夾限、Cue 在外接螢幕）；本機在兩個螢幕上各測一次。

### 7-1-03 feat: 送到螢幕 2 與收回螢幕 1
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/arrange.rs`、`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/commands.rs`
- **實作重點**：
  - 「送到螢幕 2 對照」：把該工作階段所在視窗**滿版**放到副螢幕（第一個非主螢幕）的 `visibleFrame`；其餘視窗保留原位、不移動。
  - 「收回螢幕 1」：把該視窗放回主螢幕的並排位置（同 `beside_rect`；Cue 不在主螢幕時用主螢幕 `visibleFrame` 右側 2/3）。
  - 僅一個螢幕時回傳錯誤「未偵測到第二個螢幕」。
  - 回傳 `{ movedWithOtherTabs: bool }`：該視窗還有其他分頁（依 registry 判斷）時為 true，前端提示「將連同同視窗的其他分頁一起移動」（未啟用 Socket 引擎時）。
  - `send_to_screen2(sessionId)`、`return_to_screen1(sessionId)` commands；完成後聚焦該工作階段。
- **驗證**：單元測試（目標矩形計算）；本機雙螢幕操作。

### 7-1-04 feat: 工作區收回與分頁整理（整窗模式）
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/arrange.rs`、`src-tauri/src/terminal/iterm2.rs`、`src-tauri/src/terminal/terminal_app.rs`、`src-tauri/src/commands.rs`、`src-tauri/src/layout.rs`
- **實作重點**：
  - `gather_windows(window_ids, target)`：以 `set bounds` 將多個視窗**整齊收攏疊齊**在螢幕 1 右側（以 `beside_rect` 為基準，每個視窗向右下錯位 24 點，總錯位不超出 `visibleFrame`）。
  - 「只收回此工作區回螢幕 1」：取該工作區的專屬視窗（3-1-02 `dedicated_window` 的所有符合者）收攏；混裝視窗不動並回傳提示。
  - 「分頁整理」（整窗降級版）：依工作區把各自的專屬視窗依序收攏疊齊；Socket 引擎啟用時改走 7-2-04 的精準版本（本 Commit 先只做降級版）。
  - `recall_workspace(workspaceId)`、`tidy_tabs()` commands。
- **驗證**：單元測試（錯位計算、夾限）；本機多視窗操作。

### 7-1-05 feat: 雙螢幕與整理操作的前端入口
- **UI**：是
- **預計檔案**（8）：`src/components/sidebar/BottomBar.tsx`、`src/components/sidebar/SessionNode.tsx`、`src/components/sidebar/WorkspaceNode.tsx`、`src/components/newsroom/DeskCard.tsx`、`src/commands/registry.ts`、`src/ipc/api.ts`、`src/ipc/events.ts`、`src/store/useCue.ts`
- **實作重點**：
  - 底部「並排終端」「分頁整理」啟用。
  - 工作階段右鍵選單加「送到螢幕 2 對照」「收回螢幕 1」（僅偵測到多螢幕時顯示）；節點與卡片懸停時顯示細線 `Monitor` 快捷鈕。若 store 判斷該視窗還有其他分頁，執行前以 `ask` 確認：「將連同同視窗的其他分頁一起移動，要繼續嗎？」。
  - 工作區右鍵選單加「只收回此工作區回螢幕 1」。
  - `Cmd + P` 的並排終端、分頁整理、送往螢幕 2／收回螢幕 1 啟用（依螢幕數量與目前選取決定可用性）。
  - store 記錄 `screenCount`（`get_screens` 與 `screens-changed`）。
- **驗證**：`npm run build`；本機單螢幕時選項隱藏、雙螢幕時可用。
