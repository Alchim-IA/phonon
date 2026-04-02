pub mod app_log;
pub mod config;
pub mod dictionary;
pub mod history;
pub mod snippets;
pub mod stats;
pub mod translation_history;

use std::path::PathBuf;

pub fn get_app_data_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("com.wakastellar.wakascribe")
}

pub fn ensure_app_data_dir() -> std::io::Result<PathBuf> {
    let dir = get_app_data_dir();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}
