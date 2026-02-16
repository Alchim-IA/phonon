use crate::audio::AudioCapture;
use crate::types::AudioDevice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use serde::Serialize;

static MIC_PREVIEW_ACTIVE: AtomicBool = AtomicBool::new(false);
static MIC_PREVIEW_DEVICE: Mutex<Option<String>> = Mutex::new(None);

#[derive(Clone, Serialize)]
struct MicLevelEvent {
    levels: Vec<f32>,
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    AudioCapture::list_devices()
}

#[tauri::command]
pub fn start_mic_preview(app: AppHandle, device_id: Option<String>) -> Result<(), String> {
    // Stop any existing preview
    MIC_PREVIEW_ACTIVE.store(false, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(100));

    MIC_PREVIEW_ACTIVE.store(true, Ordering::SeqCst);
    if let Ok(mut guard) = MIC_PREVIEW_DEVICE.lock() {
        *guard = device_id.clone();
    }

    std::thread::spawn(move || {
        let mut capture = match AudioCapture::new(device_id.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Mic preview: failed to create capture: {}", e);
                return;
            }
        };

        if let Err(e) = capture.start(device_id.as_deref()) {
            log::error!("Mic preview: failed to start capture: {}", e);
            return;
        }

        log::info!("Mic preview started");
        let num_bars: usize = 32;

        while MIC_PREVIEW_ACTIVE.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));

            let (buffer, _sample_rate) = capture.get_audio_snapshot();
            let len = buffer.len();

            // Take only the last ~2400 samples (~50ms at 48kHz)
            let recent_start = len.saturating_sub(2400);
            let recent = &buffer[recent_start..];

            let mut levels = Vec::with_capacity(num_bars);
            if recent.is_empty() {
                levels.resize(num_bars, 0.0);
            } else {
                let chunk_size = recent.len() / num_bars;
                if chunk_size == 0 {
                    levels.resize(num_bars, 0.0);
                } else {
                    for i in 0..num_bars {
                        let start = i * chunk_size;
                        let end = (start + chunk_size).min(recent.len());
                        let rms: f32 = (recent[start..end]
                            .iter()
                            .map(|s| s * s)
                            .sum::<f32>()
                            / (end - start) as f32)
                            .sqrt();
                        // Normalize: typical mic RMS is 0.0-0.3, amplify for visual
                        levels.push((rms * 5.0).min(1.0));
                    }
                }
            }

            let _ = app.emit("mic-level", MicLevelEvent { levels });
        }

        // Cleanup
        let _ = capture.stop();
        log::info!("Mic preview stopped");
    });

    Ok(())
}

#[tauri::command]
pub fn stop_mic_preview() -> Result<(), String> {
    MIC_PREVIEW_ACTIVE.store(false, Ordering::SeqCst);
    Ok(())
}
