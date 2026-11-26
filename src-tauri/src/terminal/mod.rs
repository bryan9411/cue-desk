//! 終端機轉接層。
//!
//! `TerminalBackend` 是上層唯一依賴的終端機介面；上層只能透過 trait 與
//! `capabilities()` 操作終端機，新增其他終端機（例如 Ghostty、WezTerm）時不需改動上層。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CueError;
use crate::model::{Bounds, DiscoveredSession, ScreenIndex, SessionKey, SessionKind, WindowId};

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
/// 實際送進終端機的指令只能來自程式內固定列舉，`kind` 為 `Other` 時由指令產生端拒絕。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateSpec {
    pub workspace_path: PathBuf,
    pub kind: SessionKind,
    pub label: Option<String>,
    pub mode: OpenMode,
    pub target_window: Option<WindowId>,
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

    /// 建立工作階段。
    fn create(&self, _spec: &CreateSpec) -> Result<(), CueError> {
        Err(CueError::Unsupported)
    }

    /// 在指定工作階段旁分割出 zsh 窗格並切換到 `cwd`。
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
            kind: SessionKind::Claude,
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
    }
}
