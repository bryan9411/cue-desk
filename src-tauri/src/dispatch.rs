//! 拖檔派工：把檔案路徑填入工作階段的輸入列，**絕不送出 Enter**，由使用者補上指示後親自送出。
//!
//! 只有同時符合三個條件才接受：後端能輸入文字而不送 Enter、前景是 AI CLI 或閒置的 shell、
//! 狀態不是 `waiting`／`exited`。其他情況下路徑字元可能被當成按鍵或選項，因此一律拒絕。

use std::fmt;

use crate::error::CueError;
use crate::model::{SessionKey, SessionKind, Status};
use crate::shell;
use crate::terminal::{Capabilities, TerminalBackend};

/// 拒絕派工的原因；`Display` 是給使用者看的說明。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    /// 後端無法只輸入文字而不送出 Enter（Terminal.app 後備模式）。
    NoTypeWithoutEnter,
    /// 前景不是 AI CLI 或閒置的 shell；內容為程式名稱。
    ForegroundProgram(String),
    /// AI 正在等待授權。
    WaitingForApproval,
    /// 工作階段已結束。
    Exited,
}

impl RejectReason {
    /// 給前端判斷用的固定代碼。
    pub fn code(&self) -> &'static str {
        match self {
            RejectReason::NoTypeWithoutEnter => "no_type_without_enter",
            RejectReason::ForegroundProgram(_) => "foreground_program",
            RejectReason::WaitingForApproval => "waiting_for_approval",
            RejectReason::Exited => "exited",
        }
    }
}

impl fmt::Display for RejectReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RejectReason::NoTypeWithoutEnter => {
                write!(f, "終端機後備模式無法只填入文字而不送出 Enter")
            }
            RejectReason::ForegroundProgram(name) => {
                let name = if name.is_empty() { "其他程式" } else { name.as_str() };
                write!(f, "前景是 {name}，路徑字元可能被當成按鍵指令")
            }
            RejectReason::WaitingForApproval => {
                write!(f, "AI 正在等你授權，路徑中的字元可能被當成選項按鍵")
            }
            RejectReason::Exited => write!(f, "這個工作階段已結束"),
        }
    }
}

/// 判斷能否對工作階段派工。三個條件都要符合；依「後端 → 狀態 → 前景程式」的順序回報第一個不符的原因。
pub fn check_dispatch(caps: Capabilities, kind: &SessionKind, status: Status) -> Result<(), RejectReason> {
    // Terminal.app 的 `do script` 一定會送出 Enter，路徑會被當成指令執行。
    if !caps.type_text_no_enter {
        return Err(RejectReason::NoTypeWithoutEnter);
    }
    match status {
        Status::Waiting => return Err(RejectReason::WaitingForApproval),
        Status::Exited => return Err(RejectReason::Exited),
        Status::Idle | Status::Running | Status::Done => {}
    }
    match kind {
        SessionKind::Claude | SessionKind::Gemini | SessionKind::Zsh => Ok(()),
        SessionKind::Other(name) => Err(RejectReason::ForegroundProgram(name.clone())),
    }
}

/// 組出要填入輸入列的文字：每個路徑以 shell 單引號跳脫、以單一空白分隔，結尾不加換行。
///
/// 任一路徑含換行或其他控制字元時整批拒絕（[`CueError::UnsafePath`]），不送出部分路徑。
pub fn build_paths_text(paths: &[String]) -> Result<String, CueError> {
    let quoted = paths.iter().map(|p| shell::quote(p)).collect::<Result<Vec<_>, _>>()?;
    Ok(quoted.join(" "))
}

/// 先聚焦窗格再填入文字：使用者要能立刻在該窗格補上指示並按 Enter。
pub fn send(backend: &dyn TerminalBackend, key: &SessionKey, text: &str) -> Result<(), CueError> {
    backend.focus(key)?;
    backend.type_text_no_enter(key, text)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::model::DiscoveredSession;

    fn iterm_caps() -> Capabilities {
        Capabilities { type_text_no_enter: true, ..Capabilities::default() }
    }

    #[test]
    fn accepts_ai_and_idle_shell_in_non_waiting_states() {
        for kind in [SessionKind::Claude, SessionKind::Gemini, SessionKind::Zsh] {
            for status in [Status::Idle, Status::Running, Status::Done] {
                assert_eq!(check_dispatch(iterm_caps(), &kind, status), Ok(()), "{kind:?} {status:?}");
            }
        }
    }

    #[test]
    fn rejects_backend_without_type_text_no_enter() {
        let caps = Capabilities { type_text_no_enter: false, ..iterm_caps() };
        let err = check_dispatch(caps, &SessionKind::Claude, Status::Idle).unwrap_err();
        assert_eq!(err, RejectReason::NoTypeWithoutEnter);
        assert_eq!(err.to_string(), "終端機後備模式無法只填入文字而不送出 Enter");
    }

    #[test]
    fn rejects_other_foreground_programs() {
        for name in ["vim", "less", "npm"] {
            let err = check_dispatch(iterm_caps(), &SessionKind::Other(name.into()), Status::Idle).unwrap_err();
            assert_eq!(err, RejectReason::ForegroundProgram(name.into()));
        }
        let err = check_dispatch(iterm_caps(), &SessionKind::Other("vim".into()), Status::Idle).unwrap_err();
        assert_eq!(err.to_string(), "前景是 vim，路徑字元可能被當成按鍵指令");
        assert_eq!(
            RejectReason::ForegroundProgram(String::new()).to_string(),
            "前景是 其他程式，路徑字元可能被當成按鍵指令"
        );
    }

    #[test]
    fn rejects_waiting_and_exited() {
        let err = check_dispatch(iterm_caps(), &SessionKind::Claude, Status::Waiting).unwrap_err();
        assert_eq!(err, RejectReason::WaitingForApproval);
        assert_eq!(err.to_string(), "AI 正在等你授權，路徑中的字元可能被當成選項按鍵");
        let err = check_dispatch(iterm_caps(), &SessionKind::Zsh, Status::Exited).unwrap_err();
        assert_eq!(err, RejectReason::Exited);
    }

    #[test]
    fn backend_check_comes_first() {
        let caps = Capabilities::default();
        let err = check_dispatch(caps, &SessionKind::Other("vim".into()), Status::Waiting).unwrap_err();
        assert_eq!(err, RejectReason::NoTypeWithoutEnter);
    }

    #[test]
    fn quotes_each_path_and_joins_with_space_without_newline() {
        let paths = vec![
            "/Users/me/A's 專案 $(touch /tmp/pwned) \"x\"/a.txt".to_string(),
            "/tmp/b c.png".to_string(),
        ];
        let text = build_paths_text(&paths).unwrap();
        assert_eq!(
            text,
            "'/Users/me/A'\\''s 專案 $(touch /tmp/pwned) \"x\"/a.txt' '/tmp/b c.png'"
        );
        assert!(!text.ends_with('\n') && !text.ends_with('\r'));
        assert_eq!(build_paths_text(&[]).unwrap(), "");
    }

    #[test]
    fn rejects_whole_batch_when_any_path_has_control_chars() {
        for bad in ["/tmp/a\nrm -rf ~", "/tmp/a\r", "/tmp/\x1b[A", "/tmp/\0"] {
            let paths = vec!["/tmp/ok".to_string(), bad.to_string()];
            assert_eq!(build_paths_text(&paths), Err(CueError::UnsafePath), "{bad:?}");
        }
    }

    /// 記錄呼叫順序的後端。
    #[derive(Default)]
    struct Recorder {
        calls: Mutex<Vec<String>>,
        fail_focus: bool,
    }

    impl TerminalBackend for Recorder {
        fn id(&self) -> &'static str {
            "recorder"
        }
        fn capabilities(&self) -> Capabilities {
            iterm_caps()
        }
        fn is_running(&self) -> bool {
            true
        }
        fn discover(&self) -> Result<Vec<DiscoveredSession>, CueError> {
            Ok(Vec::new())
        }
        fn focus(&self, key: &SessionKey) -> Result<(), CueError> {
            self.calls.lock().unwrap().push(format!("focus {key}"));
            if self.fail_focus {
                Err(CueError::ObjectNotFound)
            } else {
                Ok(())
            }
        }
        fn type_text_no_enter(&self, key: &SessionKey, text: &str) -> Result<(), CueError> {
            self.calls.lock().unwrap().push(format!("type {key} {text}"));
            Ok(())
        }
    }

    #[test]
    fn send_focuses_then_types() {
        let backend = Recorder::default();
        let key = SessionKey::Iterm2 { unique_id: "ABC-1".into() };
        send(&backend, &key, "'/tmp/a'").unwrap();
        assert_eq!(
            *backend.calls.lock().unwrap(),
            vec!["focus iterm2:ABC-1".to_string(), "type iterm2:ABC-1 '/tmp/a'".to_string()]
        );
    }

    #[test]
    fn send_does_not_type_when_focus_fails() {
        let backend = Recorder { fail_focus: true, ..Recorder::default() };
        let key = SessionKey::Iterm2 { unique_id: "ABC-1".into() };
        assert_eq!(send(&backend, &key, "'/tmp/a'"), Err(CueError::ObjectNotFound));
        assert_eq!(backend.calls.lock().unwrap().len(), 1);
    }
}
