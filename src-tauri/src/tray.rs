use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

use crate::platform::paste_text;
use crate::state::AppState;
use crate::storage;

// Référence globale au TrayIcon pour changer l'icône
static TRAY_ICON: Mutex<Option<tauri::tray::TrayIcon>> = Mutex::new(None);

// Icônes en cache
static ICON_DEFAULT: Mutex<Option<Image<'static>>> = Mutex::new(None);
static ICON_RECORDING: Mutex<Option<Image<'static>>> = Mutex::new(None);
static ICON_TRANSLATING: Mutex<Option<Image<'static>>> = Mutex::new(None);
static ICON_VOICE_ACTION: Mutex<Option<Image<'static>>> = Mutex::new(None);

/// État du tray icon
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrayState {
    Idle,
    Recording,
    Translating,
    VoiceAction,
}

/// Change l'icône du tray selon l'état
pub fn set_tray_state(state: TrayState) {
    log::debug!("[TRAY] set_tray_state({:?})", state);

    match TRAY_ICON.lock() {
        Ok(guard) => {
            if let Some(ref tray) = *guard {
                let icon = match state {
                    TrayState::Idle => ICON_DEFAULT.lock().ok().and_then(|g| g.clone()),
                    TrayState::Recording => ICON_RECORDING.lock().ok().and_then(|g| g.clone()),
                    TrayState::Translating => ICON_TRANSLATING.lock().ok().and_then(|g| g.clone()),
                    TrayState::VoiceAction => ICON_VOICE_ACTION.lock().ok().and_then(|g| g.clone()),
                };

                if let Some(icon) = icon {
                    if let Err(e) = tray.set_icon(Some(icon)) {
                        log::warn!("[TRAY] Failed to set icon: {:?}", e);
                    }
                }
            }
        }
        Err(e) => {
            log::warn!("[TRAY] Failed to lock TRAY_ICON: {:?}", e);
        }
    }
}

/// Raccourci pour compatibilité
pub fn set_tray_recording(recording: bool) {
    set_tray_state(if recording { TrayState::Recording } else { TrayState::Idle });
}

/// Génère une icône circulaire avec une couleur spécifique
fn create_colored_icon(r: u8, g: u8, b: u8) -> Image<'static> {
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
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            } else if dist <= radius + 1.0 {
                let alpha = ((radius + 1.0 - dist) * 255.0) as u8;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = alpha;
            }
        }
    }

    Image::new_owned(rgba, size as u32, size as u32)
}

/// Crée le menu de la tray icon
fn create_tray_menu(app: &tauri::App) -> Result<Menu<tauri::Wry>, tauri::Error> {
    let home = MenuItem::with_id(app, "home", "Accueil", true, None::<&str>)?;
    let updates = MenuItem::with_id(app, "updates", "Rechercher des mises à jour...", true, None::<&str>)?;
    let paste_last = MenuItem::with_id(app, "paste_last", "Coller dernière transcription", true, Some("Option+Cmd+V"))?;
    let last_transcript = MenuItem::with_id(app, "last_transcript_preview", "Aucune transcription", false, None::<&str>)?;
    let shortcuts = MenuItem::with_id(app, "shortcuts", "Raccourcis clavier", true, None::<&str>)?;
    let subtitles = MenuItem::with_id(app, "subtitles", "Sous-titres en direct", true, None::<&str>)?;

    let mic_default = MenuItem::with_id(app, "mic_default", "Microphone par défaut", true, None::<&str>)?;
    let mic_submenu = Submenu::with_items(app, "Microphone", true, &[&mic_default])?;

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

    let help = MenuItem::with_id(app, "help", "Centre d'aide", true, None::<&str>)?;
    let feedback = MenuItem::with_id(app, "feedback", "Envoyer un commentaire", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter Phonon", true, Some("Cmd+Q"))?;

    Menu::with_items(app, &[
        &home, &updates,
        &PredefinedMenuItem::separator(app)?,
        &paste_last, &last_transcript,
        &PredefinedMenuItem::separator(app)?,
        &shortcuts, &subtitles, &mic_submenu, &lang_submenu,
        &PredefinedMenuItem::separator(app)?,
        &help, &feedback,
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
        }
        "paste_last" => {
            log::info!("Paste last transcript clicked");
            let history = storage::history::load_history();
            if let Some(last) = history.transcriptions.last() {
                paste_text(&last.text);
            }
        }
        "shortcuts" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.emit("navigate", "/settings/shortcuts");
            }
        }
        "subtitles" => {
            log::info!("Subtitles toggled from tray");
            if let Some(window) = app.get_webview_window("subtitles") {
                let is_visible = window.is_visible().unwrap_or(false);
                if is_visible {
                    let _ = window.hide();
                } else {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        }
        "help" => {
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
        }
        id if id.starts_with("lang_") => {
            let lang = id.strip_prefix("lang_").unwrap_or("fr");
            log::info!("Language selected: {}", lang);
            update_language(app, lang);
        }
        _ => {}
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

/// Construit le tray icon complet lors du setup de l'application
pub fn build_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("[TRAY] Initializing tray icons...");

    // Charger l'icône tray
    let tray_icon_path = app.path().resource_dir()
        .ok()
        .and_then(|p| {
            let path_2x = p.join("icons/tray-iconTemplate@2x.png");
            if path_2x.exists() { return Some(path_2x); }
            let path = p.join("icons/tray-iconTemplate.png");
            if path.exists() { Some(path) } else { None }
        })
        .or_else(|| {
            let path_2x = std::path::PathBuf::from("icons/tray-iconTemplate@2x.png");
            if path_2x.exists() { return Some(path_2x); }
            let path = std::path::PathBuf::from("icons/tray-iconTemplate.png");
            if path.exists() { Some(path) } else { None }
        })
        .or_else(|| {
            let path = std::path::PathBuf::from("icons/icon.png");
            if path.exists() { Some(path) } else { None }
        });

    let (icon_rgba, icon_width, icon_height) = if let Some(path) = tray_icon_path {
        log::info!("[TRAY] Loading tray icon from: {:?}", path);
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                (rgba.into_raw(), w, h)
            }
            Err(e) => {
                log::warn!("[TRAY] Failed to load tray icon from file: {}", e);
                let default_icon = app.default_window_icon().unwrap();
                (default_icon.rgba().to_vec(), default_icon.width(), default_icon.height())
            }
        }
    } else {
        log::info!("[TRAY] Using default window icon for tray");
        let default_icon = app.default_window_icon().unwrap();
        (default_icon.rgba().to_vec(), default_icon.width(), default_icon.height())
    };

    // Stocker les icônes
    let default_icon_owned = Image::new_owned(icon_rgba.clone(), icon_width, icon_height);
    if let Ok(mut guard) = ICON_DEFAULT.lock() {
        *guard = Some(default_icon_owned);
    }

    if let Ok(mut guard) = ICON_RECORDING.lock() {
        *guard = Some(create_colored_icon(255, 59, 48)); // Rouge
    }
    if let Ok(mut guard) = ICON_TRANSLATING.lock() {
        *guard = Some(create_colored_icon(0, 122, 255)); // Bleu
    }
    if let Ok(mut guard) = ICON_VOICE_ACTION.lock() {
        *guard = Some(create_colored_icon(255, 179, 0)); // Jaune
    }

    let tray_icon = Image::new_owned(icon_rgba, icon_width, icon_height);
    let tray_menu = create_tray_menu(app)?;

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

    if let Ok(mut guard) = TRAY_ICON.lock() {
        *guard = Some(tray);
        log::info!("[TRAY] Tray icon built successfully");
    }

    Ok(())
}
