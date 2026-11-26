# slice-3-3 選單與建立 Modal

> 階段：slice-3 建立、管理、防呆與拖檔派工（對應規格 M2）
> 上一個子切片：slice-3-2 工作階段管理｜下一個子切片：slice-3-4 Git 分支監聽

## 目標

前端接上 slice-3-1、3-2 的所有操作：工作區／工作階段／未歸類的右鍵選單、行內重新命名、建立工作階段 Modal，以及同目錄多 AI 防呆提醒與警示徽章。

## 必讀

- `docs/PROMPT.md`「四、GUI 佈局…」第 2 點（右鍵選單內容）
- `docs/PROMPT.md`「三、…」第 1 點與第 3 點（Modal 與防呆提醒）

## 完成條件

- 所有右鍵選單項目可用；後端不支援的操作依 `capabilities` 停用並顯示原因。
- 同目錄開啟 2 個 AI 時顯示 SVG 警示徽章（例如「2 AI」）；在已有活躍 AI 的工作區開 Modal 並選 AI 類型時顯示防呆提醒。

---

## Commits

### 3-3-01 feat: 右鍵選單與節點操作
- **UI**：是
- **預計檔案**（7）：`src/components/ContextMenu.tsx`、`src/components/sidebar/WorkspaceNode.tsx`、`src/components/sidebar/SessionNode.tsx`、`src/components/sidebar/UnclassifiedSection.tsx`、`src/components/sidebar/RenameField.tsx`、`src/ipc/api.ts`、`src/store/useBaton.ts`
- **實作重點**：
  - `ContextMenu`：自繪浮動選單（滑鼠位置開啟、方向鍵／Enter／Esc 操作、點外部關閉、`role="menu"`），停用項目以 `aria-disabled` 並顯示原因（例如「終端機後備模式不支援分割」）。只用主題變數。
  - **工作區選單**：新增工作階段（開 3-3-02 的 Modal）、在 Finder 開啟（`opener.revealItemInDir`）、在 iTerm2 開一個 zsh（`create_session` kind=zsh、mode=預設）、移除工作區（以 `plugin-dialog` 的 `ask` 確認，文案註明「不會刪除檔案」）。「只收回此工作區至螢幕 1」於 7-1-05 加入。
  - **工作階段選單**：聚焦、重新命名、在右側補開同工作區 Zsh、在下方補開同工作區 Zsh、關閉、重新開啟（僅 `exited`）、移除（僅 `exited`）。「送到螢幕 2 對照／收回螢幕 1」於 7-1-05 加入。
  - **不提供**「移到其他工作區」或任何手動歸屬項目。
  - `RenameField`：行內輸入框，Enter 確認、Esc 取消、失焦確認。
  - **未歸類區**：每列加「＋ 設為新工作區」與「忽略」兩個按鈕（只有這兩個操作）。
  - `api.ts` 補上 3-1／3-2 的所有 command 包裝。
- **驗證**：`npm run build`；本機逐項操作選單；切到 Terminal.app 後備模式確認停用項目與說明。

### 3-3-02 feat: 建立工作階段 Modal 與多 AI 防呆
- **UI**：是
- **預計檔案**（6）：`src/components/CreateSessionModal.tsx`、`src/components/WarningBadge.tsx`、`src/lib/workspaceStats.ts`、`src/lib/__tests__/workspaceStats.test.ts`、`src/components/sidebar/WorkspaceNode.tsx`、`src/App.tsx`
- **實作重點**：
  - `workspaceStats.ts`：`activeAiCount(workspaceId, sessions)`＝該工作區內非 `exited`、類型為 `claude` 或 `gemini` 的節點數（不含 `zsh` 與其他程式）。
  - `WarningBadge`：細線 SVG 警示三角形＋數字（「2 AI」），`activeAiCount >= 2` 時顯示在工作區名稱旁，色彩用 `--status-waiting`，`title` 說明原因。**不使用 emoji**。
  - **Modal**（原生 `<dialog>`，`showModal()`；Esc 關閉、焦點鎖定、開啟時聚焦第一個欄位）：
    - 欄位：工作區（下拉，預設為觸發的工作區）、類型（`claude`／`gemini`／`zsh`）、自訂標籤（例如「權限架構」、「Q3 財報精讀」）、開啟方式（新視窗／新分頁／AI 上・Zsh 下／AI 左・Zsh 右）。
    - 開啟方式預設：`defaultOpenMode` → `lastOpenMode` → 新視窗。
    - Terminal.app 後備模式：只能選「新視窗」，其他選項停用並說明「終端機（Terminal.app）不支援分割，也無法不透過模擬按鍵開新分頁」。
    - `zsh` 類型時「AI 上／Zsh 下」「AI 左／Zsh 右」停用。
    - **防呆提醒**：所選工作區 `activeAiCount >= 1` 且類型為 AI 時，顯示溫和提醒：「這個資料夾已有 AI 正在工作。同一資料夾只有單一分支，多個 AI 同時修改檔案可能互相覆蓋，建議先分工（例如各自負責不同檔案）。」
  - 建立失敗時在 Modal 內顯示錯誤（例如路徑含控制字元被拒絕）。
- **驗證**：Vitest 測 `activeAiCount`；本機：同工作區開兩個 claude 出現「2 AI」徽章；再開 Modal 選 claude 出現提醒。
