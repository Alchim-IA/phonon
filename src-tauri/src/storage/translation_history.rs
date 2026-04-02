use crate::types::{TranslationEntry, TranslationHistoryData};
use std::fs;
use std::path::PathBuf;

const MAX_TRANSLATIONS: usize = 50;

fn translation_history_path() -> PathBuf {
    super::get_app_data_dir().join("translation_history.json")
}

pub fn load_translation_history() -> TranslationHistoryData {
    let path = translation_history_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        TranslationHistoryData::default()
    }
}

pub fn save_translation_history(data: &TranslationHistoryData) -> Result<(), String> {
    super::ensure_app_data_dir().map_err(|e| e.to_string())?;
    let path = translation_history_path();
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn add_translation(entry: TranslationEntry) -> Result<(), String> {
    let mut data = load_translation_history();
    data.translations.insert(0, entry);
    data.translations.truncate(MAX_TRANSLATIONS);
    save_translation_history(&data)
}

pub fn clear_translation_history() -> Result<(), String> {
    save_translation_history(&TranslationHistoryData::default())
}
