# Cue

Cue 是 macOS 原生的多工作區 AI CLI 指揮中心：Tauri v2（Rust）+ React + TypeScript + Tailwind CSS。本身不內嵌終端機，負責管理本機工作區、認領與調度 iTerm2（後備 Terminal.app）中的 `claude`／`gemini`／`zsh` 工作階段，並以 Unix Domain Socket Hook 驅動狀態燈號與通知。授權 MIT。

## 文件索引

| 文件 | 用途 |
|---|---|
| `docs/STATUS.md` | 所有切片與 Commit 的勾選清單（**不要整份讀**，用下方 grep 找下一項） |
| `docs/slices/slice-X-Y.md` | 每個子切片的實作說明（一次只讀當前那一份） |
| `docs/PROMPT.md` | 原始完整規格。**參考用，平常不要整份讀；只在切片文件指名章節時讀該章節** |
| `docs/design/` | 技術決策紀錄與調查筆記（`001-*.md` 起編號） |

設計稿：無。

---

## 「請繼續實作」流程

使用者說「請繼續實作」時，依序執行：

1. 用 `grep -n -m1 -E '^  - \[ \]' docs/STATUS.md` 找第一個未勾選的項目，**不要整份讀 STATUS.md**。從編號推得切片文件：`X-Y-NN` → `docs/slices/slice-X-Y.md`；`X-0-NN` → `docs/slices/slice-X.md`；人工項目 `X-Y-H1` 同樣對應 `slice-X-Y.md`。
2. 如果是 👤 人工項目：**不要實作**。說明需要使用者做什麼、完成後要提供什麼，然後停下。使用者回報完成後，把他提供的資訊記到切片文件指定的位置（例如 `docs/design/`），再勾選該項。
3. 只讀那一支切片文件與它「必讀」列出的資料，**不讀其他切片文件**。需要了解既有程式碼時，**先用 CodeGraph**（見下方「程式碼查詢」），不要直接 grep／Read。
4. 只實作這一個 Commit。檔案數超過上限（8 支）時先停下，提出拆成 `a`、`b`、`c` 的方案（例如 `2-3-01a`、`2-3-01b`），使用者同意後更新 STATUS.md 與切片文件再繼續。切片文件標示「**UI：是**」時，先啟用 frontend-design skill。
5. 執行切片文件的「驗證」，以及專案的檢查：`pnpm verify`（1-1-04 完成後可用；之前改為分別執行 `cargo check`、`cargo clippy`、`pnpm build`）。
6. 在 STATUS.md 勾選這個 Commit；子切片的最後一個 Commit 也勾選子切片那一行。只讀該行附近（步驟 1 的行號搭配 offset／limit）再修改，和程式碼放在同一個 commit。
7. 停下回報：
   - 變更檔案，分「計入」「不計入」兩組
   - 驗證結果（無法自動驗證、需要實機操作的項目要明確說明）
   - 建議的 commit 訊息

   等使用者同意才 commit，**不要 push**。

## 程式碼查詢：優先使用 CodeGraph

本專案已建 CodeGraph 索引（`.codegraph/`）。查詢、理解或修改**原始碼**時一律先用它：

- **第一步**：呼叫 MCP 工具 `codegraph_explore`（若是延遲載入，先用 ToolSearch `select:mcp__codegraph__codegraph_explore` 載入）。查詢可以是自然語言問題，或一串符號／檔名（例如 `TerminalBackend iterm2.rs capabilities`）。
- **修改前**：把要改的檔案或符號名稱放進查詢，取得行號原始碼與影響範圍後直接 Edit，不需要再 Read 同一段。
- **不要**在 CodeGraph 之前先 grep／find／Read 原始碼，也不要用 grep 重新驗證它的結果。
- MCP 工具不可用時改用 shell：`codegraph explore "<符號或問題>"`。
- 只有 CodeGraph 沒涵蓋的內容才直接讀：`docs/`、設定檔（`*.json`、`*.toml`、`*.plist`）、AppleScript 腳本，或剛寫入、索引尚未更新（約 1 秒延遲）的檔案。

## 節省讀取量

- `docs/PROMPT.md` 的章節是 `###` 標題：先用 `grep -n '^### ' docs/PROMPT.md` 找章節行號，再用 offset／limit 只讀需要的段落。
- 原始碼用 CodeGraph 取得需要的符號，不要整份讀大型檔案。
- 不要為了「了解全貌」去讀其他切片文件或整份 STATUS.md。

## 檔案數計算

- **計入**：原始碼、測試、設定檔、AppleScript 腳本、文件。
- **不計入**（但要列在回報中）：`pnpm-lock.yaml`、`Cargo.lock`、`docs/STATUS.md`、自動產生的檔案（字型 woff2、建置產物、測試快照、`create-tauri-app` 產生且未修改的檔案、圖示）。
- 目標每個 Commit 3 支到 8 支；少於 3 支沒關係，不要為了湊數加檔案。

## Commit 規範

- 主旨與內文使用**台灣繁體中文**；前綴使用英文 Conventional Commits（`feat`、`fix`、`chore`、`docs`、`test`、`refactor`、`ci`、`spike`）。
- **只描述這次實際做了什麼**。主旨與內文都**不得出現任何開發計畫用語**：
  - 不寫切片或 Commit 編號（例如 `slice-2-1`、`2-1-02`、`M1`、`里程碑`）。
  - 不寫進度描述（例如「完成 slice 前半段」、「切片第一部分」、「階段 2 之一」、「接續上一個 Commit」）。
  - 只做了某階段的一部分時，照樣只寫做了哪些功能或變更，不說明它在計畫中的位置。
- **不加任何 AI 署名**（不加 `Co-Authored-By`、「Generated with」等字樣）。
- 一個 Commit 只做一件事。
- 編號只用在 `docs/STATUS.md` 與切片文件中，不進入 git 歷史。

範例：

```
feat: 解析 ps 輸出並判斷前景程式與類型

- 以 pgid == tpgid 找出各 tty 的前景程式群組
- 辨識原生與 npm（node …/claude）兩種 claude 執行形式
- 加入 ps 範例輸出的單元測試
```

錯誤範例（不要這樣寫）：`feat(slice-2-1): …`、內文出現「切片：2-1-02」、「完成 slice-3 前半段」。

---

## 產品規則（每個 Commit 都要遵守）

**授權與依賴**
- 專案為 MIT。**禁止引入 GPL／AGPL 等與 MIT 不相容的依賴**（例如 `iterm2-client` crate 是 GPL-2.0-only，不可用）。新增依賴前確認授權。
- iTerm2 的 `api.proto` 處理方式依 `docs/design/001-tech-decisions.md` 的決定，不得自行更改。

**終端機操作（防注入）**
- 所有 AppleScript 都是程式內固定的腳本原文（放在 `src-tauri/src/terminal/scripts/*.applescript`，以 `include_str!` 載入），以 `osascript` 執行；**變動資料一律透過 `on run argv` 參數傳入，嚴禁拼接進腳本原文**。
- 腳本回傳結構化資料時，欄位以 `\x1f`（Unit Separator）、紀錄以 `\x1e`（Record Separator）分隔。
- 送進終端機的路徑一律在 Rust 端做 shell 單引號跳脫（`'` → `'\''`），含換行或其他控制字元時拒絕。
- 指令名稱只能來自程式內固定列舉（`claude`、`gemini`）。**嚴禁加入跳過權限確認的參數**（`--dangerously-skip-permissions`、`--yolo` 等）。
- 拖檔派工**絕不送出 Enter**。
- 探索（discover）只在終端機已執行時進行，**不得為了輪詢而把 iTerm2 開起來**；只有使用者主動建立或聚焦時才啟動。
- Cue 結束時**不關閉任何終端機工作階段**。

**狀態與歸屬**
- 工作階段歸屬**只依前景程式 `cwd`**（realpath 後最長前綴），不提供任何手動歸屬（拖曳、選單「移到其他工作區」都不做）。
- 狀態只有 `idle`｜`running`｜`waiting`｜`done`｜`exited`。`running`／`waiting`／`done` **只由 Hook 驅動**；**嚴禁解析終端機輸出或以閒置計時推測 AI 狀態**。聚焦時 `done` 回到 `idle`。

**資安**
- **零網路暴露**：不得啟動任何 HTTP／WebSocket 伺服器或監聽 TCP/UDP Port。Hook 只走 `~/.cue/run/cue.sock`（Cue 為伺服器端）；iTerm2 進階連線只以用戶端身分連 iTerm2 的本機 socket。
- iTerm2 API 的 **Cookie 與 Key 只存在記憶體**，不得寫入設定檔、log、錯誤訊息或前端狀態。
- 前端**禁止 `dangerouslySetInnerHTML`**；工作區名稱、標籤、路徑、主題名稱一律當純文字渲染。
- CSP 與 capabilities 維持最小權限（見 1-1-03），不得放寬；禁止載入任何遠端腳本或字型。
- 設定檔 `~/.cue/config.json` 一律**原子寫入**（寫暫存檔再 rename）。

**介面**
- 全介面使用極簡細線內嵌 SVG 圖示，**不使用任何 emoji**（包含警示徽章）。
- 元件中**不得出現硬編碼顏色、字型或圓角**，只讀主題 CSS Variables（4-1-05 起有自動檢查）。
- 主題名稱不得使用「Claude」或其他商標名稱。
- 介面文案使用台灣繁體中文。

**Hook 腳本 `cue-notify`**
- stdout 完全不輸出；任何錯誤都靜默 `exit 0`；socket 不存在直接 `exit 0`；送出時 `nc -U -w 1`（1 秒逾時）。

---

## 技術與慣例

- 套件管理：pnpm（不使用 npm）；前端建置 Vite；測試：前端 Vitest、Rust `cargo test`。
- Rust：tokio、serde／serde_json、notify、plist、window-vibrancy、objc2 系列（NSScreen、NSWorkspace 等）；M6 加入 tokio-tungstenite、prost。
- 前端狀態集中於單一 Zustand store（`src/store/useCue.ts`），所有 Session 狀態更新經由統一函式。
- Rust 端是工作階段狀態的單一真相，前端透過 Tauri IPC（`invoke`、`emit`）同步。
- `TerminalBackend` trait 是唯一的終端機介面，上層只依賴 trait 與 `capabilities()`。
- 程式碼註解可用繁體中文；識別字用英文。

### 註解

- 只寫讀程式碼看不出來的重點（為什麼這樣做、限制、陷阱），讓接手的人不用翻其他檔案就看得懂。
- 不寫指向其他檔案、符號或文件的對照，例如「對照：model.rs、registry.rs」、「（對應 `discovery::SESSIONS_UPDATED`）」、「（001 第 3.1 節）」；真正需要知道的限制直接寫成一句話。
- 不寫開發計畫用語，例如切片／Commit 編號、「之後加入 …」、「之後再定」、「目前只有 …」。

## 目錄結構（規劃）

```
src-tauri/
  src/
    lib.rs, main.rs
    model.rs                # 工作階段領域型別
    osascript.rs            # osascript 執行器與分隔字元解析
    terminal/               # TerminalBackend trait、iterm2.rs、terminal_app.rs、detect.rs
      scripts/*.applescript # 固定腳本原文
    process/                # ps 解析、cwd 取得
    attribution.rs          # 最長前綴歸屬
    registry.rs             # 節點合併與生命週期
    discovery.rs            # 探索迴圈
    config.rs               # ~/.cue/config.json
    commands.rs             # Tauri commands
    shell.rs, layout.rs, dispatch.rs
    git/                    # Git 目錄解析與監聽
    hook/                   # Unix socket 伺服器、cue-notify 安裝
    screen.rs, arrange.rs   # 螢幕座標與視窗調度
    iterm2_api/             # iTerm2 Socket 引擎（M6）
  resources/cue-notify.sh
  examples/                 # 技術探針
src/
  main.tsx, App.tsx, index.css
  ipc/  store/  lib/  hooks/  commands/
  components/  (sidebar/ shell/ newsroom/ theme/ icons/)
  themes/       # builtin.ts、classic.ts、derive.ts、engine.ts
  prefs/        # 偏好設定視窗
  assets/fonts/ # OFL 字型 woff2
docs/  scripts/  install.sh  uninstall.sh
```

## 常用指令

| 指令 | 用途 | 可用時機 |
|---|---|---|
| `pnpm tauri dev` | 開發預覽 | 1-1-01 完成後 |
| `pnpm test` | 前端 Vitest | 1-1-02 完成後 |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Rust 單元測試 | 1-1-01 完成後 |
| `pnpm verify` | cargo check + clippy + cargo test + 前端 build + test（+ 樣式檢查） | 1-1-04 完成後 |
| `cargo run --manifest-path src-tauri/Cargo.toml --example probe_discover` | 終端機探索探針 | 1-2-02 完成後 |
| `pnpm lint:style` | 檢查元件中的硬編碼顏色／字型／圓角 | 4-1-05 完成後 |
| `pnpm tauri build` | 本機打包 | 1-1-01 完成後 |
