use tauri::State;
use crate::state::AppState;
use crate::storage::{config, dictionary};
use crate::types::AppSettings;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    let settings = state.settings.read().map_err(|e| e.to_string())?;
    Ok(settings.clone())
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, new_settings: AppSettings) -> Result<(), String> {
    config::save_settings(&new_settings)?;
    let mut settings = state.settings.write().map_err(|e| e.to_string())?;
    *settings = new_settings;
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
