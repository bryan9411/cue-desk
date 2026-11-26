//! osascript 執行器與分隔字元解析。
//!
//! 防注入規則：
//! - 腳本原文只能是程式內固定的字串（參數型別為 `&'static str`），從 stdin 寫入 osascript。
//! - 變動資料（路徑、名稱、ID）一律放在 argv，由腳本的 `on run argv` 取用。
//!
//! 腳本回傳結構化資料時，欄位以 `\x1f`（Unit Separator）、紀錄以 `\x1e`（Record Separator）分隔。

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::CueError;

/// 欄位分隔字元（ASCII Unit Separator）。
pub const FIELD_SEP: char = '\x1f';
/// 紀錄分隔字元（ASCII Record Separator）。
pub const RECORD_SEP: char = '\x1e';
/// 預設逾時。
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

const OSASCRIPT: &str = "/usr/bin/osascript";
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// 以預設逾時執行固定腳本，回傳 stdout（已去掉結尾換行）。
///
/// 此函式會阻塞；在 async 環境中請以 `tokio::task::spawn_blocking` 呼叫。
pub fn run(script: &'static str, args: &[&str]) -> Result<String, CueError> {
    run_with_timeout(script, args, DEFAULT_TIMEOUT)
}

/// 以指定逾時執行固定腳本。逾時會終止子程序並回傳 [`CueError::Timeout`]。
pub fn run_with_timeout(
    script: &'static str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, CueError> {
    // `osascript -` 表示從 stdin 讀取腳本，其後的參數全部交給 `on run argv`。
    let mut child = Command::new(OSASCRIPT)
        .arg("-")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CueError::Io(e.to_string()))?;

    // 先開始讀取 stdout／stderr，避免輸出塞滿管線造成死結。
    let stdout_reader = spawn_reader(child.stdout.take());
    let stderr_reader = spawn_reader(child.stderr.take());

    if let Err(e) = write_script(&mut child, script) {
        kill(&mut child);
        return Err(e);
    }

    let status = wait_with_timeout(&mut child, timeout)?;
    let stdout = join_reader(stdout_reader);
    let stderr = join_reader(stderr_reader);

    if status.success() {
        Ok(trim_trailing_newlines(&stdout).to_owned())
    } else {
        Err(CueError::from_osascript_stderr(&stderr))
    }
}

/// 以 `\x1e` 切紀錄、`\x1f` 切欄位。會去掉結尾換行；空輸出回傳空陣列。
/// 空白紀錄（例如腳本在每筆紀錄後都加上 `\x1e` 造成的結尾空段）會略過。
pub fn parse_records(output: &str) -> Vec<Vec<String>> {
    trim_trailing_newlines(output)
        .split(RECORD_SEP)
        .filter(|record| !record.is_empty())
        .map(|record| record.split(FIELD_SEP).map(str::to_owned).collect())
        .collect()
}

fn trim_trailing_newlines(s: &str) -> &str {
    s.trim_end_matches(['\n', '\r'])
}

fn write_script(child: &mut Child, script: &'static str) -> Result<(), CueError> {
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| CueError::Io("無法取得 stdin".into()))?;
    stdin
        .write_all(script.as_bytes())
        .map_err(|e| CueError::Io(e.to_string()))?;
    // drop 時關閉 stdin，osascript 才會開始編譯執行。
    Ok(())
}

fn wait_with_timeout(
    child: &mut Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, CueError> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() >= deadline => {
                kill(child);
                return Err(CueError::Timeout);
            }
            Ok(None) => thread::sleep(POLL_INTERVAL),
            Err(e) => {
                kill(child);
                return Err(CueError::Io(e.to_string()));
            }
        }
    }
}

fn kill(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn spawn_reader<R: Read + Send + 'static>(pipe: Option<R>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut buf);
        }
        String::from_utf8_lossy(&buf).into_owned()
    })
}

fn join_reader(handle: thread::JoinHandle<String>) -> String {
    handle.join().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_output_is_empty() {
        assert!(parse_records("").is_empty());
        assert!(parse_records("\n").is_empty());
    }

    #[test]
    fn single_record() {
        assert_eq!(
            parse_records("a\x1fb\x1fc\n"),
            vec![vec!["a".to_string(), "b".into(), "c".into()]]
        );
    }

    #[test]
    fn multiple_records() {
        assert_eq!(
            parse_records("1\x1fx\x1e2\x1fy\x1e3\x1fz"),
            vec![
                vec!["1".to_string(), "x".into()],
                vec!["2".into(), "y".into()],
                vec!["3".into(), "z".into()],
            ]
        );
    }

    #[test]
    fn trailing_record_separator_is_ignored() {
        assert_eq!(
            parse_records("1\x1fx\x1e2\x1fy\x1e\n"),
            vec![vec!["1".to_string(), "x".into()], vec!["2".into(), "y".into()]]
        );
    }

    #[test]
    fn fields_with_spaces_and_cjk_are_kept() {
        assert_eq!(
            parse_records("  我的 專案 \x1f/Users/me/Documents/客戶 A\x1f\n"),
            vec![vec![
                "  我的 專案 ".to_string(),
                "/Users/me/Documents/客戶 A".into(),
                "".into(),
            ]]
        );
    }

    const ECHO_SCRIPT: &str = "on run argv\n\
        return (item 1 of argv) & (ASCII character 31) & (item 2 of argv)\n\
        end run\n";

    #[test]
    #[ignore = "需要 macOS osascript"]
    fn osascript_round_trips_hostile_args() {
        let cases: [(&str, &str); 4] = [
            ("it's", "say \"hi\""),
            ("$(rm -rf /)", "`whoami`; echo x"),
            ("line1\nline2", "中文 路徑/客戶 A"),
            ("-e", "\" & (do shell script \"id\") & \""),
        ];
        for (a, b) in cases {
            let out = run(ECHO_SCRIPT, &[a, b]).expect("osascript 應執行成功");
            assert_eq!(parse_records(&out), vec![vec![a.to_string(), b.to_string()]]);
        }
    }

    #[test]
    #[ignore = "需要 macOS osascript"]
    fn osascript_timeout_kills_child() {
        const SLOW: &str = "on run argv\ndelay 3\nreturn \"done\"\nend run\n";
        let started = Instant::now();
        let err = run_with_timeout(SLOW, &[], Duration::from_millis(300)).unwrap_err();
        assert_eq!(err, CueError::Timeout);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    #[ignore = "需要 macOS osascript"]
    fn osascript_error_is_classified() {
        const BAD: &str = "on run argv\nerror \"boom\" number -1728\nend run\n";
        assert_eq!(run(BAD, &[]).unwrap_err(), CueError::ObjectNotFound);
    }
}
