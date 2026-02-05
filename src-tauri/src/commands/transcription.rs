use std::sync::{mpsc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};
use serde::Serialize;
use crate::engines::SpeechEngine;
use crate::state::AppState;
use crate::storage::history;
use crate::types::TranscriptionResult;
use crate::audio::AudioCapture;
use crate::voice_commands;
use crate::llm;

/// Taux d'échantillonnage requis par Whisper
const TARGET_SAMPLE_RATE: u32 = 16000;

/// État global pour le streaming
static STREAMING_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Channels pour communiquer avec le thread audio
static AUDIO_CMD_SENDER: Mutex<Option<mpsc::Sender<AudioCommand>>> = Mutex::new(None);
static AUDIO_RESULT_RECEIVER: Mutex<Option<mpsc::Receiver<AudioResult>>> = Mutex::new(None);

/// Commandes pour le thread audio
#[derive(Debug)]
enum AudioCommand {
    Start { device_id: Option<String> },
    Stop,
}

/// Résultats du thread audio
#[derive(Debug)]
struct AudioResult {
    audio: Vec<f32>,
    sample_rate: u32,
}

/// Payload pour les événements de streaming
#[derive(Clone, Serialize)]
pub struct StreamingChunkEvent {
    pub text: String,
    pub is_final: bool,
    pub duration_seconds: f32,
}

/// Émet un événement de statut d'enregistrement
fn emit_recording_status(app: &AppHandle, status: &str) {
    let _ = app.emit("recording-status", status);
}

/// Émet un chunk de transcription streaming
fn emit_streaming_chunk(app: &AppHandle, chunk: StreamingChunkEvent) {
    let _ = app.emit("transcription-chunk", chunk);
}

/// Initialise le thread audio dédié pour les commandes de transcription GUI
pub fn init_gui_audio_thread() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<AudioCommand>();
    let (result_tx, result_rx) = mpsc::channel::<AudioResult>();

    // Stocker les channels
    if let Ok(mut guard) = AUDIO_CMD_SENDER.lock() {
        *guard = Some(cmd_tx);
    }
    if let Ok(mut guard) = AUDIO_RESULT_RECEIVER.lock() {
        *guard = Some(result_rx);
    }

    // Thread audio dédié
    std::thread::spawn(move || {
        log::info!("GUI audio thread started");
        let mut capture: Option<AudioCapture> = None;

        loop {
            match cmd_rx.recv() {
                Ok(AudioCommand::Start { device_id }) => {
                    log::info!("GUI Audio: Starting capture (device: {:?})", device_id);
                    match AudioCapture::new(device_id.as_deref()) {
                        Ok(mut cap) => {
                            if let Err(e) = cap.start(device_id.as_deref()) {
                                log::error!("Failed to start audio capture: {}", e);
                                continue;
                            }
                            capture = Some(cap);
                            log::info!("GUI Audio: Capture started successfully");
                        }
                        Err(e) => {
                            log::error!("Failed to create audio capture: {}", e);
                        }
                    }
                }
                Ok(AudioCommand::Stop) => {
                    log::info!("GUI Audio: Stopping capture");
                    if let Some(mut cap) = capture.take() {
                        match cap.stop() {
                            Ok((audio, sample_rate)) => {
                                log::info!("GUI Audio: Captured {} samples at {}Hz", audio.len(), sample_rate);
                                let _ = result_tx.send(AudioResult { audio, sample_rate });
                            }
                            Err(e) => {
                                log::error!("Failed to stop audio capture: {}", e);
                                // Send empty result to unblock caller
                                let _ = result_tx.send(AudioResult { audio: vec![], sample_rate: 16000 });
                            }
                        }
                    } else {
                        log::warn!("GUI Audio: No active capture to stop");
                        // Send empty result to unblock caller
                        let _ = result_tx.send(AudioResult { audio: vec![], sample_rate: 16000 });
                    }
                }
                Err(_) => {
                    log::info!("GUI audio thread: channel closed, exiting");
                    break;
                }
            }
        }
    });
}

#[tauri::command]
pub fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut is_recording = state.is_recording.write().map_err(|e| e.to_string())?;
    if *is_recording {
        return Err("Already recording".to_string());
    }

    let settings = state.settings.read().map_err(|e| e.to_string())?;
    let device_id = settings.microphone_id.clone();
    let streaming_enabled = settings.streaming_enabled;
    drop(settings);

    // Envoyer la commande de démarrage au thread audio
    {
        let guard = AUDIO_CMD_SENDER.lock().map_err(|e| e.to_string())?;
        if let Some(ref sender) = *guard {
            sender.send(AudioCommand::Start { device_id }).map_err(|e| e.to_string())?;
        } else {
            return Err("Audio thread not initialized".to_string());
        }
    }

    *is_recording = true;
    STREAMING_ACTIVE.store(streaming_enabled, Ordering::SeqCst);

    // Émettre le statut d'enregistrement
    emit_recording_status(&app, "recording");

    // Démarrer la tâche de streaming si activée
    if streaming_enabled {
        let app_clone = app.clone();
        std::thread::spawn(move || {
            run_streaming_task(app_clone);
        });
    }

    log::info!("Recording started (streaming: {})", streaming_enabled);
    Ok(())
}

/// Tâche de streaming qui émet des événements de progression pendant l'enregistrement
fn run_streaming_task(app: AppHandle) {
    log::info!("Streaming task started");

    let start_time = std::time::Instant::now();
    let mut last_emitted_duration = 0.0f32;

    while STREAMING_ACTIVE.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));

        if !STREAMING_ACTIVE.load(Ordering::SeqCst) {
            break;
        }

        let elapsed = start_time.elapsed().as_secs_f32();

        if elapsed - last_emitted_duration >= 0.5 {
            emit_streaming_chunk(&app, StreamingChunkEvent {
                text: String::new(),
                is_final: false,
                duration_seconds: elapsed,
            });
            last_emitted_duration = elapsed;
        }
    }

    log::info!("Streaming task ended after {:.1}s", start_time.elapsed().as_secs_f32());
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle, state: State<'_, AppState>) -> Result<TranscriptionResult, String> {
    // Arrêter la tâche de streaming
    STREAMING_ACTIVE.store(false, Ordering::SeqCst);

    // Émettre le statut "processing"
    emit_recording_status(&app, "processing");

    // Vérifier l'état d'enregistrement
    {
        let mut is_recording = state.is_recording.write().map_err(|e| e.to_string())?;
        if !*is_recording {
            emit_recording_status(&app, "idle");
            return Err("Not recording".to_string());
        }
        *is_recording = false;
    }

    // Envoyer la commande d'arrêt et attendre le résultat
    {
        let guard = AUDIO_CMD_SENDER.lock().map_err(|e| e.to_string())?;
        if let Some(ref sender) = *guard {
            sender.send(AudioCommand::Stop).map_err(|e| e.to_string())?;
        } else {
            emit_recording_status(&app, "idle");
            return Err("Audio thread not initialized".to_string());
        }
    }

    // Récupérer les données audio
    let audio_result = {
        let guard = AUDIO_RESULT_RECEIVER.lock().map_err(|e| e.to_string())?;
        if let Some(ref receiver) = *guard {
            match receiver.recv_timeout(std::time::Duration::from_secs(5)) {
                Ok(result) => result,
                Err(e) => {
                    emit_recording_status(&app, "idle");
                    return Err(format!("Failed to receive audio data: {}", e));
                }
            }
        } else {
            emit_recording_status(&app, "idle");
            return Err("Audio result receiver not initialized".to_string());
        }
    };

    let audio_buffer = audio_result.audio;
    let sample_rate = audio_result.sample_rate;

    if audio_buffer.is_empty() {
        emit_recording_status(&app, "idle");
        return Err("No audio captured".to_string());
    }

    let duration_seconds = audio_buffer.len() as f32 / sample_rate as f32;

    if duration_seconds < 0.5 {
        emit_recording_status(&app, "idle");
        return Err("Recording too short (minimum 0.5 seconds)".to_string());
    }

    log::info!("Audio received: {:.1}s at {}Hz", duration_seconds, sample_rate);

    // Resampling si nécessaire
    let resampled_audio = if sample_rate != TARGET_SAMPLE_RATE {
        log::info!("Resampling audio from {}Hz to {}Hz", sample_rate, TARGET_SAMPLE_RATE);
        resample_audio(&audio_buffer, sample_rate, TARGET_SAMPLE_RATE)
    } else {
        audio_buffer
    };

    // Transcription
    let result = {
        let engine_guard = state.engine.read().map_err(|e| e.to_string())?;
        let engine = engine_guard
            .as_ref()
            .ok_or("Whisper engine not initialized. Please download a model first.")?;
        engine.transcribe(&resampled_audio, TARGET_SAMPLE_RATE)?
    };

    // Lire les settings pour le post-processing
    let (voice_commands_enabled, dictation_mode, llm_enabled, llm_mode) = {
        let settings = state.settings.read().map_err(|e| e.to_string())?;
        (
            settings.voice_commands_enabled,
            settings.dictation_mode,
            settings.llm_enabled,
            settings.llm_mode,
        )
    };

    // Post-traitement
    let mut final_text = result.text.clone();

    // Voice commands
    if voice_commands_enabled {
        let parse_result = voice_commands::parse(&final_text, dictation_mode);
        final_text = parse_result.text;
        if !parse_result.actions.is_empty() {
            log::info!("Voice commands detected: {:?}", parse_result.actions);
        }
    }

    // LLM post-processing
    if llm_enabled {
        if let Some(api_key) = super::llm::get_groq_api_key_internal() {
            match llm::process(&final_text, llm_mode, dictation_mode, &api_key).await {
                Ok(processed) => {
                    log::info!("LLM processing successful");
                    final_text = processed;
                }
                Err(e) => {
                    log::warn!("LLM processing failed: {}", e);
                }
            }
        } else {
            log::warn!("LLM enabled but no API key found");
        }
    }

    // Créer le résultat final
    let final_result = TranscriptionResult {
        text: final_text.clone(),
        confidence: result.confidence,
        duration_seconds: result.duration_seconds,
        processing_time_ms: result.processing_time_ms,
        detected_language: result.detected_language,
        timestamp: result.timestamp,
        model_used: result.model_used,
    };

    // Émettre le chunk final
    emit_streaming_chunk(&app, StreamingChunkEvent {
        text: final_text,
        is_final: true,
        duration_seconds,
    });

    // Émettre le statut "idle"
    emit_recording_status(&app, "idle");

    history::add_transcription(final_result.clone())?;

    log::info!("Recording stopped, duration: {:.1}s", duration_seconds);
    Ok(final_result)
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

/// Réinitialise l'état d'enregistrement en cas de blocage
#[tauri::command]
pub fn reset_recording_state(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    log::info!("Resetting recording state");

    // Arrêter le streaming
    STREAMING_ACTIVE.store(false, Ordering::SeqCst);

    // Réinitialiser l'état
    let mut is_recording = state.is_recording.write().map_err(|e| e.to_string())?;
    *is_recording = false;

    // Émettre le statut idle
    emit_recording_status(&app, "idle");

    log::info!("Recording state reset complete");
    Ok(())
}

fn resample_audio(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return input.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = (input.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_len);

    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx_floor = src_idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(input.len() - 1);
        let frac = src_idx - idx_floor as f64;

        let sample = if idx_floor < input.len() {
            let s1 = input[idx_floor];
            let s2 = input[idx_ceil];
            s1 + (s2 - s1) * frac as f32
        } else {
            0.0
        };

        output.push(sample);
    }

    output
}
