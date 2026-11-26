# slice-3-4 Git 分支監聽

> 階段：slice-3 建立、管理、防呆與拖檔派工（對應規格 M2）
> 上一個子切片：slice-3-3 選單與建立 Modal｜下一個子切片：slice-3-5 拖放與派工

## 目標

每個工作區即時顯示 Git 分支徽章（純唯讀），支援 monorepo 子資料夾、worktree／submodule 的 `.git` 檔、Detached HEAD 與事後 `git init`。

## 必讀

- `docs/PROMPT.md`「五、…」第 1 點（Git 分支監聽）

## 完成條件

- `git switch` 後 1 秒內分支徽章更新。
- monorepo 子資料夾、worktree `.git` 檔、Detached HEAD（含 rebase 進行中）、事後 `git init` 都顯示正確；非 Git 顯示「非 Git」。

---

## Commits

### 3-4-01 feat: 解析 Git 目錄與 HEAD
- **UI**：否
- **預計檔案**（3）：`src-tauri/src/git/mod.rs`、`src-tauri/src/git/resolve.rs`、`src-tauri/src/lib.rs`
- **實作重點**：
  - `find_git_dir(workspace: &Path) -> Option<PathBuf>`：從工作區路徑**往上逐層**找 `.git`。
    - `.git` 是資料夾 → 即為 Git 目錄。
    - `.git` 是檔案 → 讀取 `gitdir: <路徑>`；相對路徑以 `.git` 檔所在目錄為基準解析（例如 worktree 指向 `<主 repo>/.git/worktrees/<名稱>`、submodule 指向 `<上層>/.git/modules/<名稱>`）。
  - `read_branch(git_dir) -> Branch`：讀 `HEAD`：
    - `ref: refs/heads/<branch>` → `Branch::Named(branch)`
    - 40 字元十六進位 SHA → `Branch::Detached(sha[..7])`，顯示為 `HEAD <前 7 碼>`（rebase 進行中也屬此類）
    - 找不到 `.git` → `Branch::NotGit`，顯示「非 Git」
  - 全程唯讀，不執行任何 `git` 指令、不寫任何檔案。
- **驗證**：單元測試（tempdir 手工建立 `.git` 結構，不依賴 git 指令）：一般 repo、monorepo 子資料夾、`.git` 檔的相對與絕對 `gitdir`、detached SHA、非 Git。

### 3-4-02 feat: 以 notify 監聽分支變化
- **UI**：否
- **預計檔案**（5）：`src-tauri/src/git/watch.rs`、`src-tauri/src/git/mod.rs`、`src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`、`src-tauri/Cargo.toml`
- **實作重點**：
  - 使用 `notify` crate，**200ms debounce**。
  - Git 以「寫 `HEAD.lock` 再 rename」原子更新 HEAD，因此**非遞迴監聽 Git 目錄本身**並過濾檔名為 `HEAD` 的事件；**不可直接監聽 `HEAD` 檔案**（rename 後 inode 改變，監聽會失效）。
  - 非 Git 工作區：非遞迴監聽**工作區根目錄**，偵測到 `.git` 出現時切換為 Git 監聽。（`.git` 出現在更上層目錄的情況不處理，可在重新加入工作區時偵測。）
  - 變動時 emit `git-branch-changed`：`{ workspaceId, branch, detached }`（`branch` 為顯示字串，非 Git 時為 `null`）。
  - 工作區新增／移除時同步建立／移除 watcher；`get_snapshot` 的 workspace 帶初始分支。
- **驗證**：`#[ignore]` 整合測試：tempdir 中 `git init`、`git switch -c feature`，1 秒內收到事件；本機以真實 repo 操作。

### 3-4-03 feat: 側邊欄分支徽章
- **UI**：是
- **預計檔案**（5）：`src/components/sidebar/BranchBadge.tsx`、`src/components/sidebar/WorkspaceNode.tsx`、`src/ipc/events.ts`、`src/store/useBaton.ts`、`src/index.css`
- **實作重點**：
  - `BranchBadge`：細線分支 SVG 圖示＋分支名（`--font-mono`、`--text-muted`、`--radius-sm` 外框）；Detached 時顯示 `HEAD abc1234`；非 Git 顯示「非 Git」。過長分支名以省略號截斷並以 `title` 顯示全名。
  - 不提供任何 Worktree 巢狀管理 UI。
  - store 新增 `applyBranchUpdate()`。
  - 包浩斯主題的「分支徽章反白」由 4-1 的主題變數控制（例如 `--branch-badge-bg`、`--branch-badge-fg`，預設透明／`--text-muted`），本 Commit 先使用這兩個變數並在 `index.css` 給預設值。
- **驗證**：`npm run build`；本機 `git switch` 後 ≤ 1 秒徽章更新。
