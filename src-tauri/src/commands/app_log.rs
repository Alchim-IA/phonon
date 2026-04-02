use crate::storage::app_log;

#[tauri::command]
pub fn get_app_logs(limit: Option<usize>) -> Vec<app_log::LogEntry> {
    app_log::get_logs(limit)
}

#[tauri::command]
pub fn clear_app_logs() -> Result<(), String> {
    app_log::clear_logs()
}

#[tauri::command]
pub fn get_log_file_path() -> String {
    app_log::log_path_string()
}
