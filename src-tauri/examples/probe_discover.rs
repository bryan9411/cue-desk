//! 終端機探索腳本探針。
//!
//! 執行 iTerm2 與 Terminal.app 的 discover 腳本，以表格印出結果並記錄耗時，
//! 同時在執行前後檢查 App 是否在執行，確認腳本不會把未執行的 App 啟動。
//!
//! ```sh
//! cargo run --manifest-path src-tauri/Cargo.toml --example probe_discover
//! ```

use std::time::{Duration, Instant};

use cue_lib::osascript;
use cue_lib::terminal::scripts::{ITERM2_DISCOVER, TERMINAL_APP_DISCOVER};

/// 每支腳本連續執行的次數（第一次含腳本編譯與字典載入成本）。
const ROUNDS: usize = 3;

/// 以 argv 傳入 Bundle ID，查詢 App 是否在執行（不會啟動 App）。
const IS_RUNNING: &str = "on run argv\n\
    return (application id (item 1 of argv) is running) as text\n\
    end run\n";

struct Target {
    label: &'static str,
    bundle_id: &'static str,
    script: &'static str,
    headers: &'static [&'static str],
}

const TARGETS: [Target; 2] = [
    Target {
        label: "iTerm2",
        bundle_id: "com.googlecode.iterm2",
        script: ITERM2_DISCOVER,
        headers: &[
            "win_id", "win#", "left", "top", "right", "bottom", "tab#", "unique_id", "tty",
            "name",
        ],
    },
    Target {
        label: "Terminal.app",
        bundle_id: "com.apple.Terminal",
        script: TERMINAL_APP_DISCOVER,
        headers: &[
            "win_id", "win#", "left", "top", "right", "bottom", "tab#", "tty", "custom_title",
        ],
    },
];

fn main() {
    for target in &TARGETS {
        probe(target);
        println!();
    }
}

fn probe(target: &Target) {
    println!("== {} ({}) ==", target.label, target.bundle_id);

    let before = is_running(target.bundle_id);
    println!("執行前 is running: {before}");

    let mut timings = Vec::with_capacity(ROUNDS);
    let mut last_output = None;
    for _ in 0..ROUNDS {
        let started = Instant::now();
        let result = osascript::run(target.script, &[]);
        timings.push(started.elapsed());
        match result {
            Ok(out) => last_output = Some(out),
            Err(e) => {
                println!("錯誤 [{}]：{e}", e.kind());
                break;
            }
        }
    }

    let after = is_running(target.bundle_id);
    println!("執行後 is running: {after}");
    if before == "false" && after == "true" {
        println!("警告：腳本執行後 App 被啟動了");
    }

    println!("耗時：{}", format_timings(&timings));

    if let Some(out) = last_output {
        let records = osascript::parse_records(&out);
        if records.is_empty() {
            println!("（沒有紀錄）");
        } else {
            print_table(target.headers, &records);
            println!("共 {} 筆", records.len());
        }
    }
}

fn is_running(bundle_id: &str) -> String {
    match osascript::run(IS_RUNNING, &[bundle_id]) {
        Ok(out) => out,
        Err(e) => format!("（查詢失敗：{e}）"),
    }
}

fn format_timings(timings: &[Duration]) -> String {
    timings
        .iter()
        .enumerate()
        .map(|(i, d)| format!("#{} {:.1} ms", i + 1, d.as_secs_f64() * 1000.0))
        .collect::<Vec<_>>()
        .join("、")
}

/// 以 `|` 分隔印出表格；欄寬以字元數估算（中文寬度不精確，僅供閱讀）。
fn print_table(headers: &[&str], records: &[Vec<String>]) {
    let columns = headers
        .len()
        .max(records.iter().map(Vec::len).max().unwrap_or(0));
    let cell = |row: &[String], i: usize| -> String {
        row.get(i).map(|s| visible(s)).unwrap_or_default()
    };

    let mut widths: Vec<usize> = (0..columns)
        .map(|i| headers.get(i).map_or(0, |h| h.chars().count()))
        .collect();
    for row in records {
        for (i, w) in widths.iter_mut().enumerate() {
            *w = (*w).max(cell(row, i).chars().count());
        }
    }

    let header_row: Vec<String> = (0..columns)
        .map(|i| headers.get(i).copied().unwrap_or("?").to_string())
        .collect();
    print_row(&header_row, &widths);
    let rule: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    print_row(&rule, &widths);
    for row in records {
        let cells: Vec<String> = (0..columns).map(|i| cell(row, i)).collect();
        print_row(&cells, &widths);
    }
    if records.iter().any(|r| r.len() != headers.len()) {
        println!("警告：有紀錄的欄位數與預期（{}）不符", headers.len());
    }
}

fn print_row(cells: &[String], widths: &[usize]) {
    let line = cells
        .iter()
        .zip(widths)
        .map(|(c, w)| format!("{c:<w$}"))
        .collect::<Vec<_>>()
        .join(" | ");
    println!("{}", line.trim_end());
}

/// 把控制字元換成可見符號，並截斷過長的欄位。
fn visible(s: &str) -> String {
    const MAX: usize = 40;
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect();
    if cleaned.chars().count() > MAX {
        let mut cut: String = cleaned.chars().take(MAX).collect();
        cut.push('…');
        cut
    } else {
        cleaned
    }
}
