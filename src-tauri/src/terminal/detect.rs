//! 偵測終端機 App 是否安裝與執行中，並依使用者偏好決定後端。
//!
//! 只透過 `NSWorkspace`／`NSRunningApplication` 查詢，**不使用 AppleScript
//! `tell application`**，避免查詢本身把終端機開起來。

use objc2::rc::autoreleasepool;
use objc2_app_kit::{NSRunningApplication, NSWorkspace};
use objc2_foundation::NSString;
use serde::{Deserialize, Serialize};

/// 支援的終端機 App。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalApp {
    Iterm2,
    TerminalApp,
}

impl TerminalApp {
    /// App 的 bundle identifier。
    pub fn bundle_id(self) -> &'static str {
        match self {
            TerminalApp::Iterm2 => "com.googlecode.iterm2",
            TerminalApp::TerminalApp => "com.apple.Terminal",
        }
    }
}

/// 是否已安裝（LaunchServices 找得到該 bundle identifier）。
pub fn is_installed(app: TerminalApp) -> bool {
    autoreleasepool(|_| {
        let bundle_id = NSString::from_str(app.bundle_id());
        NSWorkspace::sharedWorkspace()
            .URLForApplicationWithBundleIdentifier(&bundle_id)
            .is_some()
    })
}

/// 是否正在執行。只查詢目前的執行中 App 清單，不會啟動 App。
pub fn is_running(app: TerminalApp) -> bool {
    autoreleasepool(|_| {
        let bundle_id = NSString::from_str(app.bundle_id());
        NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id)
            .iter()
            .any(|running| !running.isTerminated())
    })
}

/// 使用者選擇的終端機後端。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BackendPref {
    /// 有 iTerm2 用 iTerm2，否則 Terminal.app。
    #[default]
    Auto,
    Iterm2,
    TerminalApp,
}

/// 沒有採用使用者指定後端的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FallbackReason {
    /// 指定 iTerm2，但系統上找不到 iTerm2。
    Iterm2NotInstalled,
}

/// 後端選擇結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendSelection {
    /// 實際採用的終端機。
    pub app: TerminalApp,
    /// 與偏好不同時的原因；`Auto` 不算退回，固定為 `None`。
    pub fallback: Option<FallbackReason>,
}

/// 依偏好與 iTerm2 安裝狀態決定後端（純函式，方便測試）。
pub fn resolve_backend(pref: BackendPref, iterm2_installed: bool) -> BackendSelection {
    let (app, fallback) = match pref {
        BackendPref::Auto if iterm2_installed => (TerminalApp::Iterm2, None),
        BackendPref::Auto => (TerminalApp::TerminalApp, None),
        BackendPref::Iterm2 if iterm2_installed => (TerminalApp::Iterm2, None),
        BackendPref::Iterm2 => (TerminalApp::TerminalApp, Some(FallbackReason::Iterm2NotInstalled)),
        BackendPref::TerminalApp => (TerminalApp::TerminalApp, None),
    };
    BackendSelection { app, fallback }
}

/// 依偏好與本機實際安裝狀態決定後端。Terminal.app 為系統內建，視為一定存在。
pub fn select_backend(pref: BackendPref) -> BackendSelection {
    let iterm2_installed = match pref {
        BackendPref::TerminalApp => false,
        BackendPref::Auto | BackendPref::Iterm2 => is_installed(TerminalApp::Iterm2),
    };
    resolve_backend(pref, iterm2_installed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn auto_prefers_iterm2_when_installed() {
        assert_eq!(
            resolve_backend(BackendPref::Auto, true),
            BackendSelection { app: TerminalApp::Iterm2, fallback: None }
        );
        assert_eq!(
            resolve_backend(BackendPref::Auto, false),
            BackendSelection { app: TerminalApp::TerminalApp, fallback: None }
        );
    }

    #[test]
    fn explicit_iterm2_falls_back_with_reason() {
        assert_eq!(
            resolve_backend(BackendPref::Iterm2, true),
            BackendSelection { app: TerminalApp::Iterm2, fallback: None }
        );
        assert_eq!(
            resolve_backend(BackendPref::Iterm2, false),
            BackendSelection {
                app: TerminalApp::TerminalApp,
                fallback: Some(FallbackReason::Iterm2NotInstalled)
            }
        );
    }

    #[test]
    fn explicit_terminal_app_ignores_iterm2() {
        for installed in [true, false] {
            assert_eq!(
                resolve_backend(BackendPref::TerminalApp, installed),
                BackendSelection { app: TerminalApp::TerminalApp, fallback: None }
            );
        }
    }

    #[test]
    fn pref_and_selection_use_camel_case() {
        assert_eq!(BackendPref::default(), BackendPref::Auto);
        assert_eq!(
            serde_json::from_value::<BackendPref>(json!("terminalApp")).unwrap(),
            BackendPref::TerminalApp
        );
        assert_eq!(
            serde_json::to_value(resolve_backend(BackendPref::Iterm2, false)).unwrap(),
            json!({ "app": "terminalApp", "fallback": "iterm2NotInstalled" })
        );
    }

    #[test]
    fn bundle_ids() {
        assert_eq!(TerminalApp::Iterm2.bundle_id(), "com.googlecode.iterm2");
        assert_eq!(TerminalApp::TerminalApp.bundle_id(), "com.apple.Terminal");
    }

    /// 本機手動：`cargo test -- --ignored detect --nocapture`
    #[test]
    #[ignore = "查詢本機實際的安裝與執行狀態"]
    fn detect_report_local_state() {
        for app in [TerminalApp::Iterm2, TerminalApp::TerminalApp] {
            println!(
                "{:?}: installed={} running={}",
                app,
                is_installed(app),
                is_running(app)
            );
        }
        // Terminal.app 是系統內建，一定裝著。
        assert!(is_installed(TerminalApp::TerminalApp));
        println!("auto => {:?}", select_backend(BackendPref::Auto));
    }
}
