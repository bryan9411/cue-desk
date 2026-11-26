pub mod error;
pub mod osascript;

pub mod terminal {
    /// 固定的 AppleScript 腳本原文。變動資料一律經由 `on run argv` 傳入。
    pub mod scripts {
        /// 列出 iTerm2 所有窗格；iTerm2 未執行時回傳空字串。
        pub const ITERM2_DISCOVER: &str =
            include_str!("terminal/scripts/iterm2_discover.applescript");
        /// 列出 Terminal.app 所有分頁；Terminal.app 未執行時回傳空字串。
        pub const TERMINAL_APP_DISCOVER: &str =
            include_str!("terminal/scripts/terminal_app_discover.applescript");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
