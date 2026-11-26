//! iTerm2 後端（AppleScript 引擎）。
//!
//! 探索、聚焦與建立都以 `scripts/` 中的固定腳本執行，變動資料（unique ID、
//! 視窗 ID、`shell` 模組產生的指令）只經由 argv 傳入。

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
const DISCOVER_FIELDS: usize = 10;

/// unique ID 的最大長度；iTerm2 目前是 36 字元的 UUID，保留餘裕。
const UNIQUE_ID_MAX_LEN: usize = 64;

/// 聚焦腳本找到窗格時的回傳值。
const FOCUS_OK: &str = "ok";

/// 建立工作階段的逾時：iTerm2 未執行時要先冷啟動，常超過預設的 5 秒。
const CREATE_TIMEOUT: Duration = Duration::from_secs(20);

/// iTerm2 後端。
#[derive(Debug, Default, Clone, Copy)]
pub struct ITerm2Backend;

impl ITerm2Backend {
    pub fn new() -> Self {
        Self
    }
}

impl TerminalBackend for ITerm2Backend {
    fn id(&self) -> &'static str {
        "iterm2"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            split: true,
            new_tab: true,
            type_text_no_enter: true,
            rename: true,
            close_pane: true,
            precise_tabs: false,
        }
    }

    fn is_running(&self) -> bool {
        detect::is_running(TerminalApp::Iterm2)
    }

    fn discover(&self) -> Result<Vec<DiscoveredSession>, CueError> {
        // 未執行時不跑腳本，確保輪詢不會把 iTerm2 啟動（腳本內另有一道防線）。
        if !self.is_running() {
            return Ok(Vec::new());
        }
        let output = osascript::run(scripts::ITERM2_DISCOVER, &[])?;
        Ok(parse_discover(&output))
    }

    fn focus(&self, key: &SessionKey) -> Result<(), CueError> {
        let SessionKey::Iterm2 { unique_id } = key else {
            return Err(CueError::ObjectNotFound);
        };
        if !is_valid_unique_id(unique_id) {
            return Err(CueError::ObjectNotFound);
        }
        let output = osascript::run(scripts::ITERM2_FOCUS, &[unique_id.as_str()])?;
        if output == FOCUS_OK {
            Ok(())
        } else {
            Err(CueError::ObjectNotFound)
        }
    }

    fn create(&self, spec: &CreateSpec) -> Result<CreatedSession, CueError> {
        // 先產生所有指令：路徑不安全時在執行任何腳本（啟動 iTerm2）之前就拒絕。
        let command = shell::launch_command(spec.kind, &spec.workspace_path)?;
        let target = spec.target_window.map(|id| id.to_string()).unwrap_or_default();
        let run = |script, args: &[&str]| osascript::run_with_timeout(script, args, CREATE_TIMEOUT);

        let (output, with_zsh) = match spec.mode {
            OpenMode::NewWindow => (run(scripts::ITERM2_CREATE_WINDOW, &[&command])?, false),
            OpenMode::NewTab => (run(scripts::ITERM2_CREATE_TAB, &[&command, &target])?, false),
            OpenMode::AiTopZshBottom | OpenMode::AiLeftZshRight => {
                let zsh = shell::cd_command(&spec.workspace_path)?;
                let direction = split_direction_arg(spec.mode);
                let output =
                    run(scripts::ITERM2_CREATE_SPLIT, &[&command, &target, &zsh, direction])?;
                (output, true)
            }
        };
        parse_created(&output, with_zsh)
    }

    fn split_zsh(&self, key: &SessionKey, direction: SplitDirection, cwd: &Path) -> Result<(), CueError> {
        let SessionKey::Iterm2 { unique_id } = key else {
            return Err(CueError::ObjectNotFound);
        };
        if !is_valid_unique_id(unique_id) {
            return Err(CueError::ObjectNotFound);
        }
        let command = shell::cd_command(cwd)?;
        let output = osascript::run(
            scripts::ITERM2_SPLIT_ZSH,
            &[unique_id.as_str(), split_zsh_direction_arg(direction), &command],
        )?;
        if is_valid_unique_id(&output) {
            Ok(())
        } else {
            Err(CueError::ObjectNotFound)
        }
    }
}

/// 就地補開 zsh 的方向參數：「在右側」是垂直分隔線、「在下方」是水平分隔線。
fn split_zsh_direction_arg(direction: SplitDirection) -> &'static str {
    match direction {
        SplitDirection::Right => "vertical",
        SplitDirection::Below => "horizontal",
    }
}

/// 分割腳本的方向參數。iTerm2 以分隔線命名：`horizontal` 新窗格在下、`vertical` 新窗格在右。
fn split_direction_arg(mode: OpenMode) -> &'static str {
    match mode {
        OpenMode::AiLeftZshRight => "vertical",
        _ => "horizontal",
    }
}

/// 解析建立腳本回傳的 unique ID：一般為一個，分割開啟方式為 AI ␟ zsh 兩個。
fn parse_created(output: &str, with_zsh: bool) -> Result<CreatedSession, CueError> {
    let ids: Vec<&str> = output.split(osascript::FIELD_SEP).collect();
    let expected = if with_zsh { 2 } else { 1 };
    if ids.len() != expected || !ids.iter().all(|id| is_valid_unique_id(id)) {
        return Err(CueError::Script("無法辨識 iTerm2 新窗格的 ID".into()));
    }
    let key = |id: &str| SessionKey::Iterm2 { unique_id: id.to_owned() };
    Ok(CreatedSession { key: key(ids[0]), zsh: ids.get(1).map(|id| key(id)) })
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
    let unique_id = fields[7].trim();
    if !is_valid_unique_id(unique_id) {
        return None;
    }
    let tty = normalize_tty(&fields[8])?;

    Some(DiscoveredSession {
        key: SessionKey::Iterm2 { unique_id: unique_id.to_owned() },
        tty,
        name: fields[9].clone(),
        window_id,
        window_index,
        window_bounds,
        tab_index,
    })
}

/// unique ID 只接受英數字與 `-`（UUID 形式）。
fn is_valid_unique_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= UNIQUE_ID_MAX_LEN
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
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

    /// notes-probe 第二輪的部分輸出：視窗 11896 分頁 1 為三個分割窗格。
    fn probe_sample() -> String {
        output(&[
            row(&["11896", "1", "2649", "495", "3219", "967", "1", "281FB514-2378-4265-804C-C17B1E9060AE", "/dev/ttys001", "..nts/bryan-log (-zsh)"]),
            row(&["11896", "1", "2649", "495", "3219", "967", "1", "F2769FF8-6FE7-4E82-8155-D4E41EC099A7", "/dev/ttys014", "..nts/bryan-log (-zsh)"]),
            row(&["11896", "1", "2649", "495", "3219", "967", "1", "436736AD-089F-482D-94C8-E1026897BB9F", "/dev/ttys015", "..ents/cue-desk (-zsh)"]),
            row(&["11896", "1", "2649", "495", "3219", "967", "2", "5769FDC8-FDDF-4894-92FC-87F9B49676BF", "/dev/ttys017", "✳ Claude Code (claude)"]),
            row(&["9782", "2", "1586", "82", "3220", "1018", "7", "4DA90FAA-598D-412A-8219-C0C510C378E4", "/dev/ttys012", "..ents/cue-desk (-zsh)"]),
            row(&["11905", "3", "1470", "643", "2040", "1080", "1", "B98AEF09-9608-451B-BF3D-77A455C336F4", "/dev/ttys019", "..ents/cue-desk (-zsh)"]),
        ]) + "\n"
    }

    #[test]
    fn parses_probe_output() {
        let sessions = parse_discover(&probe_sample());
        assert_eq!(sessions.len(), 6);

        let first = &sessions[0];
        assert_eq!(
            first.key,
            SessionKey::Iterm2 { unique_id: "281FB514-2378-4265-804C-C17B1E9060AE".into() }
        );
        assert_eq!(first.tty, "ttys001");
        assert_eq!(first.name, "..nts/bryan-log (-zsh)");
        assert_eq!(first.window_id, 11896);
        assert_eq!(first.window_index, 1);
        assert_eq!(first.window_bounds, Bounds { left: 2649, top: 495, right: 3219, bottom: 967 });
        assert_eq!(first.tab_index, 1);

        let last = &sessions[5];
        assert_eq!(last.window_id, 11905);
        assert_eq!(last.window_index, 3);
        assert_eq!(last.tty, "ttys019");
    }

    #[test]
    fn split_panes_are_separate_sessions_in_same_tab() {
        let sessions = parse_discover(&probe_sample());
        let tab1: Vec<_> =
            sessions.iter().filter(|s| s.window_id == 11896 && s.tab_index == 1).collect();
        assert_eq!(tab1.len(), 3);
        let ttys: Vec<_> = tab1.iter().map(|s| s.tty.as_str()).collect();
        assert_eq!(ttys, ["ttys001", "ttys014", "ttys015"]);
    }

    #[test]
    fn empty_output_yields_no_sessions() {
        assert!(parse_discover("").is_empty());
        assert!(parse_discover("\n").is_empty());
    }

    #[test]
    fn skips_rows_without_valid_tty() {
        let out = output(&[
            row(&["1", "1", "0", "0", "10", "10", "1", "AAAA-1", "", "no tty"]),
            row(&["1", "1", "0", "0", "10", "10", "2", "AAAA-2", "/dev/console", "console"]),
            row(&["1", "1", "0", "0", "10", "10", "3", "AAAA-3", "/dev/ttys123", "ok"]),
        ]);
        let sessions = parse_discover(&out);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].tty, "ttys123");
    }

    #[test]
    fn skips_malformed_rows() {
        let out = output(&[
            // 欄位不足
            row(&["1", "1", "0", "0", "10", "10", "1", "AAAA-1", "/dev/ttys001"]),
            // 欄位過多
            row(&["1", "1", "0", "0", "10", "10", "1", "AAAA-2", "/dev/ttys002", "a", "b"]),
            // window id 不是數字
            row(&["x", "1", "0", "0", "10", "10", "1", "AAAA-3", "/dev/ttys003", "n"]),
            // unique ID 含不允許的字元
            row(&["1", "1", "0", "0", "10", "10", "1", "AA AA;", "/dev/ttys004", "n"]),
            // 正常
            row(&["1", "1", "-1440", "-20", "0", "800", "1", "AAAA-5", "/dev/ttys005", "n"]),
        ]);
        let sessions = parse_discover(&out);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].tty, "ttys005");
        // 主螢幕左側的副螢幕座標為負值。
        assert_eq!(sessions[0].window_bounds.left, -1440);
    }

    #[test]
    fn unique_id_validation() {
        assert!(is_valid_unique_id("5769FDC8-FDDF-4894-92FC-87F9B49676BF"));
        assert!(!is_valid_unique_id(""));
        assert!(!is_valid_unique_id("abc\"def"));
        assert!(!is_valid_unique_id(&"A".repeat(UNIQUE_ID_MAX_LEN + 1)));
    }

    #[test]
    fn focus_rejects_foreign_or_invalid_keys_without_running_script() {
        let backend = ITerm2Backend::new();
        let foreign = SessionKey::TerminalApp { window_id: 1, tty: "ttys001".into() };
        assert_eq!(backend.focus(&foreign), Err(CueError::ObjectNotFound));
        let invalid = SessionKey::Iterm2 { unique_id: "x\" & quit".into() };
        assert_eq!(backend.focus(&invalid), Err(CueError::ObjectNotFound));
    }

    #[test]
    fn split_zsh_rejects_bad_input_without_running_script() {
        let backend = ITerm2Backend::new();
        let cwd = Path::new("/tmp");
        let foreign = SessionKey::TerminalApp { window_id: 1, tty: "ttys001".into() };
        assert_eq!(backend.split_zsh(&foreign, SplitDirection::Right, cwd), Err(CueError::ObjectNotFound));
        let invalid = SessionKey::Iterm2 { unique_id: "x\" & quit".into() };
        assert_eq!(backend.split_zsh(&invalid, SplitDirection::Below, cwd), Err(CueError::ObjectNotFound));
        let valid = SessionKey::Iterm2 { unique_id: "281FB514-2378-4265-804C-C17B1E9060AE".into() };
        assert_eq!(
            backend.split_zsh(&valid, SplitDirection::Right, Path::new("/tmp/a\nrm -rf ~")),
            Err(CueError::UnsafePath)
        );
    }

    #[test]
    fn split_zsh_direction_follows_divider_naming() {
        assert_eq!(split_zsh_direction_arg(SplitDirection::Right), "vertical");
        assert_eq!(split_zsh_direction_arg(SplitDirection::Below), "horizontal");
    }

    #[test]
    fn capabilities_match_applescript_engine() {
        let caps = ITerm2Backend::new().capabilities();
        assert!(caps.split && caps.new_tab && caps.type_text_no_enter);
        assert!(caps.rename && caps.close_pane);
        assert!(!caps.precise_tabs);
    }

    #[test]
    fn parses_created_ids() {
        let ai = "6D3C1A2B-0000-4F6E-9C51-8D0E2B1A7F10";
        let zsh = "0A1B2C3D-1111-4E5F-8A9B-0C1D2E3F4A5B";
        let one = parse_created(ai, false).unwrap();
        assert_eq!(one.key, SessionKey::Iterm2 { unique_id: ai.into() });
        assert_eq!(one.zsh, None);

        let two = parse_created(&row(&[ai, zsh]), true).unwrap();
        assert_eq!(two.key, SessionKey::Iterm2 { unique_id: ai.into() });
        assert_eq!(two.zsh, Some(SessionKey::Iterm2 { unique_id: zsh.into() }));
    }

    #[test]
    fn rejects_unexpected_create_output() {
        let ai = "6D3C1A2B-0000-4F6E-9C51-8D0E2B1A7F10";
        for (out, with_zsh) in [
            ("", false),
            (ai, true),
            (&*row(&[ai, ai]), false),
            (&*row(&[ai, ""]), true),
            ("x\" & quit", false),
        ] {
            assert!(
                matches!(parse_created(out, with_zsh), Err(CueError::Script(_))),
                "應拒絕 {out:?}（with_zsh = {with_zsh}）"
            );
        }
    }

    #[test]
    fn split_modes_map_to_iterm2_direction_names() {
        assert_eq!(split_direction_arg(OpenMode::AiTopZshBottom), "horizontal");
        assert_eq!(split_direction_arg(OpenMode::AiLeftZshRight), "vertical");
    }

    #[test]
    fn create_rejects_unsafe_path_without_running_script() {
        let backend = ITerm2Backend::new();
        for mode in [OpenMode::NewWindow, OpenMode::NewTab, OpenMode::AiTopZshBottom, OpenMode::AiLeftZshRight] {
            let spec = CreateSpec {
                workspace_path: "/tmp/a\nrm -rf ~".into(),
                kind: SessionKindToCreate::Claude,
                label: None,
                mode,
                target_window: None,
            };
            assert_eq!(backend.create(&spec), Err(CueError::UnsafePath));
        }
    }

    /// 本機手動：`cargo test -- --ignored iterm2_create --nocapture`
    /// 在含注入字元的暫存工作區以四種方式各建立一個 zsh 工作階段，檢查探索結果，
    /// 且路徑中的 `$(touch /tmp/pwned)` 沒有被執行。分割窗格在下／在右需目視確認。
    #[test]
    #[ignore = "操作本機 iTerm2"]
    fn iterm2_create_local() {
        use std::path::Path;
        use std::thread::sleep;

        let pwned = Path::new("/tmp/pwned");
        let _ = std::fs::remove_file(pwned);
        assert!(!pwned.exists(), "測試前無法移除 /tmp/pwned");

        // 保留暫存資料夾，新窗格 cd 進去後才能目視確認目錄。
        let ws = tempfile::tempdir().unwrap().keep().join("A's 專案 $(touch /tmp/pwned) \"x\"");
        std::fs::create_dir_all(&ws).unwrap();

        let backend = ITerm2Backend::new();
        let spec = |mode, target_window| CreateSpec {
            workspace_path: ws.clone(),
            kind: SessionKindToCreate::Zsh,
            label: None,
            mode,
            target_window,
        };
        let find = |key: &SessionKey| {
            sleep(Duration::from_millis(500));
            backend
                .discover()
                .expect("探索失敗")
                .into_iter()
                .find(|s| &s.key == key)
                .unwrap_or_else(|| panic!("探索不到新窗格 {key}"))
        };

        let created = backend.create(&spec(OpenMode::NewWindow, None)).expect("新視窗失敗");
        assert_eq!(created.zsh, None);
        let window = find(&created.key);
        println!("新視窗：{} | 視窗 {}", window.key, window.window_id);

        let created = backend.create(&spec(OpenMode::NewTab, Some(window.window_id))).expect("新分頁失敗");
        let tab = find(&created.key);
        assert_eq!(tab.window_id, window.window_id, "新分頁應開在指定視窗");
        assert_ne!(tab.tab_index, window.tab_index);
        println!("新分頁：{} | 分頁 {}", tab.key, tab.tab_index);

        for (mode, place) in [(OpenMode::AiTopZshBottom, "下方"), (OpenMode::AiLeftZshRight, "右側")] {
            let created = backend.create(&spec(mode, Some(window.window_id))).expect("分割建立失敗");
            let ai = find(&created.key);
            let zsh = find(&created.zsh.expect("分割應回傳 zsh 窗格"));
            assert_eq!(ai.window_id, window.window_id);
            assert_eq!((ai.window_id, ai.tab_index), (zsh.window_id, zsh.tab_index), "兩個窗格應在同一分頁");
            println!("{mode:?}：分頁 {}，請目視確認 zsh 窗格在{place}", ai.tab_index);
        }

        sleep(Duration::from_secs(2));
        assert!(!pwned.exists(), "注入的指令被執行了");
        println!("工作區：{}", ws.display());
    }

    /// 本機手動：`cargo test -- --ignored iterm2 --nocapture`
    /// iTerm2 執行中時列出所有窗格並記錄耗時，再依序聚焦每個窗格。
    #[test]
    #[ignore = "操作本機 iTerm2"]
    fn iterm2_discover_and_focus_local() {
        use std::time::Instant;

        let backend = ITerm2Backend::new();
        if !backend.is_running() {
            assert!(backend.discover().unwrap().is_empty());
            assert!(!backend.is_running(), "探索不得啟動 iTerm2");
            println!("iTerm2 未執行：探索回傳空清單，且未啟動 iTerm2");
            return;
        }

        let started = Instant::now();
        let sessions = backend.discover().expect("探索失敗");
        println!("探索 {} 個工作階段，耗時 {:?}", sessions.len(), started.elapsed());
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

        let missing = SessionKey::Iterm2 { unique_id: "00000000-0000-0000-0000-000000000000".into() };
        assert_eq!(backend.focus(&missing), Err(CueError::ObjectNotFound));
    }
}
