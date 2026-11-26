# slice-1-3 技術決策

> 階段：slice-1 技術驗證與專案骨架（風險先行的收尾）
> 上一個子切片：slice-1-2 技術探針｜下一個子切片：slice-2-1 領域模型與程序解析

## 目標

依探針結果與調查筆記，寫成決策紀錄 `docs/design/001-tech-decisions.md`，並把結論回寫到受影響的後續切片文件，讓後面的 Commit 不必再做選擇。

> 本子切片**例外允許**讀取並修改下列切片文件：`slice-2-1.md`、`slice-2-2.md`、`slice-5-1.md`、`slice-5-2.md`、`slice-7-2.md`（以及調查結果顯示需要調整的其他切片）。

## 必讀

- `docs/design/notes-research.md`
- `docs/design/notes-probe.md`（1-2-H1 的實機輸出）
- `docs/PROMPT.md`「一、系統架構」第 1 點的「授權限制（強制）」

## 完成條件

- `docs/design/001-tech-decisions.md` 涵蓋下方列出的所有決策，每項含：選項、結論、理由、對後續切片的影響。
- 受影響的切片文件已更新（實作重點、預計檔案、依賴版本），移除對應的 ⚠️ 註記或改寫為已定案內容。

---

## Commits

### 👤 1-3-H1 決定 api.proto 與通知外掛的處理方式
- Claude 會先根據 `notes-research.md` 摘要以下需要你決定的事項與建議（若調查已確認無疑義，該項直接採用建議、不需你決定）：
  1. **iTerm2 `api.proto`**：若授權無法確認與 MIT 相容，建議**不放入該檔**，改為在 Rust 中以 `#[derive(prost::Message)]` 手寫 M6 需要的最小訊息結構（只描述協定的欄位編號與型別），README 註明協定來源。請選擇：手寫最小結構／放入原檔（需確認授權）／M6 不做 Socket 引擎。
  2. **通知點擊回報**：若 `tauri-plugin-notification` 尚無版本支援 macOS 桌面點擊回報，請選擇替代方案（自行以 `UNUserNotificationCenter` 實作／點擊只帶起 Baton 並聚焦最新一則／暫不支援點擊聚焦）。
  3. 其他調查中標示「待決定」的項目。
- 完成後告訴 Claude 每一項的選擇。

### 1-3-01 docs: 技術決策紀錄 001 並更新後續切片
- **UI**：否
- **預計檔案**（7）：`docs/design/001-tech-decisions.md`、`docs/design/notes-probe.md`、`docs/slices/slice-2-1.md`、`docs/slices/slice-2-2.md`、`docs/slices/slice-5-2.md`、`docs/slices/slice-7-2.md`、`docs/slices/slice-5-1.md`
- **實作重點**：決策紀錄至少包含：
  1. **cwd 取得方式**：libproc 或 lsof（依耗時、可靠度、授權），以及所用 crate／FFI。→ 更新 `slice-2-1.md` 的 2-1-03。
  2. **tty 名稱正規化規則**：iTerm2、Terminal.app、`ps`、Hook 腳本四方的格式統一為 `ttysNNN`。→ 更新 `slice-2-1.md`、`slice-2-2.md`、`slice-5-1.md`。
  3. **探索腳本欄位定稿**：iTerm2／Terminal.app 腳本的輸出欄位順序與型別、單次執行耗時、輪詢 2 秒的可行性；Terminal.app 的「關閉分頁」能力。→ 更新 `slice-2-2.md`。
  4. **macOS 原生 API 綁定**：objc2 系列 crate 與版本。→ 更新 `slice-2-2.md`（2-2-01）。
  5. **通知外掛版本與點擊回報方案**（鎖定的精確版本號）。→ 更新 `slice-5-2.md`（5-2-02）。
  6. **iTerm2 Socket 協定**：`api.proto` 處理方式、連線標頭、最小訊息清單與欄位編號、「移分頁到新視窗」與「併入視窗」所用的 RPC。→ 更新 `slice-7-2.md`。
  7. **Gemini CLI／Claude Code Hook** 的設定鍵與事件名稱、`Notification` matcher。→ 更新 `slice-5-2.md`（5-2-04）。
  8. **依賴授權清單**（全部為 MIT／Apache-2.0／BSD／OFL 等相容授權）。
  - 若有決策會影響本文件未列出的切片，也一併更新（檔案數上限仍為 8，超過就拆成 `1-3-01a`、`1-3-01b`）。
- **驗證**：每個 ⚠️ 註記都已處理；決策紀錄沒有「待決定」殘留。
