//! Git 分支資訊（純唯讀）：只讀 `.git` 與 `HEAD` 檔案，不執行任何 `git` 指令。

pub mod resolve;
pub mod watch;

pub use resolve::{find_git_dir, read_branch, resolve_branch, Branch};
pub use watch::{BranchChange, GitWatch, GIT_BRANCH_CHANGED};
