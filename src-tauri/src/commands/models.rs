use tauri::{AppHandle, Emitter, State};
use crate::state::AppState;
use crate::types::{EngineType, ModelSize, ParakeetModelSize, VoskLanguage};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub size: ModelSize,
    pub display_name: String,
    pub available: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
    pub percent: f32,
}

#[tauri::command]
pub fn get_available_models(state: State<'_, AppState>) -> Vec<ModelInfo> {
    [ModelSize::Tiny, ModelSize::Small, ModelSize::Medium]
        .into_iter()
        .map(|size| ModelInfo {
            size,
            display_name: size.display_name().to_string(),
            available: state.model_manager.is_model_available(size),
            size_bytes: size.size_bytes(),
        })
        .collect()
}

#[tauri::command]
pub fn get_current_model(state: State<'_, AppState>) -> Result<ModelSize, String> {
    let settings = state.settings.read().map_err(|e| e.to_string())?;
    Ok(settings.whisper_model)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    state: State<'_, AppState>,
    size: ModelSize,
) -> Result<(), String> {
    let model_manager = state.model_manager.clone();

    let downloaded = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(size.size_bytes()));
    let app_clone = app.clone();
    let downloaded_clone = downloaded.clone();
    let total_clone = total.clone();

    let progress_callback = move |dl: u64, t: u64| {
        downloaded_clone.store(dl, Ordering::SeqCst);
        total_clone.store(t, Ordering::SeqCst);

        let progress = DownloadProgress {
            downloaded: dl,
            total: t,
            percent: (dl as f32 / t as f32) * 100.0,
        };

        let _ = app_clone.emit("model-download-progress", progress);
    };

    model_manager
        .download_model(size, progress_callback)
        .await?;

    let _ = app.emit("model-download-complete", size);

    Ok(())
}

#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, size: ModelSize) -> Result<(), String> {
    state.model_manager.delete_model(size).await
}

#[tauri::command]
pub fn switch_model(state: State<'_, AppState>, size: ModelSize) -> Result<(), String> {
    if !state.model_manager.is_model_available(size) {
        return Err(format!("Model {:?} is not available. Please download it first.", size));
    }

    let settings = state.settings.read().map_err(|e| e.to_string())?;
    let language = if settings.auto_detect_language {
        None
    } else {
        Some(settings.transcription_language.clone())
    };
    drop(settings);

    state.reload_engine(size, language)?;

    let mut settings = state.settings.write().map_err(|e| e.to_string())?;
    settings.whisper_model = size;
    drop(settings);

    let settings = state.settings.read().map_err(|e| e.to_string())?;
    crate::storage::config::save_settings(&settings)?;

    Ok(())
}

#[tauri::command]
pub fn is_engine_ready(state: State<'_, AppState>) -> bool {
    state.engine.read().map(|e| e.is_some()).unwrap_or(false)
}

// ===== Vosk Model Commands =====

#[derive(Debug, Clone, Serialize)]
pub struct VoskModelInfo {
    pub language: VoskLanguage,
    pub display_name: String,
    pub available: bool,
}

#[tauri::command]
pub fn get_vosk_models(state: State<'_, AppState>) -> Vec<VoskModelInfo> {
    use VoskLanguage::*;

    [En, Fr, De, Es, It, Ru, Zh, Ja, Ko, Pt, Nl, Pl, Uk, Tr, Vi, Ar, Hi, Fa, Ca, Cs]
        .into_iter()
        .map(|lang| VoskModelInfo {
            language: lang,
            display_name: lang.display_name().to_string(),
            available: state.model_manager.get_vosk_model_path(lang).is_some(),
        })
        .collect()
}

#[tauri::command]
pub async fn download_vosk_model(
    app: AppHandle,
    state: State<'_, AppState>,
    language: VoskLanguage,
) -> Result<(), String> {
    let model_manager = state.model_manager.clone();
    let app_clone = app.clone();

    let downloaded = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(1));
    let downloaded_clone = downloaded.clone();
    let total_clone = total.clone();

    let progress_callback = move |dl: u64, t: u64| {
        downloaded_clone.store(dl, Ordering::SeqCst);
        total_clone.store(t, Ordering::SeqCst);

        let progress = DownloadProgress {
            downloaded: dl,
            total: t,
            percent: (dl as f32 / t as f32) * 100.0,
        };

        let _ = app_clone.emit("vosk-download-progress", progress);
    };

    model_manager
        .download_vosk_model(language, progress_callback)
        .await?;

    let _ = app.emit("vosk-download-complete", language);

    Ok(())
}

#[tauri::command]
pub fn select_vosk_language(state: State<'_, AppState>, language: VoskLanguage) -> Result<(), String> {
    if state.model_manager.get_vosk_model_path(language).is_none() {
        return Err(format!("Vosk model for {:?} is not available. Please download it first.", language));
    }

    state.reload_vosk_engine(language)?;

    let mut settings = state.settings.write().map_err(|e| e.to_string())?;
    settings.vosk_language = Some(language);
    settings.engine_type = EngineType::Vosk;
    drop(settings);

    let settings = state.settings.read().map_err(|e| e.to_string())?;
    crate::storage::config::save_settings(&settings)?;

    Ok(())
}

// ===== Engine Type Commands =====

#[tauri::command]
pub fn switch_engine_type(state: State<'_, AppState>, engine_type: EngineType) -> Result<(), String> {
    state.switch_engine_type(engine_type)?;

    let mut settings = state.settings.write().map_err(|e| e.to_string())?;
    settings.engine_type = engine_type;
    drop(settings);

    let settings = state.settings.read().map_err(|e| e.to_string())?;
    crate::storage::config::save_settings(&settings)?;

    Ok(())
}

#[tauri::command]
pub fn is_parakeet_available(_state: State<'_, AppState>) -> bool {
    // On macOS, Parakeet uses CoreML which works on both x86_64 and ARM
    #[cfg(target_os = "macos")]
    {
        true
    }
    // On other platforms, check if ONNX runtime is available
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

// ===== Parakeet Model Commands =====

#[derive(Debug, Clone, Serialize)]
pub struct ParakeetModelInfo {
    pub size: ParakeetModelSize,
    pub display_name: String,
    pub available: bool,
    pub size_bytes: u64,
}

#[tauri::command]
pub fn get_parakeet_models(state: State<'_, AppState>) -> Vec<ParakeetModelInfo> {
    vec![
        ParakeetModelInfo {
            size: ParakeetModelSize::Tdt06bV3,
            display_name: "Parakeet TDT 0.6B v3".to_string(),
            available: state.model_manager.get_parakeet_model_path(ParakeetModelSize::Tdt06bV3).is_some(),
            size_bytes: 1_200_000_000, // ~1.2 GB
        },
    ]
}

#[tauri::command]
pub async fn download_parakeet_model(
    app: AppHandle,
    state: State<'_, AppState>,
    size: ParakeetModelSize,
) -> Result<(), String> {
    log::info!("download_parakeet_model called with size: {:?}", size);
    let model_manager = state.model_manager.clone();
    let app_clone = app.clone();

    let downloaded = Arc::new(AtomicU64::new(0));
    let total = Arc::new(AtomicU64::new(1));
    let downloaded_clone = downloaded.clone();
    let total_clone = total.clone();

    let progress_callback = move |dl: u64, t: u64| {
        downloaded_clone.store(dl, Ordering::SeqCst);
        total_clone.store(t, Ordering::SeqCst);

        let progress = DownloadProgress {
            downloaded: dl,
            total: t,
            percent: (dl as f32 / t as f32) * 100.0,
        };

        let _ = app_clone.emit("parakeet-download-progress", progress);
    };

    model_manager
        .download_parakeet_model(size, progress_callback)
        .await?;

    let _ = app.emit("parakeet-download-complete", size);

    // Si Parakeet est le moteur sélectionné, charger l'engine automatiquement
    let settings = state.settings.read().map_err(|e| e.to_string())?;
    if settings.engine_type == EngineType::Parakeet {
        drop(settings);
        if let Err(e) = state.reload_parakeet_engine(size) {
            log::warn!("Failed to load Parakeet engine after download: {}", e);
        } else {
            log::info!("Parakeet engine loaded automatically after download");
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn delete_parakeet_model(state: State<'_, AppState>, size: ParakeetModelSize) -> Result<(), String> {
    state.model_manager.delete_parakeet_model(size).await
}

#[tauri::command]
pub fn select_parakeet_model(state: State<'_, AppState>, size: ParakeetModelSize) -> Result<(), String> {
    if state.model_manager.get_parakeet_model_path(size).is_none() {
        return Err(format!("Parakeet model {:?} is not available. Please download it first.", size));
    }

    state.reload_parakeet_engine(size)?;

    let mut settings = state.settings.write().map_err(|e| e.to_string())?;
    settings.parakeet_model = size;
    settings.engine_type = EngineType::Parakeet;
    drop(settings);

    let settings = state.settings.read().map_err(|e| e.to_string())?;
    crate::storage::config::save_settings(&settings)?;

    Ok(())
}
