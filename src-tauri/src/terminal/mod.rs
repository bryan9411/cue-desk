//! 終端機轉接層。
//!
//! `TerminalBackend` 是上層唯一依賴的終端機介面；上層只能透過 trait 與
//! `capabilities()` 操作終端機，新增其他終端機（例如 Ghostty、WezTerm）時不需改動上層。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CueError;
use crate::model::{Bounds, DiscoveredSession, ScreenIndex, SessionKey, WindowId};
use crate::shell::SessionKindToCreate;

pub mod detect;
pub mod iterm2;
pub mod terminal_app;

/// 固定的 AppleScript 腳本原文。變動資料一律經由 `on run argv` 傳入。
pub mod scripts {
    /// 列出 iTerm2 所有窗格；iTerm2 未執行時回傳空字串。
    pub const ITERM2_DISCOVER: &str = include_str!("scripts/iterm2_discover.applescript");
    /// 聚焦 iTerm2 的指定窗格；argv：unique ID。找到回傳 `ok`，否則空字串。
    pub const ITERM2_FOCUS: &str = include_str!("scripts/iterm2_focus.applescript");
    /// 列出 Terminal.app 所有分頁；Terminal.app 未執行時回傳空字串。
    pub const TERMINAL_APP_DISCOVER: &str =
        include_str!("scripts/terminal_app_discover.applescript");
    /// 聚焦 Terminal.app 的指定分頁；argv：window id、tty（`/dev/ttysNNN`）。找到回傳 `ok`，否則空字串。
    pub const TERMINAL_APP_FOCUS: &str = include_str!("scripts/terminal_app_focus.applescript");
    /// Terminal.app 開新視窗並送出指令；argv：指令。回傳新視窗的 window id 與 tty（`\x1f` 分隔）。
    pub const TERMINAL_APP_CREATE: &str = include_str!("scripts/terminal_app_create.applescript");
    /// iTerm2 開新視窗並送出指令；argv：指令。回傳新窗格的 unique ID。
    pub const ITERM2_CREATE_WINDOW: &str =
        include_str!("scripts/iterm2_create_window.applescript");
    /// iTerm2 開新分頁並送出指令；argv：指令、目標視窗 ID（空字串為最前視窗）。
    /// 回傳新窗格的 unique ID。
    pub const ITERM2_CREATE_TAB: &str = include_str!("scripts/iterm2_create_tab.applescript");
    /// iTerm2 開新分頁跑 AI 並分割出 zsh；argv：AI 指令、目標視窗 ID、zsh 指令、
    /// 分割方向（`horizontal`／`vertical`）。回傳 AI 與 zsh 窗格的 unique ID（`\x1f` 分隔）。
    pub const ITERM2_CREATE_SPLIT: &str = include_str!("scripts/iterm2_create_split.applescript");
    /// iTerm2 在指定窗格旁分割出 zsh；argv：unique ID、分割方向（`vertical`／`horizontal`）、
    /// 指令。回傳新窗格的 unique ID；找不到目標窗格時回傳空字串。
    pub const ITERM2_SPLIT_ZSH: &str = include_str!("scripts/iterm2_split_zsh.applescript");
    /// 設定 iTerm2 指定窗格的名稱；argv：unique ID、新名稱。找到回傳 `ok`，否則空字串。
    /// iTerm2 未執行時不啟動它。
    pub const ITERM2_RENAME: &str = include_str!("scripts/iterm2_rename.applescript");
    /// 關閉 iTerm2 的指定窗格；argv：unique ID。找到回傳 `ok`，否則空字串。
    /// iTerm2 未執行時不啟動它。
    pub const ITERM2_CLOSE: &str = include_str!("scripts/iterm2_close.applescript");
    /// 關閉 Terminal.app 的指定分頁；argv：window id、tty（`/dev/ttysNNN`）。
    /// 回傳 `ok`；視窗不只 1 個分頁時回傳 `multi` 且不關閉；找不到時回傳空字串。
    /// Terminal.app 未執行時不啟動它。
    pub const TERMINAL_APP_CLOSE: &str = include_str!("scripts/terminal_app_close.applescript");
    /// 把文字填入 iTerm2 指定窗格的輸入列，不送出 Enter；argv：unique ID、文字。
    /// 找到回傳 `ok`，否則空字串。iTerm2 未執行時不啟動它。
    pub const ITERM2_TYPE_NO_ENTER: &str = include_str!("scripts/iterm2_type_no_enter.applescript");
}

/// 後端支援的操作，前端依此停用不支援的功能。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// 分割窗格。
    pub split: bool,
    /// 在既有視窗開新分頁。
    pub new_tab: bool,
    /// 輸入文字但不送出 Enter（拖檔派工）。
    pub type_text_no_enter: bool,
    /// 重新命名工作階段。
    pub rename: bool,
    /// 單獨關閉一個窗格／分頁。
    pub close_pane: bool,
    /// 精準分頁操作（iTerm2 Socket 引擎啟用時）。
    pub precise_tabs: bool,
}

/// 建立工作階段的開啟方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpenMode {
    NewWindow,
    NewTab,
    /// AI 在上、zsh 在下。
    AiTopZshBottom,
    /// AI 在左、zsh 在右。
    AiLeftZshRight,
}

/// 分割 zsh 窗格的方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SplitDirection {
    Right,
    Below,
}

/// 建立工作階段的規格。
///
/// `kind` 只有固定的三種，送進終端機的指令由後端以 `shell` 模組產生，不接受任意字串。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSpec {
    pub workspace_path: PathBuf,
    pub kind: SessionKindToCreate,
    /// 使用者自訂標籤；由呼叫端綁定到建立出的工作階段，後端不送進終端機。
    pub label: Option<String>,
    pub mode: OpenMode,
    /// 「新分頁」與分割開啟方式的目標視窗；`None` 表示最前視窗。
    pub target_window: Option<WindowId>,
}

/// 建立工作階段的結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatedSession {
    /// 執行 `kind` 指令的窗格。
    pub key: SessionKey,
    /// 分割開啟方式額外建立的 zsh 窗格。
    pub zsh: Option<SessionKey>,
}

/// 終端機後端。
///
/// 方法皆為同步呼叫（內部執行 `osascript` 等阻塞操作），由呼叫端以
/// `tokio::task::spawn_blocking` 執行。除 `id`／`capabilities`／`is_running`／
/// `discover`／`focus` 外皆有預設實作，回傳 `Err(CueError::Unsupported)`。
pub trait TerminalBackend: Send + Sync {
    /// 後端代號，例如 `"iterm2"`、`"terminal"`。
    fn id(&self) -> &'static str;

    /// 後端支援的操作。
    fn capabilities(&self) -> Capabilities;

    /// 終端機 App 是否正在執行。探索前先檢查，**不得為了輪詢而啟動終端機**。
    fn is_running(&self) -> bool;

    /// 列出所有工作階段。終端機未執行時回傳空清單。
    fn discover(&self) -> Result<Vec<DiscoveredSession>, CueError>;

    /// 聚焦指定工作階段（使用者主動操作，可啟動終端機）。
    fn focus(&self, key: &SessionKey) -> Result<(), CueError>;

    /// 建立工作階段（使用者主動操作，可啟動終端機）。
    fn create(&self, _spec: &CreateSpec) -> Result<CreatedSession, CueError> {
        Err(CueError::Unsupported)
    }

    /// 在指定工作階段旁開出 zsh 並切換到 `cwd`，原工作階段不送任何輸入。
    /// 不支援分割的後端（`capabilities().split == false`）改開新視窗。
    fn split_zsh(
        &self,
        _key: &SessionKey,
        _direction: SplitDirection,
        _cwd: &Path,
    ) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }

    /// 重新命名工作階段。
    fn rename(&self, _key: &SessionKey, _name: &str) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }

    /// 關閉工作階段（使用者主動操作；Cue 結束時不關閉任何工作階段）。
    /// 程式仍在執行時交由終端機自己的確認機制，回傳 `Ok` 只代表已送出關閉。
    fn close(&self, _key: &SessionKey) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }

    /// 輸入文字但**絕不送出 Enter**。
    fn type_text_no_enter(&self, _key: &SessionKey, _text: &str) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }

    /// 把終端機視窗排到 Cue 面板旁邊。
    fn arrange_beside(&self, _panel_bounds: Bounds) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }

    /// 把工作階段所在的視窗移到指定螢幕。
    fn move_to_screen(&self, _key: &SessionKey, _screen: ScreenIndex) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }

    /// 把指定視窗集中到目標框架。
    fn gather_windows(&self, _window_ids: &[WindowId], _target: Bounds) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 只實作必要方法的後端，用來確認其餘方法的預設實作。
    struct MinimalBackend;

    impl TerminalBackend for MinimalBackend {
        fn id(&self) -> &'static str {
            "minimal"
        }
        fn capabilities(&self) -> Capabilities {
            Capabilities::default()
        }
        fn is_running(&self) -> bool {
            false
        }
        fn discover(&self) -> Result<Vec<DiscoveredSession>, CueError> {
            Ok(Vec::new())
        }
        fn focus(&self, _key: &SessionKey) -> Result<(), CueError> {
            Ok(())
        }
    }

    #[test]
    fn optional_methods_default_to_unsupported() {
        let backend: Box<dyn TerminalBackend> = Box::new(MinimalBackend);
        let key = SessionKey::Iterm2 { unique_id: "ABC".into() };
        let spec = CreateSpec {
            workspace_path: PathBuf::from("/tmp"),
            kind: SessionKindToCreate::Claude,
            label: None,
            mode: OpenMode::NewWindow,
            target_window: None,
        };
        let bounds = Bounds::default();

        assert_eq!(backend.create(&spec), Err(CueError::Unsupported));
        assert_eq!(
            backend.split_zsh(&key, SplitDirection::Right, Path::new("/tmp")),
            Err(CueError::Unsupported)
        );
        assert_eq!(backend.rename(&key, "x"), Err(CueError::Unsupported));
        assert_eq!(backend.close(&key), Err(CueError::Unsupported));
        assert_eq!(backend.type_text_no_enter(&key, "x"), Err(CueError::Unsupported));
        assert_eq!(backend.arrange_beside(bounds), Err(CueError::Unsupported));
        assert_eq!(backend.move_to_screen(&key, 1), Err(CueError::Unsupported));
        assert_eq!(backend.gather_windows(&[1, 2], bounds), Err(CueError::Unsupported));
    }

    #[test]
    fn capabilities_serialize_camel_case() {
        let caps = Capabilities { type_text_no_enter: true, close_pane: true, ..Default::default() };
        assert_eq!(
            serde_json::to_value(caps).unwrap(),
            json!({
                "split": false,
                "newTab": false,
                "typeTextNoEnter": true,
                "rename": false,
                "closePane": true,
                "preciseTabs": false
            })
        );
    }

    #[test]
    fn open_mode_and_direction_use_camel_case() {
        assert_eq!(serde_json::to_value(OpenMode::AiTopZshBottom).unwrap(), json!("aiTopZshBottom"));
        assert_eq!(
            serde_json::from_value::<OpenMode>(json!("aiLeftZshRight")).unwrap(),
            OpenMode::AiLeftZshRight
        );
        assert_eq!(serde_json::to_value(SplitDirection::Below).unwrap(), json!("below"));
    }

    #[test]
    fn scripts_are_embedded() {
        assert!(!scripts::ITERM2_DISCOVER.is_empty());
        assert!(!scripts::ITERM2_FOCUS.is_empty());
        assert!(!scripts::TERMINAL_APP_DISCOVER.is_empty());
        assert!(!scripts::TERMINAL_APP_FOCUS.is_empty());
        assert!(!scripts::ITERM2_CREATE_WINDOW.is_empty());
        assert!(!scripts::ITERM2_CREATE_TAB.is_empty());
        assert!(!scripts::ITERM2_CREATE_SPLIT.is_empty());
        assert!(!scripts::TERMINAL_APP_CREATE.is_empty());
        assert!(!scripts::ITERM2_SPLIT_ZSH.is_empty());
        assert!(!scripts::ITERM2_RENAME.is_empty());
        assert!(!scripts::ITERM2_CLOSE.is_empty());
        assert!(!scripts::TERMINAL_APP_CLOSE.is_empty());
        assert!(!scripts::ITERM2_TYPE_NO_ENTER.is_empty());
    }
}
