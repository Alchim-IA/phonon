mod audio;
mod commands;
mod state;
mod storage;
mod types;

pub use audio::AudioCapture;
pub use types::*;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::list_audio_devices,
            commands::get_settings,
            commands::update_settings,
            commands::get_dictionary,
            commands::add_dictionary_word,
            commands::remove_dictionary_word,
            commands::start_recording,
            commands::stop_recording,
            commands::get_history,
            commands::clear_history,
            commands::get_recording_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
