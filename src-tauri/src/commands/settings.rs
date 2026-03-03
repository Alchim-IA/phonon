use tauri::{AppHandle, Emitter, State};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use crate::hotkeys::parse_hotkey;
use crate::state::AppState;
use crate::storage::{config, dictionary};
use crate::types::AppSettings;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.read().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    new_settings: AppSettings
) -> Result<(), String> {
    let old_settings = state.settings.read().map_err(|e| e.to_string())?.clone();
    let ptt_hotkey_changed = old_settings.hotkey_push_to_talk != new_settings.hotkey_push_to_talk;
    let translate_hotkey_changed = old_settings.hotkey_translate != new_settings.hotkey_translate;
    let translation_enabled_changed = old_settings.translation_enabled != new_settings.translation_enabled;
    let engine_type_changed = old_settings.engine_type != new_settings.engine_type;

    config::save_settings(&new_settings)?;

    {
        let mut settings = state.settings.write().map_err(|e| e.to_string())?;
        *settings = new_settings.clone();
    }

    if engine_type_changed {
        if let Err(e) = state.switch_engine_type(new_settings.engine_type) {
            log::warn!("Failed to switch engine type: {}. Model may need to be downloaded first.", e);
        }
    }

    if ptt_hotkey_changed {
        if let Err(e) = update_shortcut(&app, &old_settings.hotkey_push_to_talk, &new_settings.hotkey_push_to_talk) {
            log::warn!("Failed to update PTT shortcut: {}. Restart may be required.", e);
        }
    }

    if translation_enabled_changed || translate_hotkey_changed {
        if old_settings.translation_enabled {
            if let Some(old_shortcut) = parse_hotkey(&old_settings.hotkey_translate) {
                let _ = app.global_shortcut().unregister(old_shortcut);
            }
        }
        if new_settings.translation_enabled {
            if let Some(new_shortcut) = parse_hotkey(&new_settings.hotkey_translate) {
                if let Err(e) = app.global_shortcut().register(new_shortcut) {
                    log::warn!("Failed to register translate shortcut: {}", e);
                } else {
                    log::info!("Translate shortcut registered: {}", new_settings.hotkey_translate);
                }
            }
        }
    }

    // Notify all windows of settings change (for subtitles, floating window, etc.)
    let _ = app.emit("settings-changed", &new_settings);

    Ok(())
}

/// Met à jour un raccourci dynamiquement
fn update_shortcut(app: &AppHandle, old_hotkey: &str, new_hotkey: &str) -> Result<(), String> {
    if let Some(old_shortcut) = parse_hotkey(old_hotkey) {
        let _ = app.global_shortcut().unregister(old_shortcut);
    }

    if let Some(new_shortcut) = parse_hotkey(new_hotkey) {
        app.global_shortcut()
            .register(new_shortcut)
            .map_err(|e| format!("Failed to register new shortcut: {}", e))?;
        log::info!("PTT shortcut updated to: {}", new_hotkey);
    } else {
        return Err(format!("Invalid hotkey format: {}", new_hotkey));
    }

    Ok(())
}

#[tauri::command]
pub fn get_dictionary() -> Result<Vec<String>, String> {
    Ok(dictionary::load_dictionary().words)
}

#[tauri::command]
pub fn add_dictionary_word(word: String) -> Result<(), String> {
    dictionary::add_word(word)
}

#[tauri::command]
pub fn remove_dictionary_word(word: String) -> Result<(), String> {
    dictionary::remove_word(&word)
}
