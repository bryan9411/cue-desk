//! Cue 後端共用的錯誤型別。
//!
//! 錯誤訊息會經由 Tauri IPC 顯示在前端，因此只放簡短、可公開的描述，
//! **不得包含任何機密**（例如 iTerm2 API 的 Cookie 與 Key）。

use std::fmt;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// stderr 摘要的最大字元數，避免把過長的輸出帶進錯誤訊息。
const SUMMARY_MAX_CHARS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CueError {
    /// 使用者拒絕（或尚未允許）自動化權限（errAEEventNotPermitted, -1743）。
    AutomationDenied,
    /// 目標 App 未執行（procNotFound, -600）。
    AppNotRunning,
    /// 找不到指定的物件，例如視窗或工作階段已關閉（errAENoSuchObject, -1728）。
    ObjectNotFound,
    /// 執行逾時，子程序已被終止。
    Timeout,
    /// 無法啟動或與子程序溝通。
    Io(String),
    /// 其他腳本錯誤，內容為 stderr 摘要。
    Script(String),
    /// 目前的終端機後端不支援這個操作。
    Unsupported,
    /// 工作階段已結束，無法操作。
    SessionExited,
    /// 工作階段仍存在，只有已結束的節點能重新開啟或移除。
    SessionAlive,
    /// 找不到指定的工作區。
    WorkspaceNotFound,
    /// 無法加入工作區（路徑不存在或不是資料夾），內容為原因。
    InvalidWorkspace(String),
    /// 路徑含換行或其他控制字元（或不是合法 UTF-8），不能安全送入終端機。
    UnsafePath,
    /// 標籤不合法（過長或含控制字元），內容為原因。
    InvalidLabel(String),
}

impl CueError {
    /// 給前端判斷用的固定代碼。
    pub fn kind(&self) -> &'static str {
        match self {
            CueError::AutomationDenied => "automation_denied",
            CueError::AppNotRunning => "app_not_running",
            CueError::ObjectNotFound => "object_not_found",
            CueError::Timeout => "timeout",
            CueError::Io(_) => "io",
            CueError::Script(_) => "script",
            CueError::Unsupported => "unsupported",
            CueError::SessionExited => "session_exited",
            CueError::SessionAlive => "session_alive",
            CueError::WorkspaceNotFound => "workspace_not_found",
            CueError::InvalidWorkspace(_) => "invalid_workspace",
            CueError::UnsafePath => "unsafe_path",
            CueError::InvalidLabel(_) => "invalid_label",
        }
    }

    /// 由 osascript 的 stderr 判斷錯誤類別。
    pub fn from_osascript_stderr(stderr: &str) -> Self {
        if has_error_code(stderr, "-1743") {
            CueError::AutomationDenied
        } else if has_error_code(stderr, "-600") {
            CueError::AppNotRunning
        } else if has_error_code(stderr, "-1728") {
            CueError::ObjectNotFound
        } else {
            CueError::Script(summarize(stderr))
        }
    }
}

/// osascript 的錯誤碼以 `(-1743)` 的形式出現在 stderr 結尾。
fn has_error_code(stderr: &str, code: &str) -> bool {
    stderr.contains(&format!("({code})"))
}

/// 把 stderr 壓成單行並截斷。
pub(crate) fn summarize(text: &str) -> String {
    let single_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if single_line.chars().count() <= SUMMARY_MAX_CHARS {
        single_line
    } else {
        let mut cut: String = single_line.chars().take(SUMMARY_MAX_CHARS).collect();
        cut.push('…');
        cut
    }
}

impl fmt::Display for CueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CueError::AutomationDenied => {
                write!(f, "未取得自動化權限，請在「系統設定 › 隱私權與安全性 › 自動化」允許 Cue 控制終端機")
            }
            CueError::AppNotRunning => write!(f, "終端機 App 未執行"),
            CueError::ObjectNotFound => write!(f, "找不到指定的視窗或工作階段"),
            CueError::Timeout => write!(f, "終端機腳本執行逾時"),
            CueError::Io(msg) => write!(f, "無法執行 osascript：{msg}"),
            CueError::Script(msg) => write!(f, "終端機腳本錯誤：{msg}"),
            CueError::Unsupported => write!(f, "目前的終端機不支援這個操作"),
            CueError::SessionExited => write!(f, "這個工作階段已結束"),
            CueError::SessionAlive => write!(f, "這個工作階段仍在執行"),
            CueError::WorkspaceNotFound => write!(f, "找不到指定的工作區"),
            CueError::InvalidWorkspace(msg) => write!(f, "無法加入工作區：{msg}"),
            CueError::UnsafePath => write!(f, "路徑含有換行或控制字元，無法安全送入終端機"),
            CueError::InvalidLabel(msg) => write!(f, "標籤無效：{msg}"),
        }
    }
}

impl std::error::Error for CueError {}

/// 序列化為 `{ "kind": "...", "message": "..." }`，供前端顯示。
impl Serialize for CueError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("CueError", 2)?;
        s.serialize_field("kind", self.kind())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_codes() {
        assert_eq!(
            CueError::from_osascript_stderr(
                "execution error: Not authorized to send Apple events to iTerm. (-1743)"
            ),
            CueError::AutomationDenied
        );
        assert_eq!(
            CueError::from_osascript_stderr("execution error: iTerm got an error: Application isn't running. (-600)"),
            CueError::AppNotRunning
        );
        assert_eq!(
            CueError::from_osascript_stderr("execution error: Can't get window id 42. (-1728)"),
            CueError::ObjectNotFound
        );
    }

    #[test]
    fn unknown_code_keeps_summary() {
        let err = CueError::from_osascript_stderr("67:70: syntax error:\n  Expected end of line. (-2741)\n");
        assert_eq!(
            err,
            CueError::Script("67:70: syntax error: Expected end of line. (-2741)".into())
        );
    }

    #[test]
    fn summary_is_truncated() {
        let long = "a".repeat(500);
        let s = summarize(&long);
        assert_eq!(s.chars().count(), SUMMARY_MAX_CHARS + 1);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn serializes_kind_and_message() {
        let json = serde_json::to_value(CueError::Timeout).unwrap();
        assert_eq!(json["kind"], "timeout");
        assert_eq!(json["message"], "終端機腳本執行逾時");

        let json = serde_json::to_value(CueError::Unsupported).unwrap();
        assert_eq!(json["kind"], "unsupported");
        assert_eq!(json["message"], "目前的終端機不支援這個操作");

        let json = serde_json::to_value(CueError::SessionExited).unwrap();
        assert_eq!(json["kind"], "session_exited");

        let json = serde_json::to_value(CueError::InvalidWorkspace("不是資料夾".into())).unwrap();
        assert_eq!(json["kind"], "invalid_workspace");
        assert_eq!(json["message"], "無法加入工作區：不是資料夾");

        let json = serde_json::to_value(CueError::UnsafePath).unwrap();
        assert_eq!(json["kind"], "unsafe_path");

        let json = serde_json::to_value(CueError::InvalidLabel("最多 64 個字元".into())).unwrap();
        assert_eq!(json["kind"], "invalid_label");
        assert_eq!(json["message"], "標籤無效：最多 64 個字元");
    }
}
