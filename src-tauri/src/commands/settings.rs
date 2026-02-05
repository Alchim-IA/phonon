use tauri::{AppHandle, State};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
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
    // Récupérer les anciens settings pour comparer les raccourcis
    let old_settings = state.settings.read().map_err(|e| e.to_string())?.clone();
    let ptt_hotkey_changed = old_settings.hotkey_push_to_talk != new_settings.hotkey_push_to_talk;
    let translate_hotkey_changed = old_settings.hotkey_translate != new_settings.hotkey_translate;
    let translation_enabled_changed = old_settings.translation_enabled != new_settings.translation_enabled;
    let engine_type_changed = old_settings.engine_type != new_settings.engine_type;

    // Sauvegarder les nouveaux settings
    config::save_settings(&new_settings)?;

    // Mettre à jour l'état
    {
        let mut settings = state.settings.write().map_err(|e| e.to_string())?;
        *settings = new_settings.clone();
    }

    // Si le type d'engine a changé, recharger l'engine
    if engine_type_changed {
        if let Err(e) = state.switch_engine_type(new_settings.engine_type) {
            log::warn!("Failed to switch engine type: {}. Model may need to be downloaded first.", e);
        }
    }

    // Si le raccourci PTT a changé, le réenregistrer
    if ptt_hotkey_changed {
        if let Err(e) = update_shortcut(&app, &old_settings.hotkey_push_to_talk, &new_settings.hotkey_push_to_talk) {
            log::warn!("Failed to update PTT shortcut: {}. Restart may be required.", e);
        }
    }

    // Gérer le raccourci de traduction
    if translation_enabled_changed || translate_hotkey_changed {
        // Désenregistrer l'ancien si existait
        if old_settings.translation_enabled {
            if let Some(old_shortcut) = parse_hotkey_internal(&old_settings.hotkey_translate) {
                let _ = app.global_shortcut().unregister(old_shortcut);
            }
        }
        // Enregistrer le nouveau si activé
        if new_settings.translation_enabled {
            if let Some(new_shortcut) = parse_hotkey_internal(&new_settings.hotkey_translate) {
                if let Err(e) = app.global_shortcut().register(new_shortcut) {
                    log::warn!("Failed to register translate shortcut: {}", e);
                } else {
                    log::info!("Translate shortcut registered: {}", new_settings.hotkey_translate);
                }
            }
        }
    }

    Ok(())
}

/// Met à jour un raccourci dynamiquement
fn update_shortcut(app: &AppHandle, old_hotkey: &str, new_hotkey: &str) -> Result<(), String> {
    // Parser l'ancien raccourci
    if let Some(old_shortcut) = parse_hotkey_internal(old_hotkey) {
        // Désenregistrer l'ancien
        let _ = app.global_shortcut().unregister(old_shortcut);
    }

    // Parser et enregistrer le nouveau
    if let Some(new_shortcut) = parse_hotkey_internal(new_hotkey) {
        app.global_shortcut()
            .register(new_shortcut)
            .map_err(|e| format!("Failed to register new shortcut: {}", e))?;
        log::info!("PTT shortcut updated to: {}", new_hotkey);
    } else {
        return Err(format!("Invalid hotkey format: {}", new_hotkey));
    }

    Ok(())
}

/// Parse un raccourci clavier depuis un format string (ex: "Ctrl+Shift+R")
fn parse_hotkey_internal(hotkey: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = hotkey.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        let part_lower = part.trim().to_lowercase();
        match part_lower.as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "cmd" | "command" | "meta" => modifiers |= Modifiers::META,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            _ => {
                key_code = match part.to_uppercase().as_str() {
                    "A" => Some(Code::KeyA),
                    "B" => Some(Code::KeyB),
                    "C" => Some(Code::KeyC),
                    "D" => Some(Code::KeyD),
                    "E" => Some(Code::KeyE),
                    "F" => Some(Code::KeyF),
                    "G" => Some(Code::KeyG),
                    "H" => Some(Code::KeyH),
                    "I" => Some(Code::KeyI),
                    "J" => Some(Code::KeyJ),
                    "K" => Some(Code::KeyK),
                    "L" => Some(Code::KeyL),
                    "M" => Some(Code::KeyM),
                    "N" => Some(Code::KeyN),
                    "O" => Some(Code::KeyO),
                    "P" => Some(Code::KeyP),
                    "Q" => Some(Code::KeyQ),
                    "R" => Some(Code::KeyR),
                    "S" => Some(Code::KeyS),
                    "T" => Some(Code::KeyT),
                    "U" => Some(Code::KeyU),
                    "V" => Some(Code::KeyV),
                    "W" => Some(Code::KeyW),
                    "X" => Some(Code::KeyX),
                    "Y" => Some(Code::KeyY),
                    "Z" => Some(Code::KeyZ),
                    "0" => Some(Code::Digit0),
                    "1" => Some(Code::Digit1),
                    "2" => Some(Code::Digit2),
                    "3" => Some(Code::Digit3),
                    "4" => Some(Code::Digit4),
                    "5" => Some(Code::Digit5),
                    "6" => Some(Code::Digit6),
                    "7" => Some(Code::Digit7),
                    "8" => Some(Code::Digit8),
                    "9" => Some(Code::Digit9),
                    "SPACE" => Some(Code::Space),
                    "ENTER" | "RETURN" => Some(Code::Enter),
                    "TAB" => Some(Code::Tab),
                    "ESCAPE" | "ESC" => Some(Code::Escape),
                    "F1" => Some(Code::F1),
                    "F2" => Some(Code::F2),
                    "F3" => Some(Code::F3),
                    "F4" => Some(Code::F4),
                    "F5" => Some(Code::F5),
                    "F6" => Some(Code::F6),
                    "F7" => Some(Code::F7),
                    "F8" => Some(Code::F8),
                    "F9" => Some(Code::F9),
                    "F10" => Some(Code::F10),
                    "F11" => Some(Code::F11),
                    "F12" => Some(Code::F12),
                    _ => None,
                };
            }
        }
    }

    key_code.map(|code| {
        if modifiers.is_empty() {
            Shortcut::new(None, code)
        } else {
            Shortcut::new(Some(modifiers), code)
        }
    })
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
