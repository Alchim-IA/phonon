mod audio;
mod commands;
mod engines;
mod hotkeys;
mod llm;
mod platform;
mod ptt;
mod state;
mod storage;
mod tray;
mod types;
mod voice_commands;

pub use audio::AudioCapture;
pub use types::*;

use llm::LocalLlmEngine;
use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let settings = storage::config::load_settings();
    log::info!("[PTT] Using hotkey: {}", settings.hotkey_push_to_talk);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    ptt::handle_shortcut(app, shortcut, &event);
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::list_audio_devices,
            commands::start_mic_preview,
            commands::stop_mic_preview,
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
            commands::reset_recording_state,
            commands::get_available_models,
            commands::get_current_model,
            commands::download_model,
            commands::delete_model,
            commands::switch_model,
            commands::is_engine_ready,
            commands::get_vosk_models,
            commands::download_vosk_model,
            commands::select_vosk_language,
            commands::switch_engine_type,
            commands::is_parakeet_available,
            commands::get_parakeet_models,
            commands::download_parakeet_model,
            commands::delete_parakeet_model,
            commands::select_parakeet_model,
            commands::set_groq_api_key,
            commands::get_groq_api_key,
            commands::has_groq_api_key,
            commands::validate_groq_api_key,
            commands::delete_groq_api_key,
            commands::get_groq_quota,
            commands::translate_text,
            commands::summarize_text,
            commands::is_llm_model_available,
            commands::get_available_llm_models,
            commands::download_llm_model,
            commands::delete_llm_model,
            commands::summarize_text_local,
            commands::summarize_text_smart,
            commands::auto_paste,
            commands::show_floating_window,
            commands::hide_floating_window,
            commands::toggle_floating_window,
            commands::set_floating_window_size,
            commands::get_floating_window_position,
            commands::set_floating_window_position,
            commands::show_subtitles_window,
            commands::hide_subtitles_window,
            commands::toggle_subtitles,
            commands::file_transcription::transcribe_files,
            commands::file_transcription::get_supported_audio_formats,
            commands::get_snippets,
            commands::add_snippet,
            commands::update_snippet,
            commands::remove_snippet,
            commands::get_usage_stats,
            commands::reset_stats,
            commands::send_to_apple_notes,
            commands::send_to_obsidian,
        ])
        .setup(|app| {
            // Initialiser l'état
            let app_state = match AppState::new(app.handle()) {
                Ok(state) => state,
                Err(e) => {
                    log::error!("Failed to initialize app state: {}", e);
                    return Err(e.into());
                }
            };

            let model_manager = app_state.model_manager.clone();
            app.manage(app_state);
            app.manage(model_manager);

            let llm_engine: Arc<RwLock<Option<LocalLlmEngine>>> = Arc::new(RwLock::new(None));
            app.manage(llm_engine);

            // Initialiser les threads audio
            ptt::init_ptt_audio_thread();
            commands::transcription::init_gui_audio_thread();

            // Enregistrer les raccourcis globaux
            ptt::setup_shortcuts(app)?;

            // Construire le tray icon
            tray::build_tray(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    api.prevent_close();
                    let _ = window.hide();
                    log::debug!("[WINDOW] Main window hidden instead of closed");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
