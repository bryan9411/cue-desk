//! 監聽各工作區的分支變化（純唯讀，不執行任何 `git` 指令）。
//!
//! Git 以「寫 `HEAD.lock` 再 rename」原子更新 HEAD，rename 後 inode 會換掉，所以不能
//! 直接監聽 `HEAD` 檔案，而是非遞迴監聽 Git 目錄本身並只看檔名為 `HEAD` 的事件。
//! 非 Git 工作區改為非遞迴監聽工作區根目錄，`.git` 出現時切換成監聽 Git 目錄。
//! `.git` 出現在工作區上層目錄的情況偵測不到，要重新加入工作區才會更新。

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;

use super::resolve::{find_git_dir, read_branch, Branch};
use crate::state::lock;

/// 分支變動時送給前端的事件。
pub const GIT_BRANCH_CHANGED: &str = "git-branch-changed";
/// 最後一個事件後靜默這麼久才重新讀取，合併同一次操作的多個事件。
const DEBOUNCE: Duration = Duration::from_millis(200);
/// 事件持續不斷時，最晚在第一個事件後這麼久就處理，確保 1 秒內更新。
const MAX_DELAY: Duration = Duration::from_millis(600);

/// `git-branch-changed` 的內容。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchChange {
    pub workspace_id: String,
    /// 顯示字串；非 Git 時為 `null`。
    pub branch: Option<String>,
    pub detached: bool,
}

impl BranchChange {
    fn new(workspace_id: &str, branch: &Branch) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            branch: branch.label(),
            detached: branch.is_detached(),
        }
    }
}

struct Entry {
    workspace: PathBuf,
    /// 目前監聽的路徑（Git 目錄或工作區根目錄）；監聽失敗時為 `None`。
    watched: Option<PathBuf>,
    /// 被 drop 時停止監聽。
    watcher: Option<RecommendedWatcher>,
    branch: Branch,
}

/// 監聽器與目前分支，可在背景執行緒中共用。
#[derive(Clone)]
struct Shared {
    entries: Arc<Mutex<HashMap<String, Entry>>>,
    /// 監聽器回呼送出有事件的工作區 id。
    tx: Sender<String>,
}

impl Shared {
    /// 重新解析工作區的 Git 目錄與分支，必要時換監聽目標。回傳分支是否改變。
    ///
    /// 先換好監聽再讀 HEAD，兩者之間的變動才會再觸發一次事件，不會漏掉。
    fn refresh_entry(&self, id: &str, entry: &mut Entry) -> bool {
        let git_dir = find_git_dir(&entry.workspace);
        let target = git_dir.clone().unwrap_or_else(|| entry.workspace.clone());
        if entry.watched.as_ref() != Some(&target) {
            entry.watcher = None;
            entry.watched = None;
            if let Some(watcher) = self.make_watcher(id, &target) {
                entry.watcher = Some(watcher);
                entry.watched = Some(target);
            }
        }
        let branch = git_dir.map_or(Branch::NotGit, |dir| read_branch(&dir));
        if branch == entry.branch {
            return false;
        }
        entry.branch = branch;
        true
    }

    fn refresh(&self, ids: &HashSet<String>) -> Vec<BranchChange> {
        let mut entries = lock(&self.entries);
        let mut changes = Vec::new();
        for id in ids {
            // 工作區可能在等待期間被移除。
            let Some(entry) = entries.get_mut(id) else {
                continue;
            };
            if self.refresh_entry(id, entry) {
                changes.push(BranchChange::new(id, &entry.branch));
            }
        }
        changes
    }

    fn make_watcher(&self, id: &str, path: &Path) -> Option<RecommendedWatcher> {
        let tx = self.tx.clone();
        let id = id.to_string();
        let handler = move |res: notify::Result<Event>| {
            let Ok(event) = res else {
                return;
            };
            if event.paths.iter().any(|p| is_relevant(p)) {
                // 接收端只在 App 結束時消失，送不出去也沒有要補救的。
                let _ = tx.send(id.clone());
            }
        };
        let mut watcher = match notify::recommended_watcher(handler) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("[cue] 無法建立 Git 分支監聽：{e}");
                return None;
            }
        };
        if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
            eprintln!("[cue] 無法監聽 Git 分支：{e}");
            return None;
        }
        Some(watcher)
    }
}

/// 只有 `HEAD`（Git 目錄內）與 `.git`（工作區根目錄內）的變動會影響分支。
/// 只比對檔名：事件路徑可能是 realpath（例如 `/private/var/...`），與監聽時的路徑不同。
fn is_relevant(path: &Path) -> bool {
    matches!(path.file_name().and_then(OsStr::to_str), Some("HEAD" | ".git"))
}

/// 所有工作區的分支監聽。
pub struct GitWatch {
    shared: Shared,
    /// 尚未啟動背景執行緒前暫存的接收端；啟動前的事件會留在佇列中。
    rx: Mutex<Option<Receiver<String>>>,
}

impl Default for GitWatch {
    fn default() -> Self {
        Self::new()
    }
}

impl GitWatch {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            shared: Shared { entries: Arc::new(Mutex::new(HashMap::new())), tx },
            rx: Mutex::new(Some(rx)),
        }
    }

    /// 啟動處理事件的背景執行緒；分支改變時呼叫 `on_change`。重複呼叫不會有作用。
    pub fn start<F>(&self, on_change: F)
    where
        F: Fn(BranchChange) + Send + 'static,
    {
        let Some(rx) = lock(&self.rx).take() else {
            return;
        };
        let shared = self.shared.clone();
        thread::Builder::new()
            .name("cue-git-watch".into())
            .spawn(move || run_worker(&shared, &rx, &on_change))
            .map_err(|e| eprintln!("[cue] 無法啟動 Git 分支監聽：{e}"))
            .ok();
    }

    /// 依目前的工作區清單建立／移除監聽。新加入的工作區會立即讀取分支，不送事件
    /// （初始分支由快照帶出）。
    pub fn sync(&self, workspaces: &[(String, PathBuf)]) {
        let mut entries = lock(&self.shared.entries);
        let wanted: HashSet<&str> = workspaces.iter().map(|(id, _)| id.as_str()).collect();
        entries.retain(|id, _| wanted.contains(id.as_str()));
        for (id, path) in workspaces {
            if entries.get(id).is_some_and(|e| &e.workspace == path) {
                continue;
            }
            let mut entry =
                Entry { workspace: path.clone(), watched: None, watcher: None, branch: Branch::NotGit };
            self.shared.refresh_entry(id, &mut entry);
            entries.insert(id.clone(), entry);
        }
    }

    /// 各工作區目前的分支。
    pub fn branches(&self) -> HashMap<String, Branch> {
        lock(&self.shared.entries).iter().map(|(id, e)| (id.clone(), e.branch.clone())).collect()
    }
}

fn run_worker<F: Fn(BranchChange)>(shared: &Shared, rx: &Receiver<String>, on_change: &F) {
    while let Ok(first) = rx.recv() {
        let deadline = Instant::now() + MAX_DELAY;
        let mut pending = HashSet::from([first]);
        loop {
            let wait = DEBOUNCE.min(deadline.saturating_duration_since(Instant::now()));
            if wait.is_zero() {
                break;
            }
            match rx.recv_timeout(wait) {
                Ok(id) => {
                    pending.insert(id);
                }
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
            }
        }
        for change in shared.refresh(&pending) {
            on_change(change);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn relevant_file_names() {
        assert!(is_relevant(Path::new("/r/.git/HEAD")));
        assert!(is_relevant(Path::new("/r/.git")));
        assert!(!is_relevant(Path::new("/r/.git/HEAD.lock")));
        assert!(!is_relevant(Path::new("/r/.git/index")));
        assert!(!is_relevant(Path::new("/r/src/main.rs")));
    }

    #[test]
    fn sync_reads_initial_branch_and_drops_removed() {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        let plain = tmp.path().join("plain");
        fs::create_dir_all(&plain).unwrap();

        let watch = GitWatch::new();
        watch.sync(&[("a".into(), repo), ("b".into(), plain)]);
        let branches = watch.branches();
        assert_eq!(branches.get("a"), Some(&Branch::Named("main".into())));
        assert_eq!(branches.get("b"), Some(&Branch::NotGit));

        watch.sync(&[]);
        assert!(watch.branches().is_empty());
    }

    #[test]
    fn change_payload_shape() {
        let change = BranchChange::new("w", &Branch::Detached("abc1234".into()));
        assert_eq!(
            serde_json::to_value(&change).unwrap(),
            serde_json::json!({ "workspaceId": "w", "branch": "HEAD abc1234", "detached": true })
        );
        let change = BranchChange::new("w", &Branch::NotGit);
        assert_eq!(change.branch, None);
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(["-c", "user.name=cue", "-c", "user.email=cue@example.invalid"])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap()
            .status;
        assert!(status.success(), "git {args:?} 失敗");
    }

    fn started(dir: &Path) -> (GitWatch, Receiver<BranchChange>) {
        let watch = GitWatch::new();
        watch.sync(&[("w".into(), dir.to_path_buf())]);
        let (tx, rx) = mpsc::channel();
        watch.start(move |c| {
            let _ = tx.send(c);
        });
        (watch, rx)
    }

    /// 需要本機 `git`：`cargo test -- --ignored git::watch`
    #[test]
    #[ignore]
    fn switch_branch_emits_within_one_second() {
        let tmp = TempDir::new().unwrap();
        git(tmp.path(), &["init", "-q", "-b", "main"]);
        git(tmp.path(), &["commit", "-q", "--allow-empty", "-m", "init"]);
        let (_watch, rx) = started(tmp.path());

        git(tmp.path(), &["switch", "-q", "-c", "feature"]);
        let change = rx.recv_timeout(Duration::from_secs(1)).expect("1 秒內沒有收到事件");
        assert_eq!(change.branch.as_deref(), Some("feature"));
        assert!(!change.detached);
    }

    #[test]
    #[ignore]
    fn git_init_after_add_switches_to_git_watch() {
        let tmp = TempDir::new().unwrap();
        let (_watch, rx) = started(tmp.path());

        git(tmp.path(), &["init", "-q", "-b", "main"]);
        let change = rx.recv_timeout(Duration::from_secs(1)).expect("1 秒內沒有收到 git init 事件");
        assert_eq!(change.branch.as_deref(), Some("main"));

        git(tmp.path(), &["commit", "-q", "--allow-empty", "-m", "init"]);
        git(tmp.path(), &["switch", "-q", "--detach"]);
        let change = rx.recv_timeout(Duration::from_secs(1)).expect("1 秒內沒有收到 detach 事件");
        assert!(change.detached);
        assert!(change.branch.unwrap().starts_with("HEAD "));
    }
}
