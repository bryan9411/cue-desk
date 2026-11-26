//! 以 libproc 取得程序的工作目錄。
//!
//! 透過 `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO)` 讀 `pvi_cdir.vip_path`，
//! 不使用 `lsof`，也不引入 `libproc` crate。

use std::collections::HashMap;
use std::ffi::{CStr, OsStr};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;

/// 批次取得多支程序的 cwd。
///
/// 只應傳入前景程序群組的領頭 pid，一輪探索呼叫一次。任何 pid 失敗都只略過該 pid：
/// - `EPERM`：非本使用者程序，無法取得 cwd，該工作階段歸為「未歸類」。
/// - `ESRCH`：程序已結束，屬正常競態，下一輪再試。
/// - 回傳大小與結構大小不符：視為無法取得。
pub fn cwds(pids: &[i32]) -> HashMap<i32, PathBuf> {
    pids.iter()
        .filter_map(|&pid| cwd_of(pid).ok().map(|path| (pid, path)))
        .collect()
}

/// 取得單一程序的 cwd。
fn cwd_of(pid: i32) -> io::Result<PathBuf> {
    if pid <= 0 {
        return Err(io::Error::from_raw_os_error(libc::ESRCH));
    }
    // SAFETY: proc_vnodepathinfo 為純 POD 結構，全零是合法值。
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
    // SAFETY: proc_pidinfo 最多寫入 buffersize（= 結構大小）位元組。
    let ret = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            (&mut info as *mut libc::proc_vnodepathinfo).cast(),
            size,
        )
    };
    if ret <= 0 {
        return Err(io::Error::last_os_error());
    }
    if ret != size {
        return Err(io::Error::other("proc_pidinfo 回傳大小不符"));
    }
    // libc 把 MAXPATHLEN 陣列拆成 [[c_char; 32]; 32]，攤平成連續位元組讀取。
    let path = &info.pvi_cdir.vip_path;
    // SAFETY: [[c_char; 32]; 32] 在記憶體中是連續 1024 位元組，生命週期受 info 約束。
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(path.as_ptr().cast::<u8>(), std::mem::size_of_val(path))
    };
    let cstr =
        CStr::from_bytes_until_nul(bytes).map_err(|_| io::Error::other("路徑沒有結尾 NUL"))?;
    let raw = cstr.to_bytes();
    if raw.is_empty() {
        return Err(io::Error::other("cwd 為空"));
    }
    Ok(PathBuf::from(OsStr::from_bytes(raw)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_process_cwd_matches_env() {
        let pid = i32::try_from(std::process::id()).unwrap();
        let map = cwds(&[pid]);
        let got = map.get(&pid).expect("應取得目前程序的 cwd");
        let expected = std::env::current_dir().unwrap();
        // current_dir 可能經過 symlink（例如 /tmp → /private/tmp），兩者皆以 realpath 比較
        assert_eq!(
            std::fs::canonicalize(got).unwrap(),
            std::fs::canonicalize(expected).unwrap()
        );
    }

    #[test]
    fn missing_pid_is_skipped() {
        // pid 上限為 99998，此值不會存在
        assert!(cwds(&[i32::MAX]).is_empty());
        assert!(cwds(&[0, -1]).is_empty());
        assert!(cwds(&[]).is_empty());
    }

    #[test]
    fn failures_do_not_drop_other_pids() {
        let pid = i32::try_from(std::process::id()).unwrap();
        let map = cwds(&[i32::MAX, pid, 1]);
        assert!(map.contains_key(&pid));
        assert!(!map.contains_key(&i32::MAX));
    }

    /// `cargo test --manifest-path src-tauri/Cargo.toml process::cwd -- --ignored --nocapture`
    #[test]
    #[ignore = "需要本機開啟的終端機分頁"]
    fn local_foreground_shells() {
        let procs = crate::process::snapshot().expect("ps 執行失敗");
        let fg = crate::process::foreground_by_tty(&procs);
        let pids: Vec<i32> = fg.values().map(|f| f.pid).collect();
        let map = cwds(&pids);
        let mut ttys: Vec<_> = fg.iter().collect();
        ttys.sort_by(|a, b| a.0.cmp(b.0));
        for (tty, f) in ttys {
            println!("{tty}\t{}\t{:?}\t{:?}", f.pid, f.kind, map.get(&f.pid));
        }
        assert!(!pids.is_empty(), "沒有任何前景程序，請先開終端機分頁");
        assert!(!map.is_empty(), "應至少取得一支前景程序的 cwd");
    }
}
