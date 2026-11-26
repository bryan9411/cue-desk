//! Terminal.app 後端（AppleScript 引擎）。
//!
//! 探索與聚焦都以 `scripts/` 中的固定腳本執行，變動資料（視窗 ID、tty）只經由 argv 傳入。
//! 以 Cmd+T 開的原生分頁在 AppleScript 中是獨立的視窗，
//! 因此每個分頁的身分為「視窗 ID + 正規化後 tty」。
//! 建立工作階段只支援開新視窗：AppleScript 無法在既有視窗開分頁或分割，
//! 就地補開 zsh 也因此改開同工作區的新視窗。
//! 關閉分頁則是關閉該分頁所屬的 AppleScript 視窗。

use std::path::Path;
use std::time::Duration;

use crate::error::CueError;
use crate::model::{normalize_tty, Bounds, DiscoveredSession, SessionKey};
use crate::osascript;
use crate::shell;

use super::detect::{self, TerminalApp};
use super::{
    scripts, Capabilities, CreateSpec, CreatedSession, OpenMode, SplitDirection, TerminalBackend,
};

/// 探索腳本每筆紀錄的欄位數。
const DISCOVER_FIELDS: usize = 9;

/// 聚焦與關閉腳本找到分頁時的回傳值。
const SCRIPT_OK: &str = "ok";

/// 關閉腳本發現視窗不只 1 個分頁、因此未關閉時的回傳值。
const CLOSE_MULTI_TABS: &str = "multi";

/// 建立工作階段的逾時：Terminal.app 未執行時要先冷啟動，且腳本會等新分頁取得 tty。
const CREATE_TIMEOUT: Duration = Duration::from_secs(20);

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
        if output == SCRIPT_OK {
            Ok(())
        } else {
            Err(CueError::ObjectNotFound)
        }
    }

    fn create(&self, spec: &CreateSpec) -> Result<CreatedSession, CueError> {
        if spec.mode != OpenMode::NewWindow {
            return Err(CueError::Unsupported);
        }
        // 路徑不安全時在執行腳本（啟動 Terminal.app）之前就拒絕。
        let command = shell::launch_command(spec.kind, &spec.workspace_path)?;
        let output = osascript::run_with_timeout(scripts::TERMINAL_APP_CREATE, &[&command], CREATE_TIMEOUT)?;
        parse_created(&output)
    }

    /// 無法分割，改開新視窗並 `cd` 到工作區；方向不影響結果。
    fn split_zsh(&self, key: &SessionKey, _direction: SplitDirection, cwd: &Path) -> Result<(), CueError> {
        if !matches!(key, SessionKey::TerminalApp { .. }) {
            return Err(CueError::ObjectNotFound);
        }
        let command = shell::cd_command(cwd)?;
        let output = osascript::run_with_timeout(scripts::TERMINAL_APP_CREATE, &[&command], CREATE_TIMEOUT)?;
        parse_created(&output).map(|_| ())
    }

    /// 關閉分頁。原生分頁在 AppleScript 中各自是一個視窗，關閉該視窗只會關閉這個分頁；
    /// 視窗不只 1 個分頁時拒絕，避免連同其他分頁一起關閉。不切換焦點；
    /// Terminal.app 未執行時回傳 `ObjectNotFound`。
    fn close(&self, key: &SessionKey) -> Result<(), CueError> {
        let SessionKey::TerminalApp { window_id, tty } = key else {
            return Err(CueError::ObjectNotFound);
        };
        if *window_id <= 0 || normalize_tty(tty).as_deref() != Some(tty.as_str()) {
            return Err(CueError::ObjectNotFound);
        }
        let window_arg = window_id.to_string();
        let tty_arg = format!("/dev/{tty}");
        let output = osascript::run(scripts::TERMINAL_APP_CLOSE, &[&window_arg, &tty_arg])?;
        parse_close(&output)
    }
}

/// 解析關閉腳本的回傳值。
fn parse_close(output: &str) -> Result<(), CueError> {
    match output {
        SCRIPT_OK => Ok(()),
        CLOSE_MULTI_TABS => Err(CueError::Script("Terminal.app 無法單獨關閉分頁".into())),
        _ => Err(CueError::ObjectNotFound),
    }
}

/// 解析建立腳本回傳的「window id ␟ tty」。
fn parse_created(output: &str) -> Result<CreatedSession, CueError> {
    let invalid = || CueError::Script("無法辨識 Terminal.app 新分頁的視窗與 tty".into());
    let (window, tty) = output.trim_end_matches('\n').split_once(osascript::FIELD_SEP).ok_or_else(invalid)?;
    let window_id = window.trim().parse::<i64>().ok().filter(|id| *id > 0).ok_or_else(invalid)?;
    let tty = normalize_tty(tty).ok_or_else(invalid)?;
    Ok(CreatedSession { key: SessionKey::TerminalApp { window_id, tty }, zsh: None })
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
    use crate::shell::SessionKindToCreate;

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

    #[test]
    fn parses_created_window_and_tty() {
        let created = parse_created(&row(&["11955", "/dev/ttys026"])).unwrap();
        assert_eq!(created.key, SessionKey::TerminalApp { window_id: 11955, tty: "ttys026".into() });
        assert_eq!(created.zsh, None);
        assert_eq!(parse_created(&(row(&["7", "/dev/ttys001"]) + "\n")).unwrap().key.as_id(), "terminal:7:ttys001");
    }

    #[test]
    fn rejects_unexpected_create_output() {
        for out in [
            String::new(),
            "11955".into(),
            row(&["11955", ""]),
            row(&["x", "/dev/ttys001"]),
            row(&["0", "/dev/ttys001"]),
            row(&["-3", "/dev/ttys001"]),
            row(&["5", "/dev/console"]),
            row(&["5", "/dev/ttys001", "extra"]),
        ] {
            assert!(matches!(parse_created(&out), Err(CueError::Script(_))), "應拒絕 {out:?}");
        }
    }

    #[test]
    fn create_supports_only_new_window_and_rejects_unsafe_path() {
        let backend = TerminalAppBackend::new();
        let spec = |mode, path: &str| CreateSpec {
            workspace_path: path.into(),
            kind: SessionKindToCreate::Claude,
            label: None,
            mode,
            target_window: Some(1),
        };
        for mode in [OpenMode::NewTab, OpenMode::AiTopZshBottom, OpenMode::AiLeftZshRight] {
            assert_eq!(backend.create(&spec(mode, "/tmp")), Err(CueError::Unsupported), "{mode:?}");
        }
        assert_eq!(backend.create(&spec(OpenMode::NewWindow, "/tmp/a\nrm -rf ~")), Err(CueError::UnsafePath));
    }

    #[test]
    fn split_zsh_rejects_bad_input_without_running_script() {
        let backend = TerminalAppBackend::new();
        let foreign = SessionKey::Iterm2 { unique_id: "ABC".into() };
        assert_eq!(
            backend.split_zsh(&foreign, SplitDirection::Right, Path::new("/tmp")),
            Err(CueError::ObjectNotFound)
        );
        let key = SessionKey::TerminalApp { window_id: 1, tty: "ttys001".into() };
        assert_eq!(
            backend.split_zsh(&key, SplitDirection::Below, Path::new("/tmp/a\nrm -rf ~")),
            Err(CueError::UnsafePath)
        );
    }

    #[test]
    fn close_rejects_foreign_or_invalid_keys_without_running_script() {
        let backend = TerminalAppBackend::new();
        let foreign = SessionKey::Iterm2 { unique_id: "ABC".into() };
        assert_eq!(backend.close(&foreign), Err(CueError::ObjectNotFound));
        for (window_id, tty) in [(1, "/dev/ttys001"), (1, "ttys001\" & quit"), (1, ""), (0, "ttys001"), (-5, "ttys001")] {
            let key = SessionKey::TerminalApp { window_id, tty: tty.into() };
            assert_eq!(backend.close(&key), Err(CueError::ObjectNotFound), "{key}");
        }
    }

    #[test]
    fn close_refuses_windows_with_several_tabs() {
        assert_eq!(parse_close("ok"), Ok(()));
        assert_eq!(parse_close(""), Err(CueError::ObjectNotFound));
        assert_eq!(parse_close("unexpected"), Err(CueError::ObjectNotFound));
        let Err(CueError::Script(msg)) = parse_close("multi") else {
            panic!("多分頁視窗應回傳錯誤");
        };
        assert_eq!(msg, "Terminal.app 無法單獨關閉分頁");
    }

    /// 本機手動：在 Terminal.app 以 Cmd+T 開 3 個分頁，在中間那個執行 `tty`，再執行
    /// `CUE_CLOSE_TTY=ttysNNN cargo test -- --ignored terminal_app_close --nocapture`。
    /// 確認只有該分頁被關閉，同群組其他分頁仍在。
    #[test]
    #[ignore = "操作本機 Terminal.app"]
    fn terminal_app_close_local() {
        let tty = std::env::var("CUE_CLOSE_TTY").expect("請以 CUE_CLOSE_TTY 指定要關閉的 tty（例如 ttys012）");
        let backend = TerminalAppBackend::new();
        let before = backend.discover().expect("探索失敗");
        let target = before.iter().find(|s| s.tty == tty).unwrap_or_else(|| panic!("探索不到 {tty}"));
        for s in &before {
            println!("關閉前：{} | 視窗 #{} | {}", s.key, s.window_index, s.tty);
        }

        backend.close(&target.key).expect("關閉失敗");
        std::thread::sleep(std::time::Duration::from_millis(800));
        let after: Vec<_> = backend.discover().expect("探索失敗").into_iter().map(|s| s.key).collect();
        assert!(!after.contains(&target.key), "分頁 {} 仍存在", target.key);
        for s in before.iter().filter(|s| s.key != target.key) {
            assert!(after.contains(&s.key), "其他分頁 {} 被一併關閉", s.key);
        }
        println!("只關閉了 {}，其餘 {} 個分頁不受影響", target.key, after.len());
    }

    /// 本機手動：`cargo test -- --ignored terminal_app_create --nocapture`
    /// 在含注入字元的暫存工作區開新視窗建立 zsh 工作階段，檢查探索結果，
    /// 且路徑中的 `$(touch /tmp/pwned)` 沒有被執行。新視窗的目錄需目視確認（`pwd`）。
    #[test]
    #[ignore = "操作本機 Terminal.app"]
    fn terminal_app_create_local() {
        use std::path::Path;
        use std::time::Duration;

        let pwned = Path::new("/tmp/pwned");
        let _ = std::fs::remove_file(pwned);
        assert!(!pwned.exists(), "測試前無法移除 /tmp/pwned");

        // 保留暫存資料夾，新視窗 cd 進去後才能目視確認目錄。
        let ws = tempfile::tempdir().unwrap().keep().join("A's 專案 $(touch /tmp/pwned) \"x\"");
        std::fs::create_dir_all(&ws).unwrap();

        let backend = TerminalAppBackend::new();
        let spec = CreateSpec {
            workspace_path: ws.clone(),
            kind: SessionKindToCreate::Zsh,
            label: None,
            mode: OpenMode::NewWindow,
            target_window: None,
        };
        let created = backend.create(&spec).expect("建立失敗");
        assert_eq!(created.zsh, None);

        std::thread::sleep(Duration::from_millis(500));
        let found = backend
            .discover()
            .expect("探索失敗")
            .into_iter()
            .find(|s| s.key == created.key)
            .unwrap_or_else(|| panic!("探索不到新分頁 {}", created.key));
        println!("新視窗：{} | 視窗 #{} | {}", found.key, found.window_index, found.tty);

        std::thread::sleep(Duration::from_secs(2));
        assert!(!pwned.exists(), "注入的指令被執行了");
        println!("工作區：{}", ws.display());
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
