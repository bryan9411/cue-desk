//! 工作階段歸屬：以前景程式 cwd 的 realpath 對工作區根目錄做最長前綴比對。
//!
//! 歸屬的**單一真相**就是這裡的比對結果，沒有任何手動覆寫參數。

use std::path::{Path, PathBuf};

/// 取 realpath；失敗（例如路徑不存在、無權限）時退回字面路徑。
fn resolve(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// 已解析 realpath 的工作區根目錄清單，可在工作區清單變動前重複使用。
#[derive(Debug, Clone)]
pub struct WorkspaceRoots<Id> {
    roots: Vec<(Id, PathBuf)>,
}

impl<Id: Clone> WorkspaceRoots<Id> {
    /// 解析每個工作區根目錄的 realpath。工作區清單變動時重新建立。
    pub fn new(workspaces: &[(Id, PathBuf)]) -> Self {
        let roots = workspaces.iter().map(|(id, root)| (id.clone(), resolve(root))).collect();
        Self { roots }
    }

    /// 回傳 cwd 所屬的工作區；都不符合時回傳 `None`（未歸類）。
    ///
    /// 以路徑元件比對前綴（`/a/foo` 不屬於 `/a/fo`），多個工作區都符合時取最長者；
    /// 長度相同時取清單中較前面的那個。
    pub fn attribute(&self, cwd: &Path) -> Option<Id> {
        let cwd = resolve(cwd);
        let mut best: Option<(&Id, usize)> = None;
        for (id, root) in &self.roots {
            if !cwd.starts_with(root) {
                continue;
            }
            let depth = root.components().count();
            if best.is_none_or(|(_, d)| depth > d) {
                best = Some((id, depth));
            }
        }
        best.map(|(id, _)| id.clone())
    }
}

/// 單次歸屬：解析所有工作區根目錄後比對。需要重複比對時改用 [`WorkspaceRoots`] 快取。
pub fn attribute<Id: Clone>(cwd: &Path, workspaces: &[(Id, PathBuf)]) -> Option<Id> {
    WorkspaceRoots::new(workspaces).attribute(cwd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn ws(list: &[(&str, &Path)]) -> Vec<(String, PathBuf)> {
        list.iter().map(|(id, p)| (id.to_string(), p.to_path_buf())).collect()
    }

    #[test]
    fn nested_workspace_takes_longest() {
        let tmp = TempDir::new().unwrap();
        let outer = tmp.path().join("code");
        let inner = outer.join("app");
        fs::create_dir_all(inner.join("src")).unwrap();
        let list = ws(&[("outer", &outer), ("inner", &inner)]);

        assert_eq!(attribute(&inner.join("src"), &list).as_deref(), Some("inner"));
        assert_eq!(attribute(&inner, &list).as_deref(), Some("inner"));
        assert_eq!(attribute(&outer, &list).as_deref(), Some("outer"));

        // 清單順序不影響結果
        let reversed = ws(&[("inner", &inner), ("outer", &outer)]);
        assert_eq!(attribute(&inner.join("src"), &reversed).as_deref(), Some("inner"));
    }

    #[test]
    fn prefix_boundary_uses_components() {
        let tmp = TempDir::new().unwrap();
        let fo = tmp.path().join("fo");
        let foo = tmp.path().join("foo");
        fs::create_dir_all(&fo).unwrap();
        fs::create_dir_all(&foo).unwrap();
        let list = ws(&[("fo", &fo)]);

        assert_eq!(attribute(&foo, &list), None);
        assert_eq!(attribute(&fo, &list).as_deref(), Some("fo"));
    }

    #[test]
    fn symlink_into_workspace_resolves() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(proj.join("pkg")).unwrap();
        let link = tmp.path().join("shortcut");
        std::os::unix::fs::symlink(proj.join("pkg"), &link).unwrap();
        let list = ws(&[("proj", &proj)]);

        assert_eq!(attribute(&link, &list).as_deref(), Some("proj"));
    }

    #[test]
    fn workspace_root_via_symlink_resolves() {
        let tmp = TempDir::new().unwrap();
        let real = tmp.path().join("real");
        fs::create_dir_all(real.join("sub")).unwrap();
        let link = tmp.path().join("alias");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        // 工作區以 symlink 路徑登錄，cwd 是真實路徑
        let list = ws(&[("ws", &link)]);

        assert_eq!(attribute(&real.join("sub"), &list).as_deref(), Some("ws"));
    }

    #[test]
    fn trailing_slash_is_ignored() {
        let tmp = TempDir::new().unwrap();
        let proj = tmp.path().join("proj");
        fs::create_dir_all(proj.join("src")).unwrap();
        let root_with_slash = PathBuf::from(format!("{}/", proj.display()));
        let cwd_with_slash = PathBuf::from(format!("{}/", proj.join("src").display()));
        let list = ws(&[("proj", &root_with_slash)]);

        assert_eq!(attribute(&cwd_with_slash, &list).as_deref(), Some("proj"));
        assert_eq!(attribute(&proj, &list).as_deref(), Some("proj"));
    }

    #[test]
    fn home_and_downloads_are_unattributed() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        let downloads = home.join("Downloads");
        let proj = home.join("code").join("proj");
        fs::create_dir_all(&downloads).unwrap();
        fs::create_dir_all(&proj).unwrap();
        let list = ws(&[("proj", &proj)]);

        assert_eq!(attribute(&home, &list), None);
        assert_eq!(attribute(&downloads, &list), None);
    }

    #[test]
    fn missing_paths_fall_back_to_literal() {
        let list = ws(&[("ws", Path::new("/nonexistent-cue-test/ws"))]);
        assert_eq!(
            attribute(Path::new("/nonexistent-cue-test/ws/deep"), &list).as_deref(),
            Some("ws")
        );
        assert_eq!(attribute(Path::new("/nonexistent-cue-test/other"), &list), None);
    }

    #[test]
    fn cached_roots_reusable() {
        let tmp = TempDir::new().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();
        let roots = WorkspaceRoots::new(&[(1u32, a.clone()), (2u32, b.clone())]);

        assert_eq!(roots.attribute(&a), Some(1));
        assert_eq!(roots.attribute(&b), Some(2));
        assert_eq!(roots.attribute(tmp.path()), None);
    }
}
