use crate::audio::AudioCapture;
use crate::types::AudioDevice;

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    AudioCapture::list_devices()
}
