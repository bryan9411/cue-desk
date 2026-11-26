# slice-2-1 領域模型與程序解析

> 階段：slice-2 探索與認領（對應規格 M1）
> 上一個子切片：slice-1-3 技術決策｜下一個子切片：slice-2-2 終端機後端

## 目標

建立 Rust 端的核心型別與 `TerminalBackend` trait，並完成不依賴終端機 App 的純邏輯：tty 正規化、`ps` 解析、前景程式與類型判斷、cwd 取得、最長前綴歸屬。全部有單元測試。

## 必讀

- `docs/design/001-tech-decisions.md` 第 1、2 節（cwd 取得方式、tty 正規化）與第 3.4 節（前景程式與類型判斷）
- `docs/PROMPT.md`「一、系統架構」第 1 點「終端機轉接層」段落

## 完成條件

- `cargo test` 涵蓋：tty 正規化、ps 解析、前景判斷（群組領頭、領頭已結束）、類型判斷（原生 claude、舊版 npm `cli.js`、`npm exec @anthropic-ai/claude-code`、gemini 及其重新啟動形式、`npm exec @google/gemini-cli`、`-zsh`／完整路徑 shell、vim、一般 `npm`）、最長前綴歸屬（巢狀、邊界、symlink、未歸類）。
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
    - `DiscoveredSession`：key、tty（正規化後）、name、window_id、window_index、window_bounds（點座標 `left, top, right, bottom`）、tab_index。`window_id` 是視窗身分；`window_index` 與 `name` 只用於顯示（001 第 3.1 節）。
    - `Bounds`、`ScreenIndex` 等之後會用的小型別。
    - `normalize_tty(&str) -> Option<String>`（001 第 2 節）：去前後空白 → 去開頭 `/dev/` → 必須符合 `^ttys[0-9]+$`（位數不固定），否則 `None`。iTerm2、Terminal.app、`ps`、Hook 四個來源**一律經過這個函式**，其他地方不得自行處理 tty 字串。
  - `terminal/mod.rs`：
    - `Capabilities { split, new_tab, type_text_no_enter, rename, close_pane, precise_tabs }`。
    - `OpenMode`：`NewWindow`｜`NewTab`｜`AiTopZshBottom`｜`AiLeftZshRight`；`SplitDirection`：`Right`｜`Below`。
    - `trait TerminalBackend: Send + Sync`（同步方法，由呼叫端以 `spawn_blocking` 執行）：`id()`、`capabilities()`、`is_running()`、`discover()`、`focus(&SessionKey)`、`create(&CreateSpec)`、`split_zsh(&SessionKey, SplitDirection, cwd)`、`rename(&SessionKey, &str)`、`close(&SessionKey)`、`type_text_no_enter(&SessionKey, &str)`、`arrange_beside(Bounds)`、`move_to_screen(&SessionKey, usize)`、`gather_windows(&[WindowId], Bounds)`。除 `id`／`capabilities`／`is_running`／`discover`／`focus` 外，預設實作回傳 `Err(CueError::Unsupported)`。
  - 上層只能依賴 trait，之後新增 Ghostty／WezTerm 不需改上層。
- **驗證**：`cargo check`、`cargo clippy`；序列化測試確認 `Status::Waiting` → `"waiting"`、`SessionKey` 字串形式；`normalize_tty` 測試：`/dev/ttys000`、`ttys021`、` ttys1234 `、`??`、`console`、空字串、`/dev/ttys`、`ttys0a`。

### 2-1-02 feat: 解析 ps 輸出並判斷前景程式與類型
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/process/mod.rs`、`src-tauri/src/process/ps.rs`、`src-tauri/src/process/fixtures/ps_sample.txt`、`src-tauri/src/lib.rs`
- **實作重點**（001 第 3.4 節）：
  - 每次輪詢**只執行一次** `ps -A -ww -o pid=,ppid=,pgid=,tpgid=,tty=,args=`（`-ww` 避免 `args` 被欄寬截斷），解析成 `ProcInfo { pid, ppid, pgid, tpgid, tty, args }`。`args` 可能含空白，前五欄以空白切割，剩餘整段為 args。tty 經 `model::normalize_tty`，`None`（例如 `??`）的程序略過。
  - 排除 Cue 自己的子程序（`ppid == std::process::id()`，例如 `ps`、`osascript`）。
  - **前景程式＝前景程序群組的領頭**：每個 tty 中 `pgid == tpgid` 的群組，取 `pid == pgid` 的那支；領頭已結束時取群組中 pid 最小者。**不得把群組內每支程序都當前景**（`claude` 的 MCP 伺服器等子程序在同一群組，cwd 也可能不同）；2-1-03 的 cwd 也只取這支程序。
  - 對外提供以 tty 查詢前景的介面（例如 `foreground_by_tty(&[ProcInfo]) -> HashMap<String, Foreground { pid, kind }>`）。**工作階段只來自 `discover()`**：這裡只負責查詢，不得為 `ps` 看到的其他 tty（例如其他 App 內建的終端機面板）產生工作階段。
  - **類型判斷**（`classify`）：先看領頭；領頭是包裝程式（`npm exec …`、`npx …`、腳本不是 claude／gemini 的 `node`）時，再往**同一群組**的子孫程序找 claude／gemini。
    - 執行檔 basename 比對前去掉登入 shell 開頭的 `-`，並接受完整路徑。
    - `Claude`：basename 為 `claude`（原生版；新版 npm 也是內附原生執行檔）；或 `node`／`bun` 執行的腳本路徑含 `@anthropic-ai/claude-code`（舊版 `cli.js` 形式）；或 `npm exec @anthropic-ai/claude-code`。
    - `Gemini`：腳本檔名為 `gemini`（例如 `node …/.bin/gemini`，含 `node --max-old-space-size=8192 …/.bin/gemini` 的重新啟動形式）；或 `npm exec @google/gemini-cli`。
    - `Zsh`：前景是 shell 本身（`zsh`、`bash`、`fish`、`sh`，含 `-zsh`、`/opt/homebrew/bin/zsh -l`）。
    - 其他 → `Other(basename)`，例如 `vim`、一般的 `npm run dev`。
  - fixture 收錄真實格式的範例（參考 `docs/design/notes-probe.md`），涵蓋上述所有類型、同一 tty 多個程序群組、claude 群組內的 MCP 子程序、領頭已結束、`??` 與 Cue 子程序。
- **驗證**：`cargo test process::ps`。

### 2-1-03 feat: 取得前景程式的工作目錄
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/process/cwd.rs`、`src-tauri/src/process/mod.rs`、`src-tauri/Cargo.toml`
- **實作重點**（001 第 1 節）：
  - `cwds(pids: &[i32]) -> HashMap<i32, PathBuf>`：以 `libc::proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, …)` 讀 `pvi_cdir.vip_path`。**不使用 `lsof`，不引入 `libproc` crate**。
  - `Cargo.toml`：把 `libc = "0.2"`（MIT OR Apache-2.0）從 `[dev-dependencies]` 移到 `[dependencies]`。
  - `vip_path` 在 `libc` 中是 `[[c_char; 32]; 32]`，攤平成 1024 位元組後以 `CStr::from_bytes_until_nul` 讀出（同 `examples/probe_cwd.rs` 的 `cwd_libproc`）。
  - 錯誤處理：回傳值 `<= 0` 或大小不等於結構大小時略過該 pid。`EPERM`（非本使用者程序）表示無法取得 cwd，該工作階段放進「未歸類」，不當錯誤上報；`ESRCH`（程序已結束）是正常競態，下一輪再試。任何 pid 失敗都不讓整輪探索失敗。
  - 只傳入 2-1-02 找到的前景領頭 pid；一輪探索只呼叫一次。
- **驗證**：單元測試以目前程序（`std::process::id()`）驗證回傳值等於 `std::env::current_dir()`；不存在的 pid 回傳空結果不 panic；`#[ignore]` 測試對本機 shell 執行。

### 2-1-04 feat: 以 realpath 最長前綴歸屬工作區
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/attribution.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`（加入 dev-dependency `tempfile`）
- **實作重點**：
  - `attribute(cwd: &Path, workspaces: &[(WorkspaceId, PathBuf)]) -> Option<WorkspaceId>`：cwd 與所有工作區根目錄都做 `realpath`（`std::fs::canonicalize`；失敗時退回字面路徑），以**路徑元件**比對前綴（`/a/foo` 不屬於 `/a/fo`），取最長者；都不符合回傳 `None`（即「未歸類」）。
  - 工作區根目錄的 realpath 可快取，工作區清單變動時重算。
  - **單一真相**：歸屬只由這個函式決定，沒有任何手動覆寫參數。
- **驗證**：單元測試（使用 `tempfile` 建目錄）：巢狀工作區取最長、前綴邊界、symlink 指向工作區內、尾端斜線、`~` 與 `~/Downloads` 未歸類。
