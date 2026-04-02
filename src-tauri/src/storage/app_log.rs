use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MAX_LOG_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: i64,
    pub level: String,
    pub category: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppLogData {
    pub entries: Vec<LogEntry>,
}

fn log_path() -> PathBuf {
    super::get_app_data_dir().join("app_log.json")
}

pub fn log_path_string() -> String {
    log_path().to_string_lossy().to_string()
}

fn load_log_data() -> AppLogData {
    let path = log_path();
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppLogData::default()
    }
}

fn save_log_data(data: &AppLogData) -> Result<(), String> {
    super::ensure_app_data_dir().map_err(|e| e.to_string())?;
    let path = log_path();
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn append_log(level: &str, category: &str, message: &str) {
    let entry = LogEntry {
        timestamp: chrono::Utc::now().timestamp(),
        level: level.to_string(),
        category: category.to_string(),
        message: message.to_string(),
    };
    let mut data = load_log_data();
    data.entries.insert(0, entry);
    data.entries.truncate(MAX_LOG_ENTRIES);
    let _ = save_log_data(&data);
}

pub fn get_logs(limit: Option<usize>) -> Vec<LogEntry> {
    let data = load_log_data();
    let limit = limit.unwrap_or(100);
    data.entries.into_iter().take(limit).collect()
}

pub fn clear_logs() -> Result<(), String> {
    save_log_data(&AppLogData::default())
}
