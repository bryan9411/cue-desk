//! 由工作區路徑找出 Git 目錄並解析目前的 HEAD。

use std::fs;
use std::path::{Path, PathBuf};

/// 工作區目前的分支狀態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Branch {
    /// `HEAD` 指向 `refs/heads/<name>`；名稱可含 `/`（例如 `feature/x`）。
    Named(String),
    /// `HEAD` 直接是 commit SHA（rebase 進行中也是這種狀態），只保留前 7 碼。
    Detached(String),
    /// 往上找不到 `.git`，或 Git 目錄的 `HEAD` 無法辨識。
    NotGit,
}

impl Branch {
    /// 徽章上的顯示字串；非 Git 時為 `None`，由介面顯示「非 Git」。
    pub fn label(&self) -> Option<String> {
        match self {
            Branch::Named(name) => Some(name.clone()),
            Branch::Detached(short) => Some(format!("HEAD {short}")),
            Branch::NotGit => None,
        }
    }

    pub fn is_detached(&self) -> bool {
        matches!(self, Branch::Detached(_))
    }
}

/// 從工作區路徑往上逐層找 `.git`，回傳實際的 Git 目錄。
///
/// `.git` 是檔案時（worktree、submodule）讀取其中的 `gitdir:`；相對路徑以 `.git`
/// 檔所在目錄為基準。`gitdir:` 指向不存在的目錄時視為非 Git，不再往上找，
/// 以免把子專案誤認成外層 repo 的分支。
pub fn find_git_dir(workspace: &Path) -> Option<PathBuf> {
    for dir in workspace.ancestors() {
        let dot_git = dir.join(".git");
        let Ok(meta) = fs::metadata(&dot_git) else {
            continue;
        };
        if meta.is_dir() {
            return Some(dot_git);
        }
        if meta.is_file() {
            return gitdir_from_file(&dot_git, dir);
        }
    }
    None
}

fn gitdir_from_file(dot_git: &Path, base: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(dot_git).ok()?;
    let target = content
        .lines()
        .find_map(|line| line.strip_prefix("gitdir:"))?
        .trim();
    if target.is_empty() {
        return None;
    }
    let path = Path::new(target);
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    resolved.is_dir().then_some(resolved)
}

/// 讀取 Git 目錄中的 `HEAD`。
pub fn read_branch(git_dir: &Path) -> Branch {
    let Ok(content) = fs::read_to_string(git_dir.join("HEAD")) else {
        return Branch::NotGit;
    };
    parse_head(content.trim())
}

/// 由工作區路徑直接取得分支狀態。
pub fn resolve_branch(workspace: &Path) -> Branch {
    find_git_dir(workspace).map_or(Branch::NotGit, |dir| read_branch(&dir))
}

fn parse_head(head: &str) -> Branch {
    if let Some(target) = head.strip_prefix("ref:") {
        let target = target.trim();
        let name = target.strip_prefix("refs/heads/").unwrap_or(target);
        return if name.is_empty() {
            Branch::NotGit
        } else {
            Branch::Named(name.to_string())
        };
    }
    // SHA-1 為 40 碼；SHA-256 格式的 repo 為 64 碼。
    let is_sha = matches!(head.len(), 40 | 64) && head.bytes().all(|b| b.is_ascii_hexdigit());
    if is_sha {
        Branch::Detached(head[..7].to_ascii_lowercase())
    } else {
        Branch::NotGit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const SHA: &str = "a1b2c3d4e5f60718293a4b5c6d7e8f9012345678";

    fn make_git_dir(dir: &Path, head: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("HEAD"), head).unwrap();
    }

    #[test]
    fn plain_repo_named_branch() {
        let tmp = TempDir::new().unwrap();
        make_git_dir(&tmp.path().join(".git"), "ref: refs/heads/main\n");
        assert_eq!(find_git_dir(tmp.path()), Some(tmp.path().join(".git")));
        assert_eq!(resolve_branch(tmp.path()), Branch::Named("main".into()));
    }

    #[test]
    fn branch_name_with_slash() {
        let tmp = TempDir::new().unwrap();
        make_git_dir(&tmp.path().join(".git"), "ref: refs/heads/feature/login\n");
        let branch = resolve_branch(tmp.path());
        assert_eq!(branch.label().as_deref(), Some("feature/login"));
        assert!(!branch.is_detached());
    }

    #[test]
    fn monorepo_subfolder_walks_up() {
        let tmp = TempDir::new().unwrap();
        make_git_dir(&tmp.path().join(".git"), "ref: refs/heads/develop\n");
        let sub = tmp.path().join("packages/web/src");
        fs::create_dir_all(&sub).unwrap();
        assert_eq!(find_git_dir(&sub), Some(tmp.path().join(".git")));
        assert_eq!(resolve_branch(&sub), Branch::Named("develop".into()));
    }

    #[test]
    fn dot_git_file_relative_gitdir() {
        let tmp = TempDir::new().unwrap();
        let wt_git = tmp.path().join("main/.git/worktrees/wt");
        make_git_dir(&wt_git, "ref: refs/heads/hotfix\n");
        let wt = tmp.path().join("wt");
        fs::create_dir_all(&wt).unwrap();
        fs::write(wt.join(".git"), "gitdir: ../main/.git/worktrees/wt\n").unwrap();

        assert_eq!(find_git_dir(&wt), Some(wt.join("../main/.git/worktrees/wt")));
        assert_eq!(resolve_branch(&wt), Branch::Named("hotfix".into()));
    }

    #[test]
    fn dot_git_file_absolute_gitdir() {
        let tmp = TempDir::new().unwrap();
        let module_git = tmp.path().join("parent/.git/modules/lib");
        make_git_dir(&module_git, "ref: refs/heads/main\n");
        let sub = tmp.path().join("parent/lib");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(".git"), format!("gitdir: {}\n", module_git.display())).unwrap();

        assert_eq!(find_git_dir(&sub), Some(module_git));
        assert_eq!(resolve_branch(&sub), Branch::Named("main".into()));
    }

    #[test]
    fn dot_git_file_with_missing_target_is_not_git() {
        let tmp = TempDir::new().unwrap();
        make_git_dir(&tmp.path().join(".git"), "ref: refs/heads/outer\n");
        let sub = tmp.path().join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(".git"), "gitdir: ../nowhere\n").unwrap();

        assert_eq!(find_git_dir(&sub), None);
        assert_eq!(resolve_branch(&sub), Branch::NotGit);
    }

    #[test]
    fn detached_sha_keeps_seven_chars() {
        let tmp = TempDir::new().unwrap();
        make_git_dir(&tmp.path().join(".git"), &format!("{SHA}\n"));
        let branch = resolve_branch(tmp.path());
        assert_eq!(branch, Branch::Detached("a1b2c3d".into()));
        assert_eq!(branch.label().as_deref(), Some("HEAD a1b2c3d"));
        assert!(branch.is_detached());
    }

    #[test]
    fn non_git_folder() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("plain");
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(find_git_dir(&dir), None);
        let branch = resolve_branch(&dir);
        assert_eq!(branch, Branch::NotGit);
        assert_eq!(branch.label(), None);
    }

    #[test]
    fn malformed_head_is_not_git() {
        assert_eq!(parse_head("garbage"), Branch::NotGit);
        assert_eq!(parse_head("ref:"), Branch::NotGit);
        assert_eq!(parse_head(&SHA[..39]), Branch::NotGit);
    }
}
