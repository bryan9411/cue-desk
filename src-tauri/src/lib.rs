pub mod attribution;
pub mod commands;
pub mod config;
pub mod discovery;
pub mod error;
pub mod layout;
pub mod model;
pub mod osascript;
pub mod process;
pub mod registry;
pub mod shell;
pub mod state;
pub mod terminal;

use tauri::{Manager, RunEvent};

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::load())
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::focus_session,
            commands::add_workspace,
            commands::set_workspace_collapsed,
            commands::create_session,
        ])
        .setup(|app| {
            discovery::spawn(app.handle().clone());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|handle, event| {
        // 結束前寫完尚未寫入的設定。不關閉任何終端機工作階段。
        if let RunEvent::Exit = event {
            handle.state::<AppState>().flush_config();
        }
    });
}
