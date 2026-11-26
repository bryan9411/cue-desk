//! Terminal.app 後端（AppleScript 引擎）。
//!
//! 探索與聚焦都以 `scripts/` 中的固定腳本執行，變動資料（視窗 ID、tty）只經由 argv 傳入。
//! 以 Cmd+T 開的原生分頁在 AppleScript 中是獨立的視窗（001 第 3.3 節），
//! 因此每個分頁的身分為「視窗 ID + 正規化後 tty」。

use crate::error::CueError;
use crate::model::{normalize_tty, Bounds, DiscoveredSession, SessionKey};
use crate::osascript;

use super::detect::{self, TerminalApp};
use super::{scripts, Capabilities, TerminalBackend};

/// 探索腳本每筆紀錄的欄位數（001 第 3.1 節）。
const DISCOVER_FIELDS: usize = 9;

/// 聚焦腳本找到分頁時的回傳值。
const FOCUS_OK: &str = "ok";

/// Terminal.app 後端。
#[derive(Debug, Default, Clone, Copy)]
pub struct TerminalAppBackend;

impl TerminalAppBackend {
    pub fn new() -> Self {
        Self
    }
}

impl TerminalBackend for TerminalAppBackend {
    fn id(&self) -> &'static str {
        "terminal"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            split: false,
            new_tab: false,
            type_text_no_enter: false,
            rename: false,
            close_pane: true,
            precise_tabs: false,
        }
    }

    fn is_running(&self) -> bool {
        detect::is_running(TerminalApp::TerminalApp)
    }

    fn discover(&self) -> Result<Vec<DiscoveredSession>, CueError> {
        // 未執行時不跑腳本，確保輪詢不會把 Terminal.app 啟動（腳本內另有一道防線）。
        if !self.is_running() {
            return Ok(Vec::new());
        }
        let output = osascript::run(scripts::TERMINAL_APP_DISCOVER, &[])?;
        Ok(parse_discover(&output))
    }

    fn focus(&self, key: &SessionKey) -> Result<(), CueError> {
        let SessionKey::TerminalApp { window_id, tty } = key else {
            return Err(CueError::ObjectNotFound);
        };
        // tty 必須已是正規化形式（ttysNNN），才組回腳本比對用的 /dev/ttysNNN。
        if *window_id <= 0 || normalize_tty(tty).as_deref() != Some(tty.as_str()) {
            return Err(CueError::ObjectNotFound);
        }
        let window_arg = window_id.to_string();
        let tty_arg = format!("/dev/{tty}");
        let output = osascript::run(scripts::TERMINAL_APP_FOCUS, &[&window_arg, &tty_arg])?;
        if output == FOCUS_OK {
            Ok(())
        } else {
            Err(CueError::ObjectNotFound)
        }
    }
}

/// 解析探索腳本的輸出。欄位數不符、數值無法解析或 tty 無效的紀錄會略過。
pub(crate) fn parse_discover(output: &str) -> Vec<DiscoveredSession> {
    osascript::parse_records(output)
        .iter()
        .filter_map(|fields| parse_row(fields))
        .collect()
}

fn parse_row(fields: &[String]) -> Option<DiscoveredSession> {
    if fields.len() != DISCOVER_FIELDS {
        return None;
    }
    let int = |i: usize| fields[i].trim().parse::<i32>().ok();

    let window_id = fields[0].trim().parse().ok()?;
    let window_index = fields[1].trim().parse().ok()?;
    let window_bounds = Bounds { left: int(2)?, top: int(3)?, right: int(4)?, bottom: int(5)? };
    let tab_index = fields[6].trim().parse().ok()?;
    let tty = normalize_tty(&fields[7])?;

    Some(DiscoveredSession {
        key: SessionKey::TerminalApp { window_id, tty: tty.clone() },
        tty,
        name: fields[8].clone(),
        window_id,
        window_index,
        window_bounds,
        tab_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FS: char = '\x1f';
    const RS: char = '\x1e';

    fn row(fields: &[&str]) -> String {
        fields.join(&FS.to_string())
    }

    fn output(rows: &[String]) -> String {
        rows.join(&RS.to_string())
    }

    /// notes-probe 第二輪的輸出：11955 是在 11920 按 Cmd+T 開的分頁，被回報為獨立視窗。
    fn probe_sample() -> String {
        output(&[
            row(&["11955", "1", "1627", "119", "2207", "540", "1", "/dev/ttys026", "admin@admindeMacBook-Air"]),
            row(&["11917", "2", "1470", "68", "2050", "453", "1", "/dev/ttys021", "admin@admindeMacBook-Air"]),
            row(&["11920", "3", "1627", "119", "2207", "540", "1", "/dev/ttys022", "admin@admindeMacBook-Air"]),
        ]) + "\n"
    }

    #[test]
    fn parses_probe_output() {
        let sessions = parse_discover(&probe_sample());
        assert_eq!(sessions.len(), 3);

        let first = &sessions[0];
        assert_eq!(first.key, SessionKey::TerminalApp { window_id: 11955, tty: "ttys026".into() });
        assert_eq!(first.key.as_id(), "terminal:11955:ttys026");
        assert_eq!(first.tty, "ttys026");
        assert_eq!(first.name, "admin@admindeMacBook-Air");
        assert_eq!(first.window_id, 11955);
        assert_eq!(first.window_index, 1);
        assert_eq!(first.window_bounds, Bounds { left: 1627, top: 119, right: 2207, bottom: 540 });
        assert_eq!(first.tab_index, 1);

        let last = &sessions[2];
        assert_eq!(last.window_id, 11920);
        assert_eq!(last.window_index, 3);
        assert_eq!(last.tty, "ttys022");
    }

    #[test]
    fn native_tabs_are_separate_sessions_with_same_bounds() {
        let sessions = parse_discover(&probe_sample());
        let group: Vec<_> = sessions
            .iter()
            .filter(|s| s.window_bounds == Bounds { left: 1627, top: 119, right: 2207, bottom: 540 })
            .collect();
        assert_eq!(group.len(), 2);
        assert_ne!(group[0].key, group[1].key);
        assert!(group.iter().all(|s| s.tab_index == 1));
    }

    #[test]
    fn multiple_tabs_in_one_window_are_keyed_by_tty() {
        let out = output(&[
            row(&["5", "1", "0", "0", "10", "10", "1", "/dev/ttys001", ""]),
            row(&["5", "1", "0", "0", "10", "10", "2", "/dev/ttys002", "建置"]),
        ]);
        let sessions = parse_discover(&out);
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].key, SessionKey::TerminalApp { window_id: 5, tty: "ttys001".into() });
        assert_eq!(sessions[1].key, SessionKey::TerminalApp { window_id: 5, tty: "ttys002".into() });
        assert_eq!(sessions[1].tab_index, 2);
        assert_eq!(sessions[1].name, "建置");
    }

    #[test]
    fn empty_output_yields_no_sessions() {
        assert!(parse_discover("").is_empty());
        assert!(parse_discover("\n").is_empty());
    }

    #[test]
    fn skips_malformed_rows() {
        let out = output(&[
            // 欄位不足
            row(&["1", "1", "0", "0", "10", "10", "1", "/dev/ttys001"]),
            // 欄位過多（iTerm2 形式的 10 欄）
            row(&["1", "1", "0", "0", "10", "10", "1", "AAAA-2", "/dev/ttys002", "n"]),
            // window id 不是數字
            row(&["x", "1", "0", "0", "10", "10", "1", "/dev/ttys003", "n"]),
            // bounds 不是數字
            row(&["1", "1", "a", "0", "10", "10", "1", "/dev/ttys004", "n"]),
            // tty 無效
            row(&["1", "1", "0", "0", "10", "10", "1", "", "n"]),
            row(&["1", "1", "0", "0", "10", "10", "1", "/dev/console", "n"]),
            // 正常
            row(&["1", "1", "-1440", "-20", "0", "800", "1", "/dev/ttys005", "n"]),
        ]);
        let sessions = parse_discover(&out);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].tty, "ttys005");
        assert_eq!(sessions[0].window_bounds.left, -1440);
    }

    #[test]
    fn focus_rejects_foreign_or_invalid_keys_without_running_script() {
        let backend = TerminalAppBackend::new();
        let foreign = SessionKey::Iterm2 { unique_id: "ABC".into() };
        assert_eq!(backend.focus(&foreign), Err(CueError::ObjectNotFound));
        for (window_id, tty) in [
            (1, "/dev/ttys001"),
            (1, "ttys001\" & quit"),
            (1, "ttys"),
            (1, ""),
            (0, "ttys001"),
            (-5, "ttys001"),
        ] {
            let key = SessionKey::TerminalApp { window_id, tty: tty.into() };
            assert_eq!(backend.focus(&key), Err(CueError::ObjectNotFound), "{key}");
        }
    }

    #[test]
    fn capabilities_match_applescript_engine() {
        let caps = TerminalAppBackend::new().capabilities();
        assert!(!caps.split && !caps.new_tab && !caps.type_text_no_enter && !caps.rename);
        assert!(caps.close_pane);
        assert!(!caps.precise_tabs);
    }

    /// 本機手動：`cargo test -- --ignored terminal_app --nocapture`
    /// Terminal.app 執行中時列出所有分頁並記錄耗時，再依序聚焦每個分頁。
    #[test]
    #[ignore = "操作本機 Terminal.app"]
    fn terminal_app_discover_and_focus_local() {
        use std::time::Instant;

        let backend = TerminalAppBackend::new();
        if !backend.is_running() {
            assert!(backend.discover().unwrap().is_empty());
            assert!(!backend.is_running(), "探索不得啟動 Terminal.app");
            println!("Terminal.app 未執行：探索回傳空清單，且未啟動 Terminal.app");
            return;
        }

        let started = Instant::now();
        let sessions = backend.discover().expect("探索失敗");
        println!("探索 {} 個分頁，耗時 {:?}", sessions.len(), started.elapsed());
        for s in &sessions {
            println!(
                "{} | 視窗 {} (#{}) | 分頁 {} | {} | {}",
                s.key, s.window_id, s.window_index, s.tab_index, s.tty, s.name
            );
        }

        for s in &sessions {
            backend.focus(&s.key).expect("聚焦失敗");
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        let missing = SessionKey::TerminalApp { window_id: i64::from(i32::MAX), tty: "ttys999".into() };
        assert_eq!(backend.focus(&missing), Err(CueError::ObjectNotFound));
    }
}
