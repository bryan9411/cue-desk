//! 工作階段領域型別。
//!
//! 這些型別會經由 Tauri IPC 送到前端，欄位一律序列化為 camelCase。

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};

/// 終端機 App 回報的視窗 ID（AppleScript `id of window`）。
/// 同一次終端機執行期間穩定，是**視窗身分**；視窗序號只用於顯示。
pub type WindowId = i64;

/// 螢幕序號（0 起算），對應 `NSScreen.screens` 的順序。
pub type ScreenIndex = usize;

/// 工作階段中執行的程式類型。
///
/// 序列化為 `{ "type": "claude" }`；`Other` 為 `{ "type": "other", "name": "vim" }`，
/// `name` 是前景程式的執行檔名稱，只用於顯示。設定檔以相同格式記錄最後已知類型。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", content = "name", rename_all = "lowercase")]
pub enum SessionKind {
    Claude,
    Gemini,
    Zsh,
    Other(String),
}

/// 工作階段狀態。`Running`／`Waiting`／`Done` 只由 Hook 驅動。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Idle,
    Running,
    Waiting,
    Done,
    Exited,
}

/// 工作階段身分。
///
/// 穩定字串形式（同時作為前端節點 id）：
/// - iTerm2：`iterm2:<unique id>`
/// - Terminal.app：`terminal:<window id>:<tty>`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SessionKey {
    Iterm2 { unique_id: String },
    TerminalApp { window_id: WindowId, tty: String },
}

const ITERM2_PREFIX: &str = "iterm2:";
const TERMINAL_APP_PREFIX: &str = "terminal:";

impl SessionKey {
    /// 穩定字串形式，作為節點 id。
    pub fn as_id(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for SessionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionKey::Iterm2 { unique_id } => write!(f, "{ITERM2_PREFIX}{unique_id}"),
            SessionKey::TerminalApp { window_id, tty } => {
                write!(f, "{TERMINAL_APP_PREFIX}{window_id}:{tty}")
            }
        }
    }
}

/// 解析節點 id 時的錯誤（格式不符）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSessionKey;

impl fmt::Display for InvalidSessionKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "無效的工作階段 id")
    }
}

impl std::error::Error for InvalidSessionKey {}

impl FromStr for SessionKey {
    type Err = InvalidSessionKey;

    /// 由前端傳回的節點 id 還原身分；格式不符一律拒絕。
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(unique_id) = s.strip_prefix(ITERM2_PREFIX) {
            let valid = !unique_id.is_empty()
                && unique_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
            return if valid {
                Ok(SessionKey::Iterm2 { unique_id: unique_id.to_string() })
            } else {
                Err(InvalidSessionKey)
            };
        }
        if let Some(rest) = s.strip_prefix(TERMINAL_APP_PREFIX) {
            let (wid, tty) = rest.split_once(':').ok_or(InvalidSessionKey)?;
            let window_id: WindowId = wid.parse().map_err(|_| InvalidSessionKey)?;
            // tty 必須已是正規化形式，不接受 `/dev/` 或空白等變體。
            let tty = normalize_tty(tty).filter(|t| t == tty).ok_or(InvalidSessionKey)?;
            return Ok(SessionKey::TerminalApp { window_id, tty });
        }
        Err(InvalidSessionKey)
    }
}

/// 以穩定字串形式序列化，前端直接當節點 id 使用。
impl Serialize for SessionKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// 視窗框架（點座標，左上為原點）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Bounds {
    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

/// `TerminalBackend::discover()` 回報的單一工作階段（窗格或分頁）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredSession {
    /// 工作階段身分。
    pub key: SessionKey,
    /// 經 [`normalize_tty`] 正規化的 tty（例如 `ttys003`）。
    pub tty: String,
    /// 終端機顯示的名稱；只用於顯示，不得當身分或判斷類型。
    pub name: String,
    /// 視窗身分。
    pub window_id: WindowId,
    /// 視窗序號（1 起算）；會隨視窗前後順序變動，只用於顯示。
    pub window_index: u32,
    /// 視窗框架。
    pub window_bounds: Bounds,
    /// 分頁序號（1 起算）；同一分頁的分割窗格相同。
    pub tab_index: u32,
}

/// 把各來源的 tty 字串正規化為 `ttysNNN`。
///
/// iTerm2、Terminal.app、`ps`、Hook 四個來源**一律經過這個函式**，
/// 其他地方不得自行處理 tty 字串。不符合 `^ttys[0-9]+$` 時回傳 `None`（代表沒有 tty）。
pub fn normalize_tty(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let name = trimmed.strip_prefix("/dev/").unwrap_or(trimmed);
    let digits = name.strip_prefix("ttys")?;
    if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
        Some(name.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_tty_accepts_known_formats() {
        assert_eq!(normalize_tty("/dev/ttys000").as_deref(), Some("ttys000"));
        assert_eq!(normalize_tty("ttys021").as_deref(), Some("ttys021"));
        assert_eq!(normalize_tty(" ttys1234 ").as_deref(), Some("ttys1234"));
        assert_eq!(normalize_tty("/dev/ttys5\n").as_deref(), Some("ttys5"));
    }

    #[test]
    fn normalize_tty_rejects_invalid() {
        for raw in ["??", "console", "", "   ", "/dev/ttys", "ttys", "ttys0a", "tty000", "/dev/console", "ttys-1"] {
            assert_eq!(normalize_tty(raw), None, "應拒絕 {raw:?}");
        }
    }

    #[test]
    fn status_serializes_lowercase() {
        assert_eq!(serde_json::to_value(Status::Waiting).unwrap(), json!("waiting"));
        assert_eq!(serde_json::to_value(Status::Idle).unwrap(), json!("idle"));
        assert_eq!(serde_json::to_value(Status::Exited).unwrap(), json!("exited"));
        assert_eq!(Status::default(), Status::Idle);
    }

    #[test]
    fn session_kind_serialization() {
        assert_eq!(serde_json::to_value(SessionKind::Claude).unwrap(), json!({ "type": "claude" }));
        assert_eq!(serde_json::to_value(SessionKind::Zsh).unwrap(), json!({ "type": "zsh" }));
        assert_eq!(
            serde_json::to_value(SessionKind::Other("vim".into())).unwrap(),
            json!({ "type": "other", "name": "vim" })
        );
        for kind in [SessionKind::Gemini, SessionKind::Other("npm".into())] {
            let v = serde_json::to_value(&kind).unwrap();
            assert_eq!(serde_json::from_value::<SessionKind>(v).unwrap(), kind);
        }
    }

    #[test]
    fn session_key_string_forms() {
        let iterm = SessionKey::Iterm2 { unique_id: "9F2C1A3B-0D4E-4F5A-8B6C-7D8E9F0A1B2C".into() };
        assert_eq!(iterm.as_id(), "iterm2:9F2C1A3B-0D4E-4F5A-8B6C-7D8E9F0A1B2C");
        assert_eq!(serde_json::to_value(&iterm).unwrap(), json!("iterm2:9F2C1A3B-0D4E-4F5A-8B6C-7D8E9F0A1B2C"));

        let term = SessionKey::TerminalApp { window_id: 1234, tty: "ttys021".into() };
        assert_eq!(term.as_id(), "terminal:1234:ttys021");
        assert_eq!(serde_json::to_value(&term).unwrap(), json!("terminal:1234:ttys021"));
    }

    #[test]
    fn session_key_round_trip() {
        for key in [
            SessionKey::Iterm2 { unique_id: "9F2C1A3B-0D4E-4F5A-8B6C-7D8E9F0A1B2C".into() },
            SessionKey::TerminalApp { window_id: 42, tty: "ttys003".into() },
        ] {
            assert_eq!(key.as_id().parse::<SessionKey>(), Ok(key));
        }
    }

    #[test]
    fn session_key_rejects_malformed() {
        for s in [
            "",
            "iterm2:",
            "iterm2:abc def",
            "iterm2:a;b",
            "terminal:",
            "terminal:42",
            "terminal:x:ttys003",
            "terminal:42:/dev/ttys003",
            "terminal:42:??",
            "other:1",
        ] {
            assert_eq!(s.parse::<SessionKey>(), Err(InvalidSessionKey), "應拒絕 {s:?}");
        }
    }

    #[test]
    fn discovered_session_serializes_camel_case() {
        let s = DiscoveredSession {
            key: SessionKey::TerminalApp { window_id: 7, tty: "ttys001".into() },
            tty: "ttys001".into(),
            name: "user@host".into(),
            window_id: 7,
            window_index: 1,
            window_bounds: Bounds { left: 0, top: 25, right: 800, bottom: 625 },
            tab_index: 1,
        };
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["key"], json!("terminal:7:ttys001"));
        assert_eq!(v["windowId"], json!(7));
        assert_eq!(v["windowIndex"], json!(1));
        assert_eq!(v["tabIndex"], json!(1));
        assert_eq!(v["windowBounds"], json!({ "left": 0, "top": 25, "right": 800, "bottom": 625 }));
        assert_eq!(s.window_bounds.width(), 800);
        assert_eq!(s.window_bounds.height(), 600);
    }
}
