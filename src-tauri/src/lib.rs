pub mod attribution;
pub mod commands;
pub mod config;
pub mod discovery;
pub mod dispatch;
pub mod error;
pub mod git;
pub mod layout;
pub mod model;
pub mod osascript;
pub mod process;
pub mod registry;
pub mod shell;
pub mod state;
pub mod terminal;

use tauri::{Emitter, Manager, RunEvent};

use crate::git::GIT_BRANCH_CHANGED;
use crate::state::{lock, AppState};

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
            commands::classify_paths,
            commands::set_workspace_collapsed,
            commands::create_session,
            commands::split_zsh,
            commands::rename_session,
            commands::close_session,
            commands::reopen_session,
            commands::remove_node,
            commands::set_as_workspace,
            commands::ignore_session,
            commands::remove_workspace,
            commands::dispatch_files,
        ])
        .setup(|app| {
            let state = app.state::<AppState>();
            state.sync_git(&lock(&state.config));
            let handle = app.handle().clone();
            state.git.start(move |change| {
                if let Err(e) = handle.emit(GIT_BRANCH_CHANGED, &change) {
                    eprintln!("[cue] 無法送出 {GIT_BRANCH_CHANGED}：{e}");
                }
            });
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
