# slice-2-3 探索迴圈與設定

> 階段：slice-2 探索與認領（對應規格 M1）
> 上一個子切片：slice-2-2 終端機後端｜下一個子切片：slice-2-4 基礎側邊欄

## 目標

把後端、程序資訊與歸屬串成每 2 秒一次的探索迴圈，管理節點生命週期（包含 `exited` 與 iTerm2 重啟），並以 `~/.cue/config.json` 持久化工作區與節點身分；透過 Tauri IPC 提供快照與事件。

## 必讀

- `docs/PROMPT.md`「二、工作階段探索與認領」第 3～5 點
- `docs/PROMPT.md`「四、GUI 佈局…」第 6 點（持久化）
- `docs/design/001-tech-decisions.md` 第 3.2、3.3 節（輪詢耗時、Terminal.app 分頁行為）

## 完成條件

- 前端可呼叫 `get_snapshot` 取得完整狀態，並在每次變動時收到 `sessions-updated`。
- 關閉 iTerm2 分頁後 ≤ 3 秒節點變為 `exited`；`cd` 進／出工作區 ≤ 3 秒改變歸屬。
- 設定檔原子寫入，重開 Cue 後工作區與標籤仍在。

---

## Commits

### 2-3-01 feat: 設定檔模型與原子寫入
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/config.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`
- **實作重點**：
  - 位置：`~/.cue/config.json`；目錄 `~/.cue` 權限 `0700`。
  - `CueConfig`（`serde`，camelCase，所有欄位 `#[serde(default)]`，加 `version: 1`）：
    - `workspaces: Vec<Workspace { id, path, name, collapsed, #[serde(flatten)] extra: Map<String, Value> }>`（`extra` 為保留給未來的擴充欄位，讀寫時原樣保留）。`name` 預設為資料夾名稱。
    - `nodes: HashMap<String /*SessionKey 字串*/, NodeMeta { label: Option<String>, last_tty, last_cwd, last_kind }>`。
    - `ignored: Vec<String>`（未歸類的忽略清單，以 SessionKey 字串記錄）。
    - `terminal_backend: "auto" | "iterm2" | "terminal"`。
    - `preferences: Map<String, Value>`、`window: Map<String, Value>`（之後切片再定型別，先保留不遺失）。
  - **原子寫入**：寫到同目錄 `config.json.tmp` → `sync_all` → `rename`。寫入失敗只記錄錯誤，不讓 App 崩潰。讀取失敗（JSON 壞掉）時把原檔備份為 `config.json.bak-<時間>` 並以預設值啟動。
  - 寫入以 debounce（例如 300ms）合併，避免輪詢頻繁寫檔。
- **驗證**：單元測試（tempdir）：往返序列化、未知欄位保留（`extra`）、壞檔備份、原子寫入後無殘留 `.tmp`。

### 2-3-02 feat: 合併探索結果並管理節點生命週期
- **UI**：否
- **預計檔案**（2）：`src-tauri/src/registry.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `Registry` 持有所有節點 `SessionNode { id, key, kind, status, workspace_id: Option, cwd, tty, label, default_name, location_hint, window_id, tab_index, alive }`。
  - `merge(discovered, procs, cwds, workspaces, config)`：
    - 以 tty 對應前景程式 → 類型與 cwd → `attribution::attribute` 決定歸屬（`None` 即未歸類）。
    - 已存在的節點更新欄位；新出現的建立節點（標籤從 `config.nodes` 依身分取回）。
    - **消失的節點**標為 `Exited` 並保留（直到使用者「移除」）。
    - **iTerm2 重啟**：所有舊 `unique ID` 都消失 → 全部 `Exited`，新探索到的依歸屬規則重新認領（不做跨 ID 的猜測對應）。
    - `Running`／`Waiting`／`Done` 只能由 Hook 設定（slice-5），merge 不會改變它們，只會在工作階段消失時改為 `Exited`；新節點預設 `Idle`。
  - **預設名稱與位置提示**：名稱預設為類型名（`claude`、`gemini`、`zsh`、或執行檔名）；有自訂標籤時以標籤為名稱。位置提示依後端（001 第 3.3 節）：
    - iTerm2：「視窗 N · 分頁 M」（N 為視窗序號，M 為分頁序號，從 1 起算）。
    - Terminal.app：只顯示「視窗 N」。原生分頁在 AppleScript 中各自是一個視窗，分頁序號永遠是 1，不顯示。
    - 視窗序號會隨前後順序變動，只用於顯示；節點身分一律用 `SessionKey`。
  - 回寫 `config.nodes` 的 `last_tty`、`last_cwd`、`last_kind`（有變動才觸發寫檔）。
  - 忽略清單中的未歸類節點不出現在快照的未歸類區。
- **驗證**：單元測試：新增／更新／消失→exited／全部 ID 改變（重啟）／cd 改歸屬／退回未歸類／標籤沿用／忽略清單。

### 2-3-03 feat: 每 2 秒探索迴圈與 IPC 指令、事件
- **UI**：否
- **預計檔案**（4）：`src-tauri/src/discovery.rs`、`src-tauri/src/commands.rs`、`src-tauri/src/state.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `state.rs`：`AppState { config: Mutex<CueConfig>, registry: Mutex<Registry>, backend: RwLock<Arc<dyn TerminalBackend>> }`，以 `app.manage()` 註冊。
  - `discovery.rs`：tokio 背景任務，`interval(2s)`（`MissedTickBehavior::Skip`）。每輪：`backend.is_running()` 為 false 時跳過終端機探索（但仍把先前存活的節點標為 `Exited`）；否則在 `spawn_blocking` 中執行 `discover()` → 一次 `ps` → 批次 cwd → `registry.merge`。一輪結束後才排下一輪，不重疊。快照有變動才 `emit("sessions-updated", snapshot)`。
    - debug log 記錄每輪耗時；持續超過 1 秒時再依 001 第 3.2 節調整輪詢間隔。
  - 提供 `trigger_discovery()`（之後建立／關閉／Hook 後可立即重新探索）。
  - `commands.rs`：
    - `get_snapshot() -> Snapshot { workspaces, sessions, unclassified, backend: { id, capabilities, installed } }`
    - `focus_session(id)`：呼叫 `backend.focus`；`exited` 節點回傳錯誤。
    - `add_workspace(path)`：驗證為資料夾、realpath、去重，寫入設定並立即重新探索。
    - `set_workspace_collapsed(id, collapsed)`。
    - 錯誤以 `CueError` 序列化回前端。
  - `terminal_backend` 設定在啟動時套用（M1 驗收切換 Terminal.app 時直接編輯 `config.json` 後重開；UI 切換在 4-3-02）。
- **驗證**：`cargo test`、`cargo clippy`；`pnpm tauri dev` 時以 Rust 端 debug log 印出每輪快照摘要（節點數、各自歸屬與狀態），手動開關 iTerm2 分頁、`cd` 進出工作區確認變化（前端畫面在 2-4 才有）。
