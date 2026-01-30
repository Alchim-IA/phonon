use tauri::State;
use crate::state::AppState;
use crate::storage::history;
use crate::types::TranscriptionResult;
use crate::audio::AudioCapture;
use std::time::Instant;
use std::cell::RefCell;

// Thread-local audio capture to avoid Send+Sync requirements on cpal::Stream
// This works because Tauri commands run on the main thread by default
thread_local! {
    static AUDIO_CAPTURE: RefCell<Option<AudioCapture>> = RefCell::new(None);
}

#[tauri::command]
pub fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    let mut is_recording = state.is_recording.write().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    let settings = state.settings.read().map_err(|e| e.to_string())?;
    let device_id = settings.microphone_id.clone();
    drop(settings);

    // Create and start audio capture
    let mut capture = AudioCapture::new(device_id.as_deref())?;
    capture.start(device_id.as_deref())?;

    // Store sample rate in state
    {
        let mut sr = state.sample_rate.write().map_err(|e| e.to_string())?;
        *sr = capture.sample_rate();
    }

    // Store capture in thread-local storage
    AUDIO_CAPTURE.with(|cell| {
        *cell.borrow_mut() = Some(capture);
    });

    *is_recording = true;
    log::info!("Recording started");
    Ok(())
}

#[tauri::command]
pub fn stop_recording(state: State<'_, AppState>) -> Result<TranscriptionResult, String> {
    let mut is_recording = state.is_recording.write().map_err(|e| e.to_string())?;
    if !*is_recording {
        return Err("Not recording".to_string());
    }

    let start = Instant::now();

    // Stop capture and get audio data from thread-local storage
    let (audio_buffer, sample_rate) = AUDIO_CAPTURE.with(|cell| -> Result<(Vec<f32>, u32), String> {
        let mut capture_opt = cell.borrow_mut();
        if let Some(ref mut capture) = *capture_opt {
            let result = capture.stop()?;
            *capture_opt = None;
            Ok(result)
        } else {
            Err("No active capture".to_string())
        }
    })?;

    *is_recording = false;

    let duration_seconds = audio_buffer.len() as f32 / sample_rate as f32;

    if duration_seconds < 0.5 {
        return Err("Recording too short (minimum 0.5 seconds)".to_string());
    }

    // TODO: Intégrer le vrai moteur STT ici
    let result = TranscriptionResult {
        text: format!("[Mock] Audio capturé: {:.1}s à {}Hz", duration_seconds, sample_rate),
        confidence: 0.95,
        duration_seconds,
        processing_time_ms: start.elapsed().as_millis() as u64,
        detected_language: Some("fr".to_string()),
        timestamp: chrono::Utc::now().timestamp(),
    };

    history::add_transcription(result.clone())?;

    log::info!("Recording stopped, duration: {:.1}s", duration_seconds);
    Ok(result)
}

#[tauri::command]
pub fn get_history() -> Result<Vec<TranscriptionResult>, String> {
    Ok(history::load_history().transcriptions)
}

#[tauri::command]
pub fn clear_history() -> Result<(), String> {
    history::clear_history()
}

#[tauri::command]
pub fn get_recording_status(state: State<'_, AppState>) -> Result<bool, String> {
    let is_recording = state.is_recording.read().map_err(|e| e.to_string())?;
    Ok(*is_recording)
}
