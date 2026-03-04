use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::audio::AudioCapture;
use crate::hotkeys::parse_hotkey;
use crate::platform::{copy_selected_text, paste_text, type_text_incremental};
use crate::state::AppState;
use crate::storage;
use crate::tray::{set_tray_recording, set_tray_state, TrayState};

/// Taux d'échantillonnage requis par le modèle
const TARGET_SAMPLE_RATE: u32 = 16000;

// Raccourcis globaux
static PTT_SHORTCUT: Mutex<Option<Shortcut>> = Mutex::new(None);
static TRANSLATE_SHORTCUT: Mutex<Option<Shortcut>> = Mutex::new(None);
static VOICE_ACTION_SHORTCUT: Mutex<Option<Shortcut>> = Mutex::new(None);

// État global pour le push-to-talk
static IS_PTT_ACTIVE: AtomicBool = AtomicBool::new(false);
static IS_VOICE_ACTION_ACTIVE: AtomicBool = AtomicBool::new(false);
static SELECTED_TEXT_FOR_ACTION: Mutex<String> = Mutex::new(String::new());

// Channel pour envoyer les données audio du thread d'enregistrement
static PTT_AUDIO_SENDER: Mutex<Option<mpsc::Sender<PttCommand>>> = Mutex::new(None);
static PTT_AUDIO_RECEIVER: Mutex<Option<mpsc::Receiver<PttResult>>> = Mutex::new(None);

// Pour le streaming temps réel
static STREAMING_TEXT: Mutex<String> = Mutex::new(String::new());

#[derive(Debug)]
enum PttCommand {
    Start,
    Stop,
    GetSnapshot,
}

#[derive(Debug)]
enum PttResult {
    AudioComplete { audio: Vec<f32>, sample_rate: u32 },
    AudioSnapshot { audio: Vec<f32>, sample_rate: u32 },
}

/// Initialise le thread audio pour le push-to-talk
pub fn init_ptt_audio_thread() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<PttCommand>();
    let (result_tx, result_rx) = mpsc::channel::<PttResult>();

    if let Ok(mut guard) = PTT_AUDIO_SENDER.lock() {
        *guard = Some(cmd_tx);
    }
    if let Ok(mut guard) = PTT_AUDIO_RECEIVER.lock() {
        *guard = Some(result_rx);
    }

    std::thread::spawn(move || {
        log::info!("PTT audio thread started");
        let mut capture: Option<AudioCapture> = None;

        loop {
            match cmd_rx.recv() {
                Ok(PttCommand::Start) => {
                    log::info!("PTT: Starting audio capture");
                    match AudioCapture::new(None) {
                        Ok(mut cap) => {
                            if let Err(e) = cap.start(None) {
                                log::error!("Failed to start audio capture: {}", e);
                                continue;
                            }
                            capture = Some(cap);
                        }
                        Err(e) => {
                            log::error!("Failed to create audio capture: {}", e);
                        }
                    }
                }
                Ok(PttCommand::GetSnapshot) => {
                    if let Some(ref cap) = capture {
                        let (audio, sample_rate) = cap.get_audio_snapshot();
                        let _ = result_tx.send(PttResult::AudioSnapshot { audio, sample_rate });
                    }
                }
                Ok(PttCommand::Stop) => {
                    log::info!("PTT: Stopping audio capture");
                    if let Some(mut cap) = capture.take() {
                        match cap.stop() {
                            Ok((audio, sample_rate)) => {
                                let _ = result_tx.send(PttResult::AudioComplete { audio, sample_rate });
                            }
                            Err(e) => {
                                log::error!("Failed to stop audio capture: {}", e);
                            }
                        }
                    }
                }
                Err(_) => {
                    log::info!("PTT audio thread: channel closed, exiting");
                    break;
                }
            }
        }
    });
}

/// Démarre l'enregistrement audio via cpal
fn start_ptt_recording() {
    log::info!("[PTT] start_ptt_recording() called");
    if let Ok(guard) = PTT_AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            let _ = sender.send(PttCommand::Start);
        } else {
            log::error!("[PTT] audio sender not initialized");
        }
    }
}

/// Streaming temps réel : transcrit et tape le texte pendant l'enregistrement
fn start_streaming_transcription(app: &tauri::AppHandle) {
    log::info!("[STREAMING] Starting streaming transcription");

    let settings = storage::config::load_settings();
    if !settings.streaming_enabled {
        log::info!("[STREAMING] Streaming disabled in settings");
        return;
    }

    const STREAMING_INTERVAL_MS: u64 = 1000;
    let mut last_text_len = 0;

    while IS_PTT_ACTIVE.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(STREAMING_INTERVAL_MS));

        if !IS_PTT_ACTIVE.load(Ordering::SeqCst) {
            break;
        }

        if let Ok(guard) = PTT_AUDIO_SENDER.lock() {
            if let Some(ref sender) = *guard {
                let _ = sender.send(PttCommand::GetSnapshot);
            }
        }

        let snapshot = if let Ok(guard) = PTT_AUDIO_RECEIVER.lock() {
            if let Some(ref receiver) = *guard {
                match receiver.recv_timeout(std::time::Duration::from_millis(500)) {
                    Ok(PttResult::AudioSnapshot { audio, sample_rate }) => Some((audio, sample_rate)),
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let (audio_data, sample_rate) = match snapshot {
            Some(data) => data,
            None => continue,
        };

        let duration = audio_data.len() as f32 / sample_rate as f32;
        if duration < 1.0 {
            continue;
        }

        // Audio processing: pre-resample
        let audio_data = {
            let state: tauri::State<'_, AppState> = app.state();
            let result = if let Ok(mut proc) = state.audio_processor.write() {
                proc.process_pre_resample(&audio_data, sample_rate)
            } else {
                audio_data
            };
            result
        };

        let resampled = if sample_rate != TARGET_SAMPLE_RATE {
            crate::audio::resampling::resample_audio(&audio_data, sample_rate, TARGET_SAMPLE_RATE)
        } else {
            audio_data
        };

        // Audio processing: post-resample
        let (resampled, has_speech) = {
            let state: tauri::State<'_, AppState> = app.state();
            let result = if let Ok(mut proc) = state.audio_processor.write() {
                proc.process_post_resample(&resampled)
            } else {
                (resampled, true)
            };
            result
        };

        if !has_speech {
            continue;
        }

        let state: tauri::State<'_, AppState> = app.state();
        let engine_guard = match state.engine.read() {
            Ok(guard) => guard,
            Err(_) => continue,
        };
        let engine = match engine_guard.as_ref() {
            Some(e) => e,
            None => continue,
        };
        let result = match engine.transcribe(&resampled, TARGET_SAMPLE_RATE) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("[STREAMING] Transcription error: {}", e);
                continue;
            }
        };

        if result.text.is_empty() {
            continue;
        }

        log::info!("[STREAMING] Transcribed: '{}'", result.text);

        #[derive(serde::Serialize, Clone)]
        struct StreamingChunk {
            text: String,
            is_final: bool,
            duration_seconds: f32,
        }
        let chunk = StreamingChunk {
            text: result.text.clone(),
            is_final: false,
            duration_seconds: duration,
        };
        let _ = app.emit("transcription-chunk", chunk);

        let current_text = result.text.trim();
        if current_text.len() > last_text_len {
            let new_text = &current_text[last_text_len..];
            if !new_text.trim().is_empty() {
                type_text_incremental(new_text);
            }
            last_text_len = current_text.len();

            if let Ok(mut streaming_text) = STREAMING_TEXT.lock() {
                *streaming_text = current_text.to_string();
            }
        }
    }

    log::info!("[STREAMING] Streaming transcription ended");
}

/// Arrête l'enregistrement et colle le texte transcrit
fn stop_ptt_and_paste(app: &tauri::AppHandle) {
    log::info!("[PTT] stop_ptt_and_paste() called");

    let streaming_text = STREAMING_TEXT.lock().ok().map(|t| t.clone()).unwrap_or_default();
    let had_streaming = !streaming_text.is_empty();

    if let Ok(guard) = PTT_AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            let _ = sender.send(PttCommand::Stop);
        }
    }

    let (audio_data, sample_rate) = if let Ok(guard) = PTT_AUDIO_RECEIVER.lock() {
        if let Some(ref receiver) = *guard {
            loop {
                match receiver.recv_timeout(std::time::Duration::from_secs(2)) {
                    Ok(PttResult::AudioComplete { audio, sample_rate }) => break (audio, sample_rate),
                    Ok(PttResult::AudioSnapshot { .. }) => continue,
                    Err(e) => {
                        log::error!("Failed to receive audio data: {}", e);
                        return;
                    }
                }
            }
        } else {
            log::error!("PTT audio receiver not initialized");
            return;
        }
    } else {
        log::error!("Failed to lock PTT audio receiver");
        return;
    };

    if audio_data.is_empty() {
        log::warn!("Audio buffer is empty");
        return;
    }

    let duration = audio_data.len() as f32 / sample_rate as f32;
    log::info!("PTT captured {:.2}s of audio ({} samples at {}Hz)", duration, audio_data.len(), sample_rate);

    if duration < 0.3 {
        log::warn!("Recording too short");
        return;
    }

    // Reset and apply audio processing: pre-resample (noise suppression)
    let audio_data = {
        let state: tauri::State<'_, AppState> = app.state();
        let result = if let Ok(mut processor) = state.audio_processor.write() {
            processor.reset();
            processor.process_pre_resample(&audio_data, sample_rate)
        } else {
            audio_data
        };
        result
    };

    let resampled_audio = if sample_rate != TARGET_SAMPLE_RATE {
        crate::audio::resampling::resample_audio(&audio_data, sample_rate, TARGET_SAMPLE_RATE)
    } else {
        audio_data
    };

    // Audio processing: post-resample (limiter + AGC + VAD)
    let (resampled_audio, has_speech) = {
        let state: tauri::State<'_, AppState> = app.state();
        let result = if let Ok(mut processor) = state.audio_processor.write() {
            processor.process_post_resample(&resampled_audio)
        } else {
            (resampled_audio, true)
        };
        result
    };

    if !has_speech {
        log::info!("PTT: no speech detected, skipping");
        return;
    }

    let state: tauri::State<'_, AppState> = app.state();
    let engine_guard = match state.engine.read() {
        Ok(guard) => guard,
        Err(e) => {
            log::error!("Failed to lock engine: {}", e);
            return;
        }
    };
    let engine = match engine_guard.as_ref() {
        Some(e) => e,
        None => {
            log::error!("Whisper engine not initialized");
            return;
        }
    };
    let result = match engine.transcribe(&resampled_audio, TARGET_SAMPLE_RATE) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Transcription failed: {}", e);
            return;
        }
    };

    if result.text.is_empty() {
        log::warn!("Transcription returned empty text");
        return;
    }

    log::info!("Transcribed: '{}'", result.text);

    #[derive(serde::Serialize, Clone)]
    struct TranscriptionChunk {
        text: String,
        is_final: bool,
        duration_seconds: f32,
    }
    let chunk = TranscriptionChunk {
        text: result.text.clone(),
        is_final: true,
        duration_seconds: result.duration_seconds,
    };
    let _ = app.emit("transcription-chunk", chunk);

    let _ = storage::history::add_transcription(result.clone());

    let final_text = result.text.trim();
    if had_streaming && final_text.len() > streaming_text.len() {
        let remaining = &final_text[streaming_text.len()..];
        if !remaining.trim().is_empty() {
            type_text_incremental(remaining.trim());
        }
    } else if !had_streaming {
        paste_text(&result.text);
    }

    if let Ok(mut text) = STREAMING_TEXT.lock() {
        text.clear();
    }
}

/// Lit le texte du presse-papiers, le traduit et le colle
fn translate_clipboard_and_paste(app: &tauri::AppHandle) {
    log::info!("[TRANSLATE] translate_clipboard_and_paste() called");

    set_tray_state(TrayState::Translating);
    let _ = app.emit("translation-status", "translating");

    copy_selected_text();

    let clipboard_text = match app.clipboard().read_text() {
        Ok(text) => {
            if text.is_empty() {
                log::info!("[TRANSLATE] Clipboard is empty");
                set_tray_state(TrayState::Idle);
                let _ = app.emit("translation-status", "idle");
                return;
            }
            text
        }
        Err(e) => {
            log::error!("[TRANSLATE] Failed to read clipboard: {}", e);
            set_tray_state(TrayState::Idle);
            let _ = app.emit("translation-status", "idle");
            return;
        }
    };

    let settings = storage::config::load_settings();
    let target_language = settings.translation_target_language;

    let api_key = match crate::commands::llm::get_groq_api_key_internal() {
        Some(key) => key,
        None => {
            log::warn!("[TRANSLATE] No Groq API key configured");
            set_tray_state(TrayState::Idle);
            let _ = app.emit("translation_error", "Clé API Groq non configurée");
            let _ = app.emit("translation-status", "idle");
            return;
        }
    };

    let language_name = match target_language.as_str() {
        "fr" => "French", "en" => "English", "de" => "German",
        "es" => "Spanish", "it" => "Italian", "pt" => "Portuguese",
        "nl" => "Dutch", "ru" => "Russian", "zh" => "Chinese",
        "ja" => "Japanese", "ko" => "Korean", "ar" => "Arabic",
        _ => &target_language,
    };

    let system_prompt = format!(
        "You are a professional translator. Translate the following text to {}. \
         Only output the translation, nothing else. Preserve the original formatting, \
         punctuation and tone. If the text is already in {}, return it unchanged.",
        language_name, language_name
    );

    log::info!("[TRANSLATE] Calling Groq API for translation to {}...", language_name);

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("[TRANSLATE] Failed to create tokio runtime: {}", e);
            set_tray_state(TrayState::Idle);
            let _ = app.emit("translation-status", "idle");
            return;
        }
    };

    let translated = rt.block_on(async {
        crate::llm::groq_client::send_completion(&api_key, &system_prompt, &clipboard_text).await
    });

    match translated {
        Ok(text) => {
            let trimmed = text.trim().to_string();
            log::info!("[TRANSLATE] Translation successful");
            paste_text(&trimmed);
            let _ = app.emit("translation_complete", &trimmed);
        }
        Err(e) => {
            log::error!("[TRANSLATE] Translation failed: {}", e);
            let _ = app.emit("translation_error", format!("Erreur de traduction: {}", e));
        }
    }

    set_tray_state(TrayState::Idle);
    let _ = app.emit("translation-status", "idle");
}

/// Démarre le Voice Action: copie le texte sélectionné et démarre l'enregistrement
fn start_voice_action(app: &tauri::AppHandle) {
    log::info!("[VOICE_ACTION] Starting voice action...");

    copy_selected_text();

    let selected_text = match app.clipboard().read_text() {
        Ok(text) => {
            if text.is_empty() {
                log::info!("[VOICE_ACTION] No text selected");
                String::new()
            } else {
                log::info!("[VOICE_ACTION] Selected text: {} chars", text.len());
                text
            }
        }
        Err(e) => {
            log::warn!("[VOICE_ACTION] Failed to read clipboard: {}", e);
            String::new()
        }
    };

    if let Ok(mut guard) = SELECTED_TEXT_FOR_ACTION.lock() {
        *guard = selected_text;
    }

    set_tray_state(TrayState::VoiceAction);
    start_ptt_recording();
    let _ = app.emit("voice-action-status", "recording");
}

/// Arrête le Voice Action: transcrit l'instruction et exécute via Groq
fn stop_voice_action_and_execute(app: &tauri::AppHandle) {
    log::info!("[VOICE_ACTION] Stopping and executing...");

    let _ = app.emit("voice-action-status", "processing");

    let selected_text = SELECTED_TEXT_FOR_ACTION.lock()
        .ok()
        .map(|g| g.clone())
        .unwrap_or_default();

    if let Ok(guard) = PTT_AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            let _ = sender.send(PttCommand::Stop);
        }
    }

    let (audio_data, sample_rate) = if let Ok(guard) = PTT_AUDIO_RECEIVER.lock() {
        if let Some(ref receiver) = *guard {
            loop {
                match receiver.recv_timeout(std::time::Duration::from_secs(2)) {
                    Ok(PttResult::AudioComplete { audio, sample_rate }) => break (audio, sample_rate),
                    Ok(PttResult::AudioSnapshot { .. }) => continue,
                    Err(e) => {
                        log::error!("[VOICE_ACTION] Failed to receive audio: {}", e);
                        set_tray_state(TrayState::Idle);
                        let _ = app.emit("voice-action-status", "idle");
                        return;
                    }
                }
            }
        } else {
            set_tray_state(TrayState::Idle);
            let _ = app.emit("voice-action-status", "idle");
            return;
        }
    } else {
        set_tray_state(TrayState::Idle);
        let _ = app.emit("voice-action-status", "idle");
        return;
    };

    if audio_data.is_empty() {
        set_tray_state(TrayState::Idle);
        let _ = app.emit("voice-action-status", "idle");
        return;
    }

    let duration = audio_data.len() as f32 / sample_rate as f32;
    if duration < 0.5 {
        set_tray_state(TrayState::Idle);
        let _ = app.emit("voice-action-status", "idle");
        return;
    }

    // Audio processing: pre-resample (noise suppression)
    let audio_data = {
        let state: tauri::State<'_, AppState> = app.state();
        let result = if let Ok(mut processor) = state.audio_processor.write() {
            processor.reset();
            processor.process_pre_resample(&audio_data, sample_rate)
        } else {
            audio_data
        };
        result
    };

    let resampled = if sample_rate != TARGET_SAMPLE_RATE {
        crate::audio::resampling::resample_audio(&audio_data, sample_rate, TARGET_SAMPLE_RATE)
    } else {
        audio_data
    };

    // Audio processing: post-resample (limiter + AGC + VAD)
    let (resampled, has_speech) = {
        let state: tauri::State<'_, AppState> = app.state();
        let result = if let Ok(mut processor) = state.audio_processor.write() {
            processor.process_post_resample(&resampled)
        } else {
            (resampled, true)
        };
        result
    };

    if !has_speech {
        log::info!("[VOICE_ACTION] No speech detected, skipping");
        set_tray_state(TrayState::Idle);
        let _ = app.emit("voice-action-status", "idle");
        return;
    }

    let state: tauri::State<'_, crate::state::AppState> = app.state();
    let engine_guard = match state.engine.read() {
        Ok(guard) => guard,
        Err(_) => {
            set_tray_state(TrayState::Idle);
            let _ = app.emit("voice-action-status", "idle");
            return;
        }
    };

    let engine = match engine_guard.as_ref() {
        Some(e) => e,
        None => {
            set_tray_state(TrayState::Idle);
            let _ = app.emit("voice-action-status", "idle");
            return;
        }
    };

    let transcription = match engine.transcribe(&resampled, TARGET_SAMPLE_RATE) {
        Ok(r) => r.text,
        Err(e) => {
            log::error!("[VOICE_ACTION] Transcription failed: {}", e);
            set_tray_state(TrayState::Idle);
            let _ = app.emit("voice-action-status", "idle");
            return;
        }
    };

    drop(engine_guard);

    if transcription.is_empty() {
        set_tray_state(TrayState::Idle);
        let _ = app.emit("voice-action-status", "idle");
        return;
    }

    log::info!("[VOICE_ACTION] Instruction: '{}'", transcription);

    let api_key = match crate::commands::llm::get_groq_api_key_internal() {
        Some(key) => key,
        None => {
            let _ = app.emit("voice-action-error", "Clé API Groq non configurée");
            set_tray_state(TrayState::Idle);
            let _ = app.emit("voice-action-status", "idle");
            return;
        }
    };

    let system_prompt = r#"Tu es un assistant qui exécute des instructions sur du texte.
L'utilisateur te donne un texte et une instruction vocale.
Exécute l'instruction demandée sur le texte fourni.
Retourne UNIQUEMENT le résultat, sans explication ni commentaire."#;

    let user_prompt = if selected_text.is_empty() {
        transcription.clone()
    } else {
        format!("Texte:\n{}\n\nInstruction: {}", selected_text, transcription)
    };

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("[VOICE_ACTION] Failed to create runtime: {}", e);
            set_tray_state(TrayState::Idle);
            let _ = app.emit("voice-action-status", "idle");
            return;
        }
    };

    let result = rt.block_on(async {
        crate::llm::groq_client::send_completion(&api_key, system_prompt, &user_prompt).await
    });

    match result {
        Ok(response) => {
            let trimmed = response.trim().to_string();
            log::info!("[VOICE_ACTION] Success");
            paste_text(&trimmed);
            let _ = app.emit("voice-action-complete", &trimmed);
        }
        Err(e) => {
            log::error!("[VOICE_ACTION] Groq error: {}", e);
            let _ = app.emit("voice-action-error", format!("Erreur: {}", e));
        }
    }

    set_tray_state(TrayState::Idle);
    let _ = app.emit("voice-action-status", "idle");
}

/// Configure le global shortcut handler et enregistre les raccourcis
pub fn setup_shortcuts(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let settings = storage::config::load_settings();

    // Raccourci push-to-talk
    let ptt_hotkey = settings.hotkey_push_to_talk.clone();
    let ptt_shortcut = parse_hotkey(&ptt_hotkey)
        .unwrap_or_else(|| Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space));

    if let Ok(mut guard) = PTT_SHORTCUT.lock() {
        *guard = Some(ptt_shortcut);
    }
    match app.global_shortcut().register(ptt_shortcut) {
        Ok(_) => log::info!("[PTT] Shortcut '{}' registered successfully!", ptt_hotkey),
        Err(e) => log::error!("[PTT] ERROR registering shortcut: {:?}", e),
    }

    // Raccourci traduction (si activé)
    if settings.translation_enabled {
        let translate_hotkey = settings.hotkey_translate.clone();
        if let Some(translate_shortcut) = parse_hotkey(&translate_hotkey) {
            if let Ok(mut guard) = TRANSLATE_SHORTCUT.lock() {
                *guard = Some(translate_shortcut);
            }
            match app.global_shortcut().register(translate_shortcut) {
                Ok(_) => log::info!("[TRANSLATE] Shortcut '{}' registered!", translate_hotkey),
                Err(e) => log::error!("[TRANSLATE] ERROR registering shortcut: {:?}", e),
            }
        }
    }

    // Raccourci Voice Action
    let voice_action_hotkey = settings.hotkey_voice_action.clone();
    if let Some(voice_action_shortcut) = parse_hotkey(&voice_action_hotkey) {
        if let Ok(mut guard) = VOICE_ACTION_SHORTCUT.lock() {
            *guard = Some(voice_action_shortcut);
        }
        match app.global_shortcut().register(voice_action_shortcut) {
            Ok(_) => log::info!("[VOICE_ACTION] Shortcut '{}' registered!", voice_action_hotkey),
            Err(e) => log::error!("[VOICE_ACTION] ERROR registering shortcut: {:?}", e),
        }
    }

    Ok(())
}

/// Handler pour les événements de raccourcis globaux
pub fn handle_shortcut(app: &tauri::AppHandle, shortcut: &Shortcut, event: &tauri_plugin_global_shortcut::ShortcutEvent) {
    let is_ptt = PTT_SHORTCUT.lock().ok()
        .and_then(|guard| guard.as_ref().map(|s| *s == *shortcut))
        .unwrap_or(false);
    let is_translate = TRANSLATE_SHORTCUT.lock().ok()
        .and_then(|guard| guard.as_ref().map(|s| *s == *shortcut))
        .unwrap_or(false);
    let is_voice_action = VOICE_ACTION_SHORTCUT.lock().ok()
        .and_then(|guard| guard.as_ref().map(|s| *s == *shortcut))
        .unwrap_or(false);

    if is_ptt {
        match event.state() {
            ShortcutState::Pressed => {
                if !IS_PTT_ACTIVE.swap(true, Ordering::SeqCst) {
                    if let Ok(mut text) = STREAMING_TEXT.lock() {
                        text.clear();
                    }
                    set_tray_recording(true);
                    start_ptt_recording();
                    // Show floating window centered at top of screen during PTT
                    if let Some(window) = app.get_webview_window("floating") {
                        if let Ok(Some(monitor)) = window.primary_monitor() {
                            let screen_width = monitor.size().width as i32;
                            let window_width = 380; // matches tauri.conf.json
                            let x = (screen_width - window_width) / 2;
                            let y = 40; // slight offset from top
                            let _ = window.set_position(tauri::Position::Physical(
                                tauri::PhysicalPosition { x, y },
                            ));
                        }
                        let _ = window.show();
                    }
                    let _ = app.emit("recording-status", "recording");

                    let handle = app.clone();
                    std::thread::spawn(move || {
                        start_streaming_transcription(&handle);
                    });
                }
            }
            ShortcutState::Released => {
                if IS_PTT_ACTIVE.swap(false, Ordering::SeqCst) {
                    set_tray_recording(false);
                    let _ = app.emit("recording-status", "processing");
                    let handle = app.clone();
                    std::thread::spawn(move || {
                        stop_ptt_and_paste(&handle);
                        let _ = handle.emit("recording-status", "idle");
                        // Hide floating window after a delay
                        std::thread::sleep(std::time::Duration::from_secs(3));
                        if let Some(window) = handle.get_webview_window("floating") {
                            let _ = window.hide();
                        }
                    });
                }
            }
        }
    } else if is_translate {
        if let ShortcutState::Released = event.state() {
            let handle = app.clone();
            std::thread::spawn(move || {
                translate_clipboard_and_paste(&handle);
            });
        }
    } else if is_voice_action {
        match event.state() {
            ShortcutState::Pressed => {
                if !IS_VOICE_ACTION_ACTIVE.swap(true, Ordering::SeqCst) {
                    let handle = app.clone();
                    std::thread::spawn(move || {
                        start_voice_action(&handle);
                    });
                }
            }
            ShortcutState::Released => {
                if IS_VOICE_ACTION_ACTIVE.swap(false, Ordering::SeqCst) {
                    let handle = app.clone();
                    std::thread::spawn(move || {
                        stop_voice_action_and_execute(&handle);
                    });
                }
            }
        }
    }
}
