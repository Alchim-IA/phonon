use crate::storage::translation_history;
use crate::types::TranslationEntry;

#[tauri::command]
pub fn get_translation_history() -> Vec<TranslationEntry> {
    translation_history::load_translation_history().translations
}

#[tauri::command]
pub fn clear_translation_history() -> Result<(), String> {
    translation_history::clear_translation_history()
}
