use std::sync::{Arc, RwLock};
use crate::storage::config;
use crate::types::AppSettings;

/// Thread-safe application state for Tauri
/// Note: AudioCapture is not stored here because cpal::Stream is not Send+Sync.
/// Audio capture is managed per-command in transcription.rs using thread_local storage.
pub struct AppState {
    pub is_recording: Arc<RwLock<bool>>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub sample_rate: Arc<RwLock<u32>>,
}

impl AppState {
    pub fn new() -> Result<Self, String> {
        let settings = config::load_settings();

        Ok(Self {
            is_recording: Arc::new(RwLock::new(false)),
            settings: Arc::new(RwLock::new(settings)),
            sample_rate: Arc::new(RwLock::new(44100)),
        })
    }
}
