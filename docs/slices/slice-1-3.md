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
  2. **通知點擊回報**：若 `tauri-plugin-notification` 尚無版本支援 macOS 桌面點擊回報，請選擇替代方案（自行以 `UNUserNotificationCenter` 實作／點擊只帶起 Cue 並聚焦最新一則／暫不支援點擊聚焦）。
  3. 其他調查中標示「待決定」的項目。
- 完成後告訴 Claude 每一項的選擇。

#### 使用者決定（1-3-01a、1-3-01b 依此寫入 `001-tech-decisions.md` 並回寫切片文件）

1. **iTerm2 `api.proto`：不放入原檔，手寫最小結構。**
   - 以 `#[derive(prost::Message)]` 手寫，只描述協定互通所需的事實資訊（欄位編號與型別，來源為 `notes-research.md` 4.3 節）。
   - **不複製原檔的註解、說明文字或整段內容**；Rust 型別與欄位命名、模組分組、檔案結構以 Cue 自己的方式撰寫，不照抄原版的命名與排列（Protobuf 傳輸只依欄位編號與型別，名稱不影響相容）。
   - **只實作 M6 實際用到的訊息**（收發外框、列出工作階段、三種通知訂閱、移分頁到新視窗、併入視窗）；之後的功能需要時再逐一加入，不預先寫完整協定。
   - 檔頭註明「協定格式參考 iTerm2 Python API 公開文件」，不附 GPL、不調整專案授權。
2. **通知點擊：採用 `tauri-plugin-notification` 的 `on_action`**，鎖定 2.8.1；通知 `extra` 帶工作階段 ID，收到 `tap` 即聚焦該工作階段。打包後在 macOS 26 實機若收不到 `tap`，再改以 `objc2-user-notifications` 自行實作 `UNUserNotificationCenterDelegate`。
3. **Claude Code `waiting` 觸發：即時。** 權限詢問用 `PermissionRequest`（只觀察、不輸出、`exit 0`，不影響權限決定）；另以 `Notification` matcher 精確比對 `elicitation_dialog`、`elicitation_url_dialog`；排除 `idle_prompt`。
   - 使用者要求：`waiting` **只亮燈，不干擾當下操作**——不搶焦點、不把 Cue 或終端機帶到前景、不自動切換工作階段。
   - 進入 `waiting` 時**只亮燈並更新 Dock 徽章**，不播放提示音、不發送 macOS 通知橫幅；`done` 的提示音與通知照原規格保留。1-3-01b 需回寫 `slice-5-2.md`。
4. **`bundle.macOS.minimumSystemVersion`：`"26.0"`**（只支援實測過的 macOS 26）。

### 1-3-01a docs: 加入技術決策紀錄
- **UI**：否
- **預計檔案**（3）：`docs/design/001-tech-decisions.md`、`docs/design/notes-probe.md`、`docs/slices/slice-1-3.md`
- **實作重點**：決策紀錄至少包含下列各節，每節含選項、結論、理由、對後續切片的影響：
  1. **cwd 取得方式**：libproc 或 lsof（依耗時、可靠度、授權），以及所用 crate／FFI。
  2. **tty 名稱正規化規則**：iTerm2、Terminal.app、`ps`、Hook 腳本四方的格式統一為 `ttysNNN`。
  3. **探索腳本欄位定稿**：iTerm2／Terminal.app 腳本的輸出欄位順序與型別、單次執行耗時、輪詢 2 秒的可行性；Terminal.app 的分頁行為與「關閉分頁」能力；前景程式與類型判斷規則。
  4. **macOS 原生 API 綁定**：objc2 系列 crate 與版本。
  5. **通知外掛版本與點擊回報方案**（鎖定的精確版本號），以及各狀態的提示方式。
  6. **iTerm2 Socket 協定**：`api.proto` 處理方式、連線標頭、最小訊息清單與欄位編號、「移分頁到新視窗」與「併入視窗」所用的 RPC。
  7. **Gemini CLI／Claude Code Hook** 的設定鍵與事件名稱、`Notification` matcher。
  8. **依賴授權清單**（全部為 MIT／Apache-2.0／BSD／OFL 等相容授權）。
  9. **最低 macOS 版本**。
  - 一併提交 1-2-H1 的實機紀錄 `notes-probe.md`。
- **驗證**：決策紀錄沒有「待決定」殘留；涵蓋上方全部章節；每節都寫明影響的切片。

### 1-3-01b docs: 依技術決策更新實作說明
- **UI**：否
- **預計檔案**（8）：`docs/slices/slice-2-1.md`、`docs/slices/slice-2-2.md`、`docs/slices/slice-2-3.md`、`docs/slices/slice-3-2.md`、`docs/slices/slice-5-1.md`、`docs/slices/slice-5-2.md`、`docs/slices/slice-7-2.md`、`docs/slices/slice-8-1.md`
- **實作重點**：依 `001-tech-decisions.md` 回寫：
  - `slice-2-1.md`：cwd（2-1-03）、tty 正規化、前景程式取群組領頭與包裝程式判斷（2-1-02）。
  - `slice-2-2.md`：探索腳本欄位與批次取值、Terminal.app 分頁行為、objc2 版本（2-2-01）。
  - `slice-2-3.md`：Terminal.app 的位置提示。
  - `slice-3-2.md`：Terminal.app 關閉分頁的方式。
  - `slice-5-1.md`：tty 正規化、`cue-notify` 的 stderr 靜默。
  - `slice-5-2.md`：通知外掛版本（5-2-02）、`waiting` 只亮燈與 Dock 徽章（5-2-01、5-2-02）、Hook 設定（5-2-04）。
  - `slice-7-2.md`：手寫最小協定子集、連線標頭、RPC。
  - `slice-8-1.md`：最低 macOS 版本。
- **驗證**：上列切片的 ⚠️ 註記都已處理；`grep -n '⚠️' docs/slices/*.md` 不再出現待 slice-1-3 定案的註記。
