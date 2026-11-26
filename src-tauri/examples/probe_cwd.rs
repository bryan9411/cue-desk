//! 前景程式工作目錄取得方式探針。
//!
//! 執行一次 `ps` 找出每個 tty 的前景程序群組（`pgid == tpgid`），再對這些 pid
//! 分別以 libproc（`proc_pidinfo` + `PROC_PIDVNODEPATHINFO`）與批次 `lsof`
//! 取得 cwd，比較兩者結果與耗時。最後以 pid 1（launchd，屬 root）示範
//! 非本使用者程序的錯誤處理。
//!
//! ```sh
//! cargo run --manifest-path src-tauri/Cargo.toml --example probe_cwd
//! ```

use std::collections::HashMap;
use std::ffi::CStr;
use std::io;
use std::process::Command;
use std::time::{Duration, Instant};

/// 每種方式連續執行的次數（第一次可能含快取暖機成本）。
const ROUNDS: usize = 3;

/// `args` 欄位最多顯示的字元數。
const ARGS_WIDTH: usize = 60;

/// 示範非本使用者程序的 pid（launchd，屬 root）。
const FOREIGN_PID: i32 = 1;

struct Proc {
    pid: i32,
    pgid: i32,
    tpgid: i32,
    tty: String,
    args: String,
}

fn main() {
    let (procs, ps_elapsed) = match run_ps() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("ps 執行失敗：{e}");
            std::process::exit(1);
        }
    };

    // 前景程序群組：有 tty 且 pgid == tpgid
    let mut fg: Vec<&Proc> = procs
        .iter()
        .filter(|p| p.tty != "??" && p.tpgid > 0 && p.pgid == p.tpgid)
        .collect();
    fg.sort_by(|a, b| a.tty.cmp(&b.tty).then(a.pid.cmp(&b.pid)));
    let pids: Vec<i32> = fg.iter().map(|p| p.pid).collect();

    println!(
        "ps：{} 支程序，前景 {} 支（{} 個 tty），耗時 {}",
        procs.len(),
        fg.len(),
        count_ttys(&fg),
        fmt_dur(ps_elapsed)
    );
    if pids.is_empty() {
        println!("沒有任何 tty 的前景程序，請先開幾個終端機分頁再執行。");
        return;
    }

    // ── libproc ──
    let mut libproc_result: HashMap<i32, Result<String, String>> = HashMap::new();
    let mut libproc_times = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        let start = Instant::now();
        libproc_result = pids
            .iter()
            .map(|&pid| (pid, cwd_libproc(pid).map_err(|e| e.to_string())))
            .collect();
        libproc_times.push(start.elapsed());
    }

    // ── lsof（批次）──
    let mut lsof_result: Result<HashMap<i32, String>, String> = Ok(HashMap::new());
    let mut lsof_stderr = String::new();
    let mut lsof_times = Vec::with_capacity(ROUNDS);
    for _ in 0..ROUNDS {
        let start = Instant::now();
        match cwd_lsof(&pids) {
            Ok((map, stderr)) => {
                lsof_result = Ok(map);
                lsof_stderr = stderr;
            }
            Err(e) => lsof_result = Err(e.to_string()),
        }
        lsof_times.push(start.elapsed());
    }

    // ── 表格 ──
    let headers = ["pid", "tty", "kind", "args", "libproc", "lsof", "same"];
    let rows: Vec<Vec<String>> = fg
        .iter()
        .map(|p| {
            let lp = libproc_result
                .get(&p.pid)
                .cloned()
                .unwrap_or_else(|| Err("（無結果）".into()));
            let ls = match &lsof_result {
                Ok(map) => map
                    .get(&p.pid)
                    .cloned()
                    .ok_or_else(|| "（無輸出）".to_string()),
                Err(e) => Err(e.clone()),
            };
            let same = match (&lp, &ls) {
                (Ok(a), Ok(b)) if a == b => "=",
                (Ok(_), Ok(_)) => "DIFF",
                _ => "-",
            };
            vec![
                p.pid.to_string(),
                p.tty.clone(),
                guess_kind(&p.args).to_string(),
                truncate(&p.args, ARGS_WIDTH),
                show(&lp),
                show(&ls),
                same.to_string(),
            ]
        })
        .collect();
    print_table(&headers, &rows);

    println!();
    println!(
        "libproc：{} 支 pid，各輪耗時 {}（平均每支 {}）",
        pids.len(),
        fmt_times(&libproc_times),
        fmt_dur(min_dur(&libproc_times) / pids.len() as u32)
    );
    println!(
        "lsof   ：{} 支 pid 一次執行，各輪耗時 {}",
        pids.len(),
        fmt_times(&lsof_times)
    );
    if !lsof_stderr.trim().is_empty() {
        println!("lsof stderr：{}", lsof_stderr.trim().replace('\n', " | "));
    }

    // ── 非本使用者程序 ──
    println!();
    println!("非本使用者程序（pid {FOREIGN_PID}）：");
    match cwd_libproc(FOREIGN_PID) {
        Ok(cwd) => println!("  libproc：{cwd}"),
        Err(e) => println!(
            "  libproc：錯誤 {e}（errno {}）",
            e.raw_os_error().unwrap_or(0)
        ),
    }
    match cwd_lsof(&[FOREIGN_PID]) {
        Ok((map, stderr)) => println!(
            "  lsof   ：{}；stderr：{}",
            map.get(&FOREIGN_PID)
                .map(String::as_str)
                .unwrap_or("（無輸出）"),
            if stderr.trim().is_empty() {
                "（無）".to_string()
            } else {
                stderr.trim().replace('\n', " | ")
            }
        ),
        Err(e) => println!("  lsof   ：執行失敗 {e}"),
    }
}

/// 執行一次 `ps` 並解析。`-ww` 讓 `args` 不受欄寬截斷。
fn run_ps() -> io::Result<(Vec<Proc>, Duration)> {
    let start = Instant::now();
    let out = Command::new("/bin/ps")
        .args(["-A", "-ww", "-o", "pid=,ppid=,pgid=,tpgid=,tty=,args="])
        .output()?;
    let elapsed = start.elapsed();
    if !out.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let procs = text.lines().filter_map(parse_ps_line).collect();
    Ok((procs, elapsed))
}

/// 解析一行 `pid ppid pgid tpgid tty args…`；`args` 可含空白，取剩餘全部。
fn parse_ps_line(line: &str) -> Option<Proc> {
    let mut rest = line.trim_start();
    let mut fields = [""; 5];
    for slot in &mut fields {
        let end = rest.find(char::is_whitespace)?;
        *slot = &rest[..end];
        rest = rest[end..].trim_start();
    }
    // ppid 本探針用不到，只確認可解析
    let _ppid: i32 = fields[1].parse().ok()?;
    Some(Proc {
        pid: fields[0].parse().ok()?,
        pgid: fields[2].parse().ok()?,
        tpgid: fields[3].parse().ok()?,
        tty: fields[4].to_string(),
        args: rest.to_string(),
    })
}

/// 以 `proc_pidinfo(PROC_PIDVNODEPATHINFO)` 取 `pvi_cdir`。
fn cwd_libproc(pid: i32) -> io::Result<String> {
    // SAFETY: proc_vnodepathinfo 為純 POD 結構，全零是合法值；
    // proc_pidinfo 最多寫入 buffersize 位元組。
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
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
        return Err(io::Error::other(format!("回傳大小 {ret} ≠ {size}")));
    }
    // libc 為了相容舊版 rustc 把 MAXPATHLEN 的陣列拆成 [[c_char; 32]; 32]，攤平成位元組讀取。
    let path = &info.pvi_cdir.vip_path;
    // SAFETY: [[c_char; 32]; 32] 在記憶體中是連續 1024 位元組。
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(path.as_ptr().cast::<u8>(), std::mem::size_of_val(path))
    };
    let cstr = CStr::from_bytes_until_nul(bytes)
        .map_err(|_| io::Error::other("路徑沒有結尾 NUL"))?;
    Ok(cstr.to_string_lossy().into_owned())
}

/// 以 `lsof -a -p <pids> -d cwd -Fn` 批次取得 cwd，回傳 pid → cwd 與 stderr。
/// 部分 pid 失敗時 lsof 的結束碼非 0，但 stdout 仍有其餘結果，因此不以結束碼判斷。
fn cwd_lsof(pids: &[i32]) -> io::Result<(HashMap<i32, String>, String)> {
    let list = pids
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let out = Command::new("/usr/sbin/lsof")
        .args(["-a", "-p", &list, "-d", "cwd", "-Fn"])
        .output()?;
    let mut map = HashMap::new();
    let mut current: Option<i32> = None;
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        match line.split_at_checked(1) {
            Some(("p", v)) => current = v.parse().ok(),
            Some(("n", v)) => {
                if let Some(pid) = current {
                    map.insert(pid, v.to_string());
                }
            }
            _ => {}
        }
    }
    Ok((map, String::from_utf8_lossy(&out.stderr).into_owned()))
}

/// 粗略判斷類型，只為了方便觀察表格（正式規則在後續的 ps 解析實作）。
fn guess_kind(args: &str) -> &'static str {
    let exe = args.split_whitespace().next().unwrap_or("");
    let base = exe.rsplit('/').next().unwrap_or(exe).trim_start_matches('-');
    if args.contains("gemini") {
        "gemini"
    } else if args.contains("claude") {
        if base == "node" {
            "claude(npm)"
        } else {
            "claude"
        }
    } else if matches!(base, "zsh" | "bash" | "sh" | "fish") {
        "shell"
    } else {
        "other"
    }
}

fn count_ttys(fg: &[&Proc]) -> usize {
    let mut ttys: Vec<&str> = fg.iter().map(|p| p.tty.as_str()).collect();
    ttys.dedup();
    ttys.len()
}

fn show(r: &Result<String, String>) -> String {
    match r {
        Ok(v) => v.clone(),
        Err(e) => format!("ERR {e}"),
    }
}

fn truncate(s: &str, width: usize) -> String {
    if s.chars().count() <= width {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(width - 1).collect();
        t.push('…');
        t
    }
}

fn min_dur(times: &[Duration]) -> Duration {
    times.iter().copied().min().unwrap_or_default()
}

fn fmt_dur(d: Duration) -> String {
    let us = d.as_micros();
    if us >= 1000 {
        format!("{:.1} ms", us as f64 / 1000.0)
    } else {
        format!("{us} µs")
    }
}

fn fmt_times(times: &[Duration]) -> String {
    times.iter().map(|&d| fmt_dur(d)).collect::<Vec<_>>().join(" / ")
}

fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for row in rows {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.chars().count());
        }
    }
    let line = |cells: Vec<&str>| {
        cells
            .iter()
            .zip(&widths)
            .map(|(c, &w)| format!("{c}{}", " ".repeat(w.saturating_sub(c.chars().count()))))
            .collect::<Vec<_>>()
            .join("  ")
            .trim_end()
            .to_string()
    };
    println!("{}", line(headers.to_vec()));
    println!(
        "{}",
        widths.iter().map(|&w| "-".repeat(w)).collect::<Vec<_>>().join("  ")
    );
    for row in rows {
        println!("{}", line(row.iter().map(String::as_str).collect()));
    }
}
