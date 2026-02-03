mod audio;
mod commands;
mod engines;
mod llm;
mod state;
mod storage;
mod types;
mod voice_commands;

pub use audio::AudioCapture;
pub use types::*;

use engines::SpeechEngine;
use state::AppState;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_clipboard_manager::ClipboardExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::process::{Command, Stdio};

// Raccourcis globaux
static PTT_SHORTCUT: Mutex<Option<Shortcut>> = Mutex::new(None);
static TRANSLATE_SHORTCUT: Mutex<Option<Shortcut>> = Mutex::new(None);

// État global pour le push-to-talk
static IS_PTT_ACTIVE: AtomicBool = AtomicBool::new(false);

// Channel pour envoyer les données audio du thread d'enregistrement
static PTT_AUDIO_SENDER: Mutex<Option<mpsc::Sender<PttCommand>>> = Mutex::new(None);
static PTT_AUDIO_RECEIVER: Mutex<Option<mpsc::Receiver<PttResult>>> = Mutex::new(None);

// Référence globale au TrayIcon pour changer l'icône
static TRAY_ICON: Mutex<Option<TrayIcon>> = Mutex::new(None);

// Icônes en cache
static ICON_DEFAULT: Mutex<Option<Image<'static>>> = Mutex::new(None);
static ICON_RECORDING: Mutex<Option<Image<'static>>> = Mutex::new(None);

#[derive(Debug)]
enum PttCommand {
    Start,
    Stop,
}

#[derive(Debug)]
struct PttResult {
    audio: Vec<f32>,
    sample_rate: u32,
}

/// Parse un raccourci clavier depuis un format string (ex: "Ctrl+Shift+R")
fn parse_hotkey(hotkey: &str) -> Option<Shortcut> {
    let parts: Vec<&str> = hotkey.split('+').collect();
    if parts.is_empty() {
        return None;
    }

    let mut modifiers = Modifiers::empty();
    let mut key_code: Option<Code> = None;

    for part in parts {
        let part_lower = part.trim().to_lowercase();
        match part_lower.as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "cmd" | "command" | "meta" => modifiers |= Modifiers::META,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            _ => {
                // C'est la touche principale
                key_code = match part.to_uppercase().as_str() {
                    "A" => Some(Code::KeyA),
                    "B" => Some(Code::KeyB),
                    "C" => Some(Code::KeyC),
                    "D" => Some(Code::KeyD),
                    "E" => Some(Code::KeyE),
                    "F" => Some(Code::KeyF),
                    "G" => Some(Code::KeyG),
                    "H" => Some(Code::KeyH),
                    "I" => Some(Code::KeyI),
                    "J" => Some(Code::KeyJ),
                    "K" => Some(Code::KeyK),
                    "L" => Some(Code::KeyL),
                    "M" => Some(Code::KeyM),
                    "N" => Some(Code::KeyN),
                    "O" => Some(Code::KeyO),
                    "P" => Some(Code::KeyP),
                    "Q" => Some(Code::KeyQ),
                    "R" => Some(Code::KeyR),
                    "S" => Some(Code::KeyS),
                    "T" => Some(Code::KeyT),
                    "U" => Some(Code::KeyU),
                    "V" => Some(Code::KeyV),
                    "W" => Some(Code::KeyW),
                    "X" => Some(Code::KeyX),
                    "Y" => Some(Code::KeyY),
                    "Z" => Some(Code::KeyZ),
                    "0" => Some(Code::Digit0),
                    "1" => Some(Code::Digit1),
                    "2" => Some(Code::Digit2),
                    "3" => Some(Code::Digit3),
                    "4" => Some(Code::Digit4),
                    "5" => Some(Code::Digit5),
                    "6" => Some(Code::Digit6),
                    "7" => Some(Code::Digit7),
                    "8" => Some(Code::Digit8),
                    "9" => Some(Code::Digit9),
                    "SPACE" => Some(Code::Space),
                    "ENTER" | "RETURN" => Some(Code::Enter),
                    "TAB" => Some(Code::Tab),
                    "ESCAPE" | "ESC" => Some(Code::Escape),
                    "BACKSPACE" => Some(Code::Backspace),
                    "DELETE" => Some(Code::Delete),
                    "F1" => Some(Code::F1),
                    "F2" => Some(Code::F2),
                    "F3" => Some(Code::F3),
                    "F4" => Some(Code::F4),
                    "F5" => Some(Code::F5),
                    "F6" => Some(Code::F6),
                    "F7" => Some(Code::F7),
                    "F8" => Some(Code::F8),
                    "F9" => Some(Code::F9),
                    "F10" => Some(Code::F10),
                    "F11" => Some(Code::F11),
                    "F12" => Some(Code::F12),
                    _ => None,
                };
            }
        }
    }

    key_code.map(|code| {
        if modifiers.is_empty() {
            Shortcut::new(None, code)
        } else {
            Shortcut::new(Some(modifiers), code)
        }
    })
}

/// Génère une icône d'enregistrement (cercle rouge)
fn create_recording_icon() -> Image<'static> {
    // Créer une icône 32x32 avec un cercle rouge
    let size = 32;
    let mut rgba = vec![0u8; size * size * 4];

    let center = size as f32 / 2.0;
    let radius = center - 2.0;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - center;
            let dy = y as f32 - center;
            let dist = (dx * dx + dy * dy).sqrt();

            let idx = (y * size + x) * 4;
            if dist <= radius {
                // Rouge vif
                rgba[idx] = 255;     // R
                rgba[idx + 1] = 51;  // G
                rgba[idx + 2] = 102; // B
                rgba[idx + 3] = 255; // A
            } else if dist <= radius + 1.0 {
                // Anti-aliasing
                let alpha = ((radius + 1.0 - dist) * 255.0) as u8;
                rgba[idx] = 255;
                rgba[idx + 1] = 51;
                rgba[idx + 2] = 102;
                rgba[idx + 3] = alpha;
            }
        }
    }

    Image::new_owned(rgba, size as u32, size as u32)
}

/// Change l'icône du tray
fn set_tray_recording(recording: bool) {
    println!("[TRAY] set_tray_recording({})", recording);

    match TRAY_ICON.lock() {
        Ok(guard) => {
            if let Some(ref tray) = *guard {
                println!("[TRAY] Got tray reference");
                let icon = if recording {
                    match ICON_RECORDING.lock() {
                        Ok(guard) => {
                            println!("[TRAY] Got recording icon: {:?}", guard.is_some());
                            guard.clone()
                        }
                        Err(e) => {
                            println!("[TRAY] Failed to lock ICON_RECORDING: {:?}", e);
                            None
                        }
                    }
                } else {
                    match ICON_DEFAULT.lock() {
                        Ok(guard) => {
                            println!("[TRAY] Got default icon: {:?}", guard.is_some());
                            guard.clone()
                        }
                        Err(e) => {
                            println!("[TRAY] Failed to lock ICON_DEFAULT: {:?}", e);
                            None
                        }
                    }
                };

                if let Some(icon) = icon {
                    match tray.set_icon(Some(icon)) {
                        Ok(_) => println!("[TRAY] Icon changed successfully"),
                        Err(e) => println!("[TRAY] Failed to set icon: {:?}", e),
                    }
                } else {
                    println!("[TRAY] No icon to set");
                }
            } else {
                println!("[TRAY] TRAY_ICON is None");
            }
        }
        Err(e) => {
            println!("[TRAY] Failed to lock TRAY_ICON: {:?}", e);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    // Charger les settings pour afficher le raccourci utilisé
    let settings = storage::config::load_settings();
    println!("[PTT] Using hotkey: {}", settings.hotkey_push_to_talk);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    // Déterminer quel raccourci a été déclenché
                    let is_ptt = PTT_SHORTCUT.lock().ok()
                        .and_then(|guard| guard.as_ref().map(|s| *s == *shortcut))
                        .unwrap_or(false);
                    let is_translate = TRANSLATE_SHORTCUT.lock().ok()
                        .and_then(|guard| guard.as_ref().map(|s| *s == *shortcut))
                        .unwrap_or(false);

                    if is_ptt {
                        match event.state() {
                            ShortcutState::Pressed => {
                                println!("[PTT] Key PRESSED - Starting recording");
                                if !IS_PTT_ACTIVE.swap(true, Ordering::SeqCst) {
                                    set_tray_recording(true);
                                    start_ptt_recording();
                                    // Émettre l'événement de statut pour le frontend
                                    let _ = app.emit("recording-status", "recording");
                                }
                            }
                            ShortcutState::Released => {
                                println!("[PTT] Key RELEASED - Stopping recording");
                                if IS_PTT_ACTIVE.swap(false, Ordering::SeqCst) {
                                    set_tray_recording(false);
                                    // Émettre l'événement de traitement
                                    let _ = app.emit("recording-status", "processing");
                                    let handle = app.clone();
                                    std::thread::spawn(move || {
                                        stop_ptt_and_paste(&handle);
                                        // Émettre l'événement de fin
                                        let _ = handle.emit("recording-status", "idle");
                                    });
                                }
                            }
                        }
                    } else if is_translate {
                        // La traduction se déclenche sur le release
                        if let ShortcutState::Released = event.state() {
                            println!("[TRANSLATE] Shortcut released - Starting translation");
                            let handle = app.clone();
                            std::thread::spawn(move || {
                                translate_clipboard_and_paste(&handle);
                            });
                        }
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            commands::list_audio_devices,
            commands::get_settings,
            commands::update_settings,
            commands::get_dictionary,
            commands::add_dictionary_word,
            commands::remove_dictionary_word,
            commands::start_recording,
            commands::stop_recording,
            commands::get_history,
            commands::clear_history,
            commands::get_recording_status,
            commands::reset_recording_state,
            commands::get_available_models,
            commands::get_current_model,
            commands::download_model,
            commands::delete_model,
            commands::switch_model,
            commands::is_engine_ready,
            commands::set_groq_api_key,
            commands::get_groq_api_key,
            commands::has_groq_api_key,
            commands::validate_groq_api_key,
            commands::delete_groq_api_key,
            commands::translate_text,
            commands::auto_paste,
            commands::show_floating_window,
            commands::hide_floating_window,
            commands::toggle_floating_window,
            commands::set_floating_window_size,
            commands::get_floating_window_position,
            commands::set_floating_window_position,
        ])
        .setup(|app| {
            // Initialiser l'état avec le moteur Whisper
            let app_state = match AppState::new(app.handle()) {
                Ok(state) => state,
                Err(e) => {
                    log::error!("Failed to initialize app state: {}", e);
                    return Err(e.into());
                }
            };

            app.manage(app_state);

            // Initialiser le thread audio pour le push-to-talk
            init_ptt_audio_thread();

            // Initialiser le thread audio pour la transcription GUI
            commands::transcription::init_gui_audio_thread();

            // Charger les settings pour les raccourcis
            let settings = storage::config::load_settings();

            // Raccourci push-to-talk
            let ptt_hotkey = settings.hotkey_push_to_talk.clone();
            let ptt_shortcut = parse_hotkey(&ptt_hotkey)
                .unwrap_or_else(|| Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Space));

            // Stocker et enregistrer le raccourci PTT
            if let Ok(mut guard) = PTT_SHORTCUT.lock() {
                *guard = Some(ptt_shortcut);
            }
            match app.global_shortcut().register(ptt_shortcut) {
                Ok(_) => println!("[PTT] Shortcut '{}' registered successfully!", ptt_hotkey),
                Err(e) => println!("[PTT] ERROR registering shortcut: {:?}", e),
            }

            // Raccourci traduction (si activé)
            if settings.translation_enabled {
                let translate_hotkey = settings.hotkey_translate.clone();
                if let Some(translate_shortcut) = parse_hotkey(&translate_hotkey) {
                    // Stocker le raccourci
                    if let Ok(mut guard) = TRANSLATE_SHORTCUT.lock() {
                        *guard = Some(translate_shortcut);
                    }
                    match app.global_shortcut().register(translate_shortcut) {
                        Ok(_) => println!("[TRANSLATE] Shortcut '{}' registered successfully!", translate_hotkey),
                        Err(e) => println!("[TRANSLATE] ERROR registering shortcut: {:?}", e),
                    }
                }
            }

            // Stocker l'icône par défaut (créer une copie owned)
            println!("[TRAY] Initializing tray icons...");
            let default_icon = app.default_window_icon().unwrap();
            println!("[TRAY] Default icon size: {}x{}", default_icon.width(), default_icon.height());

            let default_icon_owned = Image::new_owned(
                default_icon.rgba().to_vec(),
                default_icon.width(),
                default_icon.height(),
            );
            if let Ok(mut guard) = ICON_DEFAULT.lock() {
                *guard = Some(default_icon_owned);
                println!("[TRAY] ICON_DEFAULT stored");
            }

            // Créer et stocker l'icône d'enregistrement
            let recording_icon = create_recording_icon();
            if let Ok(mut guard) = ICON_RECORDING.lock() {
                *guard = Some(recording_icon);
                println!("[TRAY] ICON_RECORDING stored");
            }

            // Cloner l'icône pour le tray
            let tray_icon = Image::new_owned(
                default_icon.rgba().to_vec(),
                default_icon.width(),
                default_icon.height(),
            );

            // Créer le menu tray
            let tray_menu = create_tray_menu(app)?;

            // Create tray icon
            let tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| {
                    handle_tray_menu_event(app, event.id.as_ref());
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Stocker le TrayIcon pour pouvoir changer l'icône
            if let Ok(mut guard) = TRAY_ICON.lock() {
                *guard = Some(tray);
                println!("[TRAY] TRAY_ICON stored successfully");
            } else {
                println!("[TRAY] ERROR: Failed to store TRAY_ICON");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Intercepter la fermeture de la fenêtre principale pour la cacher au lieu de la fermer
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    // Empêcher la fermeture et cacher la fenêtre à la place
                    api.prevent_close();
                    let _ = window.hide();
                    println!("[WINDOW] Main window hidden instead of closed");
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Crée le menu de la tray icon similaire à Wispr Flow
fn create_tray_menu(app: &tauri::App) -> Result<Menu<tauri::Wry>, tauri::Error> {
    // Items principaux
    let home = MenuItem::with_id(app, "home", "Accueil", true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", "Rechercher des mises à jour...", true, None::<&str>)?;

    // Paste last transcript
    let paste_last = MenuItem::with_id(app, "paste_last", "Coller dernière transcription", true, Some("Option+Cmd+V"))?;
    let last_transcript = MenuItem::with_id(app, "last_transcript_preview", "Aucune transcription", false, None::<&str>)?;

    // Raccourcis
    let shortcuts = MenuItem::with_id(app, "shortcuts", "Raccourcis clavier", true, None::<&str>)?;

    // Sous-menu Microphone
    let mic_default = MenuItem::with_id(app, "mic_default", "Microphone par défaut", true, None::<&str>)?;
    let mic_submenu = Submenu::with_items(app, "Microphone", true, &[&mic_default])?;

    // Sous-menu Langues
    let lang_fr = MenuItem::with_id(app, "lang_fr", "🇫🇷 Français", true, None::<&str>)?;
    let lang_en = MenuItem::with_id(app, "lang_en", "🇬🇧 English", true, None::<&str>)?;
    let lang_de = MenuItem::with_id(app, "lang_de", "🇩🇪 Deutsch", true, None::<&str>)?;
    let lang_es = MenuItem::with_id(app, "lang_es", "🇪🇸 Español", true, None::<&str>)?;
    let lang_it = MenuItem::with_id(app, "lang_it", "🇮🇹 Italiano", true, None::<&str>)?;
    let lang_auto = MenuItem::with_id(app, "lang_auto", "🌐 Détection auto", true, None::<&str>)?;
    let lang_submenu = Submenu::with_items(app, "Langue", true, &[
        &lang_fr, &lang_en, &lang_de, &lang_es, &lang_it,
        &PredefinedMenuItem::separator(app)?,
        &lang_auto
    ])?;

    // Aide
    let help = MenuItem::with_id(app, "help", "Centre d'aide", true, None::<&str>)?;
    let feedback = MenuItem::with_id(app, "feedback", "Envoyer un commentaire", true, None::<&str>)?;

    // Quitter
    let quit = MenuItem::with_id(app, "quit", "Quitter WakaScribe", true, Some("Cmd+Q"))?;

    // Construire le menu
    Menu::with_items(app, &[
        &home,
        &updates,
        &PredefinedMenuItem::separator(app)?,
        &paste_last,
        &last_transcript,
        &PredefinedMenuItem::separator(app)?,
        &shortcuts,
        &mic_submenu,
        &lang_submenu,
        &PredefinedMenuItem::separator(app)?,
        &help,
        &feedback,
        &PredefinedMenuItem::separator(app)?,
        &quit,
    ])
}

/// Gère les événements du menu tray
fn handle_tray_menu_event(app: &tauri::AppHandle, menu_id: &str) {
    match menu_id {
        "home" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "updates" => {
            log::info!("Check for updates clicked");
            // TODO: Implémenter la vérification des mises à jour
        }
        "paste_last" => {
            log::info!("Paste last transcript clicked");
            paste_last_transcript(app);
        }
        "shortcuts" => {
            log::info!("Shortcuts clicked");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.emit("navigate", "/settings/shortcuts");
            }
        }
        "help" => {
            log::info!("Help clicked");
            let _ = open::that("https://github.com/anthropics/claude-code");
        }
        "feedback" => {
            log::info!("Feedback clicked");
        }
        "quit" => {
            app.exit(0);
        }
        id if id.starts_with("mic_") => {
            log::info!("Microphone selected: {}", id);
            // TODO: Changer le microphone
        }
        id if id.starts_with("lang_") => {
            let lang = id.strip_prefix("lang_").unwrap_or("fr");
            log::info!("Language selected: {}", lang);
            update_language(app, lang);
        }
        _ => {}
    }
}

/// Colle la dernière transcription
fn paste_last_transcript(_app: &tauri::AppHandle) {
    let history = storage::history::load_history();
    if let Some(last) = history.transcriptions.last() {
        let text = &last.text;
        paste_text(text);
    }
}

/// Met à jour la langue de transcription
fn update_language(app: &tauri::AppHandle, lang: &str) {
    let state: tauri::State<'_, AppState> = app.state();
    if let Ok(mut settings) = state.settings.write() {
        settings.transcription_language = lang.to_string();
        settings.auto_detect_language = lang == "auto";
        let _ = storage::config::save_settings(&settings);
        log::info!("Language updated to: {}", lang);
    };
}

/// Initialise le thread audio pour le push-to-talk
fn init_ptt_audio_thread() {
    let (cmd_tx, cmd_rx) = mpsc::channel::<PttCommand>();
    let (result_tx, result_rx) = mpsc::channel::<PttResult>();

    // Stocker les channels
    if let Ok(mut guard) = PTT_AUDIO_SENDER.lock() {
        *guard = Some(cmd_tx);
    }
    if let Ok(mut guard) = PTT_AUDIO_RECEIVER.lock() {
        *guard = Some(result_rx);
    }

    // Thread audio dédié qui possède l'AudioCapture
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
                Ok(PttCommand::Stop) => {
                    log::info!("PTT: Stopping audio capture");
                    if let Some(mut cap) = capture.take() {
                        match cap.stop() {
                            Ok((audio, sample_rate)) => {
                                let _ = result_tx.send(PttResult { audio, sample_rate });
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
    println!("[PTT] start_ptt_recording() called");
    if let Ok(guard) = PTT_AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            let _ = sender.send(PttCommand::Start);
            println!("[PTT] Recording command sent");
        } else {
            println!("[PTT] ERROR: audio sender not initialized");
        }
    }
}

/// Taux d'échantillonnage requis par le modèle Parakeet
const TARGET_SAMPLE_RATE: u32 = 16000;

/// Arrête l'enregistrement et colle le texte transcrit
fn stop_ptt_and_paste(app: &tauri::AppHandle) {
    println!("[PTT] stop_ptt_and_paste() called");

    // Envoyer la commande d'arrêt
    if let Ok(guard) = PTT_AUDIO_SENDER.lock() {
        if let Some(ref sender) = *guard {
            let _ = sender.send(PttCommand::Stop);
            println!("[PTT] Stop command sent");
        }
    }

    // Attendre les données audio
    let ptt_result = if let Ok(guard) = PTT_AUDIO_RECEIVER.lock() {
        if let Some(ref receiver) = *guard {
            match receiver.recv_timeout(std::time::Duration::from_secs(2)) {
                Ok(result) => result,
                Err(e) => {
                    log::error!("Failed to receive audio data: {}", e);
                    return;
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

    let audio_data = ptt_result.audio;
    let sample_rate = ptt_result.sample_rate;

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

    // Resampler si nécessaire
    let resampled_audio = if sample_rate != TARGET_SAMPLE_RATE {
        log::info!("Resampling from {}Hz to {}Hz", sample_rate, TARGET_SAMPLE_RATE);
        resample_audio(&audio_data, sample_rate, TARGET_SAMPLE_RATE)
    } else {
        audio_data
    };

    // Transcrire
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

    // Émettre le texte transcrit vers le frontend
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

    // Sauvegarder dans l'historique
    let _ = storage::history::add_transcription(result.clone());

    // Coller le texte
    paste_text(&result.text);
}

/// Resampling linéaire simple de l'audio
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

/// Colle le texte à la position du curseur
fn paste_text(text: &str) {
    println!("[PASTE] paste_text called with: '{}'", text);

    #[cfg(target_os = "macos")]
    {
        use std::io::Write;

        // Copier dans le clipboard via pbcopy
        println!("[PASTE] Copying to clipboard via pbcopy...");
        match Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    match stdin.write_all(text.as_bytes()) {
                        Ok(_) => println!("[PASTE] Written to pbcopy stdin"),
                        Err(e) => {
                            println!("[PASTE] Failed to write to pbcopy stdin: {}", e);
                            return;
                        }
                    }
                }
                match child.wait() {
                    Ok(status) => println!("[PASTE] pbcopy exited with: {}", status),
                    Err(e) => println!("[PASTE] Failed to wait for pbcopy: {}", e),
                }
            }
            Err(e) => {
                println!("[PASTE] Failed to spawn pbcopy: {}", e);
                return;
            }
        }

        // Délai pour s'assurer que le clipboard est mis à jour
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Simuler Cmd+V avec AppleScript
        println!("[PASTE] Simulating Cmd+V via AppleScript...");
        let script = r#"tell application "System Events" to keystroke "v" using command down"#;

        match Command::new("osascript")
            .args(["-e", script])
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    println!("[PASTE] ✓ Text pasted successfully!");
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("[PASTE] AppleScript failed: {}", stderr);

                    // Essayer avec le mode texte direct si Cmd+V échoue
                    println!("[PASTE] Trying direct text insertion...");
                    let escaped_text = text.replace("\\", "\\\\").replace("\"", "\\\"");
                    let type_script = format!(
                        r#"tell application "System Events" to keystroke "{}""#,
                        escaped_text
                    );

                    match Command::new("osascript")
                        .args(["-e", &type_script])
                        .output()
                    {
                        Ok(output2) => {
                            if output2.status.success() {
                                println!("[PASTE] ✓ Text typed directly!");
                            } else {
                                println!("[PASTE] Direct typing also failed");
                                println!("[PASTE] ⚠️  Le texte est copié dans le presse-papier. Utilisez Cmd+V manuellement.");
                                println!("[PASTE] ⚠️  Pour activer le paste automatique:");
                                println!("[PASTE]    Préférences Système > Sécurité et confidentialité > Confidentialité > Accessibilité");
                                println!("[PASTE]    Ajoutez WakaScribe à la liste");
                            }
                        }
                        Err(e) => {
                            println!("[PASTE] Failed to execute type script: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                println!("[PASTE] Failed to execute osascript: {}", e);
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
        };

        // Copier dans le clipboard via clip.exe
        println!("[PASTE] Copying to clipboard via clip.exe...");
        use std::io::Write;
        match Command::new("cmd")
            .args(["/C", "clip"])
            .stdin(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    // Windows clipboard expects UTF-16 or the current code page
                    // Using UTF-8 with clip.exe should work for most cases
                    match stdin.write_all(text.as_bytes()) {
                        Ok(_) => println!("[PASTE] Written to clip.exe stdin"),
                        Err(e) => {
                            println!("[PASTE] Failed to write to clip.exe stdin: {}", e);
                            return;
                        }
                    }
                }
                match child.wait() {
                    Ok(status) => println!("[PASTE] clip.exe exited with: {}", status),
                    Err(e) => println!("[PASTE] Failed to wait for clip.exe: {}", e),
                }
            }
            Err(e) => {
                println!("[PASTE] Failed to spawn clip.exe: {}", e);
                return;
            }
        }

        // Délai pour s'assurer que le clipboard est mis à jour
        std::thread::sleep(std::time::Duration::from_millis(50));

        // Simuler Ctrl+V avec SendInput
        println!("[PASTE] Simulating Ctrl+V via SendInput...");

        let inputs: [INPUT; 4] = [
            // Ctrl down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // V down
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V,
                        wScan: 0,
                        dwFlags: KEYBD_EVENT_FLAGS(0),
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // V up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_V,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
            // Ctrl up
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_CONTROL,
                        wScan: 0,
                        dwFlags: KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            },
        ];

        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };

        if sent == 4 {
            println!("[PASTE] ✓ Text pasted successfully!");
        } else {
            println!("[PASTE] SendInput failed: only {} of 4 inputs sent", sent);
            println!("[PASTE] ⚠️  Le texte est copié dans le presse-papier. Utilisez Ctrl+V manuellement.");
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::io::Write;

        // Detect display server
        let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
        println!("[PASTE] Linux detected, display server: {}", if wayland { "Wayland" } else { "X11" });

        if wayland {
            // Wayland - use wl-copy for clipboard
            println!("[PASTE] Copying to clipboard via wl-copy...");
            match Command::new("wl-copy")
                .stdin(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        match stdin.write_all(text.as_bytes()) {
                            Ok(_) => println!("[PASTE] Written to wl-copy stdin"),
                            Err(e) => {
                                println!("[PASTE] Failed to write to wl-copy stdin: {}", e);
                                return;
                            }
                        }
                    }
                    let _ = child.wait();
                }
                Err(e) => {
                    println!("[PASTE] Failed to spawn wl-copy: {}. Install with 'sudo apt install wl-clipboard'", e);
                    return;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));

            // Try wtype first
            println!("[PASTE] Simulating Ctrl+V via wtype...");
            match Command::new("wtype")
                .args(["-M", "ctrl", "v", "-m", "ctrl"])
                .output()
            {
                Ok(output) if output.status.success() => {
                    println!("[PASTE] ✓ Text pasted successfully via wtype!");
                }
                _ => {
                    // Fallback to ydotool
                    println!("[PASTE] wtype failed, trying ydotool...");
                    match Command::new("ydotool")
                        .args(["key", "29:1", "47:1", "47:0", "29:0"])
                        .output()
                    {
                        Ok(output) if output.status.success() => {
                            println!("[PASTE] ✓ Text pasted successfully via ydotool!");
                        }
                        _ => {
                            println!("[PASTE] ⚠️  Le texte est copié dans le presse-papier. Utilisez Ctrl+V manuellement.");
                            println!("[PASTE] ⚠️  Pour activer le paste automatique sur Wayland:");
                            println!("[PASTE]    sudo apt install wtype  # ou ydotool");
                        }
                    }
                }
            }
        } else {
            // X11 - use xclip for clipboard
            println!("[PASTE] Copying to clipboard via xclip...");
            match Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(Stdio::piped())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        match stdin.write_all(text.as_bytes()) {
                            Ok(_) => println!("[PASTE] Written to xclip stdin"),
                            Err(e) => {
                                println!("[PASTE] Failed to write to xclip stdin: {}", e);
                                return;
                            }
                        }
                    }
                    let _ = child.wait();
                }
                Err(e) => {
                    println!("[PASTE] Failed to spawn xclip: {}. Install with 'sudo apt install xclip'", e);
                    return;
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(50));

            // Use xdotool for paste
            println!("[PASTE] Simulating Ctrl+V via xdotool...");
            match Command::new("xdotool")
                .args(["key", "--clearmodifiers", "ctrl+v"])
                .output()
            {
                Ok(output) if output.status.success() => {
                    println!("[PASTE] ✓ Text pasted successfully via xdotool!");
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("[PASTE] xdotool failed: {}", stderr);
                    println!("[PASTE] ⚠️  Le texte est copié dans le presse-papier. Utilisez Ctrl+V manuellement.");
                }
                Err(e) => {
                    println!("[PASTE] Failed to execute xdotool: {}. Install with 'sudo apt install xdotool'", e);
                    println!("[PASTE] ⚠️  Le texte est copié dans le presse-papier. Utilisez Ctrl+V manuellement.");
                }
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        println!("[PASTE] Paste not implemented for this platform - text copied to clipboard");
    }
}

/// Lit le texte du presse-papiers, le traduit et le colle
fn translate_clipboard_and_paste(app: &tauri::AppHandle) {
    println!("[TRANSLATE] translate_clipboard_and_paste() called");

    // 1. Lire le texte du presse-papiers
    let clipboard_text = match app.clipboard().read_text() {
        Ok(text) => {
            if text.is_empty() {
                println!("[TRANSLATE] Clipboard is empty");
                return;
            }
            text
        }
        Err(e) => {
            println!("[TRANSLATE] Failed to read clipboard: {}", e);
            return;
        }
    };

    println!("[TRANSLATE] Clipboard text: '{}' ({} chars)",
             &clipboard_text[..clipboard_text.len().min(50)], clipboard_text.len());

    // 2. Récupérer la langue cible depuis les settings
    let settings = storage::config::load_settings();
    let target_language = settings.translation_target_language;

    // 3. Récupérer la clé API
    let api_key = match commands::llm::get_groq_api_key_internal() {
        Some(key) => key,
        None => {
            println!("[TRANSLATE] No Groq API key configured");
            // Notifier l'utilisateur via une notification
            let _ = app.emit("translation_error", "Clé API Groq non configurée");
            return;
        }
    };

    // 4. Créer le prompt de traduction
    let language_name = match target_language.as_str() {
        "fr" => "French",
        "en" => "English",
        "de" => "German",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        "nl" => "Dutch",
        "ru" => "Russian",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "ar" => "Arabic",
        _ => &target_language,
    };

    let system_prompt = format!(
        "You are a professional translator. Translate the following text to {}. \
         Only output the translation, nothing else. Preserve the original formatting, \
         punctuation and tone. If the text is already in {}, return it unchanged.",
        language_name, language_name
    );

    // 5. Appeler l'API Groq de manière synchrone
    println!("[TRANSLATE] Calling Groq API for translation to {}...", language_name);

    // Utiliser tokio runtime pour l'appel async
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            println!("[TRANSLATE] Failed to create tokio runtime: {}", e);
            return;
        }
    };

    let translated = rt.block_on(async {
        llm::groq_client::send_completion(&api_key, &system_prompt, &clipboard_text).await
    });

    match translated {
        Ok(text) => {
            let trimmed = text.trim().to_string();
            println!("[TRANSLATE] Translation successful: '{}'", &trimmed[..trimmed.len().min(50)]);

            // 6. Coller le texte traduit
            paste_text(&trimmed);

            // Émettre un événement de succès
            let _ = app.emit("translation_complete", &trimmed);
        }
        Err(e) => {
            println!("[TRANSLATE] Translation failed: {}", e);
            let _ = app.emit("translation_error", format!("Erreur de traduction: {}", e));
        }
    }
}
