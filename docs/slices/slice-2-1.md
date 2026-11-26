# slice-2-1 領域模型與程序解析

> 階段：slice-2 探索與認領（對應規格 M1）
> 上一個子切片：slice-1-3 技術決策｜下一個子切片：slice-2-2 終端機後端

> ⚠️ cwd 取得方式（2-1-03）與 tty 正規化規則會在 slice-1-3 依探針結果定案後更新本文件。

## 目標

建立 Rust 端的核心型別與 `TerminalBackend` trait，並完成不依賴終端機 App 的純邏輯：`ps` 解析、前景程式與類型判斷、cwd 取得、最長前綴歸屬。全部有單元測試。

## 必讀

- `docs/design/001-tech-decisions.md` 第 1、2 節（cwd 取得方式、tty 正規化）
- `docs/PROMPT.md`「一、系統架構」第 1 點「終端機轉接層」段落

## 完成條件

- `cargo test` 涵蓋：ps 解析、前景判斷、類型判斷（原生 claude、npm claude、gemini、`@google/gemini-cli`、zsh、vim、npm）、最長前綴歸屬（巢狀、邊界、symlink、未歸類）。
- trait 已宣告所有規格方法，尚未實作的方法回傳 `Unsupported`。

---

## Commits

### 2-1-01 feat: 定義工作階段領域型別與 TerminalBackend trait
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/model.rs`、`src-tauri/src/terminal/mod.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `model.rs`（皆 `Serialize`，前端欄位 camelCase）：
    - `SessionKind`：`Claude`｜`Gemini`｜`Zsh`｜`Other(String)`（Other 顯示執行檔名稱）。
    - `Status`：`Idle`｜`Running`｜`Waiting`｜`Done`｜`Exited`（序列化為小寫字串）。
    - `SessionKey`：`Iterm2 { unique_id }`｜`TerminalApp { window_id, tty }`，提供穩定字串形式 `iterm2:<id>`／`terminal:<wid>:<tty>` 作為節點 id。
    - `DiscoveredSession`：key、tty（正規化後）、name、window_id、window_index、window_bounds（點座標 `left, top, right, bottom`）、tab_index。
    - `Bounds`、`ScreenIndex` 等之後會用的小型別。
  - `terminal/mod.rs`：
    - `Capabilities { split, new_tab, type_text_no_enter, rename, close_pane, precise_tabs }`。
    - `OpenMode`：`NewWindow`｜`NewTab`｜`AiTopZshBottom`｜`AiLeftZshRight`；`SplitDirection`：`Right`｜`Below`。
    - `trait TerminalBackend: Send + Sync`（同步方法，由呼叫端以 `spawn_blocking` 執行）：`id()`、`capabilities()`、`is_running()`、`discover()`、`focus(&SessionKey)`、`create(&CreateSpec)`、`split_zsh(&SessionKey, SplitDirection, cwd)`、`rename(&SessionKey, &str)`、`close(&SessionKey)`、`type_text_no_enter(&SessionKey, &str)`、`arrange_beside(Bounds)`、`move_to_screen(&SessionKey, usize)`、`gather_windows(&[WindowId], Bounds)`。除 `id`／`capabilities`／`is_running`／`discover`／`focus` 外，預設實作回傳 `Err(CueError::Unsupported)`。
  - 上層只能依賴 trait，之後新增 Ghostty／WezTerm 不需改上層。
- **驗證**：`cargo check`、`cargo clippy`；序列化測試確認 `Status::Waiting` → `"waiting"`、`SessionKey` 字串形式。

### 2-1-02 feat: 解析 ps 輸出並判斷前景程式與類型
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/process/mod.rs`、`src-tauri/src/process/ps.rs`、`src-tauri/src/process/fixtures/ps_sample.txt`、`src-tauri/src/lib.rs`
- **實作重點**：
  - 每次輪詢**只執行一次** `ps -A -o pid=,ppid=,pgid=,tpgid=,tty=,args=`，解析成 `ProcInfo { pid, ppid, pgid, tpgid, tty, args }`。`args` 可能含空白，前五欄以空白切割，剩餘整段為 args。`tty` 為 `??` 的略過；tty 依 001 決策正規化為 `ttysNNN`。
  - **前景程式**：每個 tty 中 `pgid == tpgid` 的那組程序；代表程序優先取 `pid == pgid`（群組 leader），沒有則取 pid 最小者。
  - **類型判斷**（`classify(args) -> SessionKind`）：
    - 執行檔 basename（去掉登入 shell 的前導 `-`）。
    - basename 為 `claude`，或為 `node`／`bun` 且後續任一參數的路徑含 `claude`（例如 `/opt/homebrew/bin/claude`、`…/@anthropic-ai/claude-code/cli.js`）→ `Claude`。
    - basename 為 `gemini`，或 `node` 且參數含 `gemini` 或 `@google/gemini-cli` → `Gemini`。
    - basename 為 `zsh`、`bash`、`fish`、`sh` → `Zsh`（前景是 shell 本身）。
    - 其他 → `Other(basename)`，例如 `npm`、`vim`。
  - fixture 收錄真實格式的範例（可參考 `docs/design/notes-probe.md`），涵蓋上述所有類型與同一 tty 多個程序群組的情況。
- **驗證**：`cargo test process::ps`。

### 2-1-03 feat: 取得前景程式的工作目錄
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/process/cwd.rs`、`src-tauri/src/process/mod.rs`、`src-tauri/Cargo.toml`
- **實作重點**：
  - 依 001 決策實作 `cwds(pids: &[i32]) -> HashMap<i32, PathBuf>`（預設方案：libproc `PROC_PIDVNODEPATHINFO` 取 `pvi_cdir`；備案：一次批次 `lsof -a -p <pids> -d cwd -Fn`）。
  - 取不到（權限、程序已結束）就略過該 pid，不讓整輪探索失敗。
  - 一輪探索只呼叫一次（批次）。
- **驗證**：單元測試以目前程序（`std::process::id()`）驗證回傳值等於 `std::env::current_dir()`；`#[ignore]` 測試對本機 shell 執行。

### 2-1-04 feat: 以 realpath 最長前綴歸屬工作區
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/attribution.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`（加入 dev-dependency `tempfile`）
- **實作重點**：
  - `attribute(cwd: &Path, workspaces: &[(WorkspaceId, PathBuf)]) -> Option<WorkspaceId>`：cwd 與所有工作區根目錄都做 `realpath`（`std::fs::canonicalize`；失敗時退回字面路徑），以**路徑元件**比對前綴（`/a/foo` 不屬於 `/a/fo`），取最長者；都不符合回傳 `None`（即「未歸類」）。
  - 工作區根目錄的 realpath 可快取，工作區清單變動時重算。
  - **單一真相**：歸屬只由這個函式決定，沒有任何手動覆寫參數。
- **驗證**：單元測試（使用 `tempfile` 建目錄）：巢狀工作區取最長、前綴邊界、symlink 指向工作區內、尾端斜線、`~` 與 `~/Downloads` 未歸類。
