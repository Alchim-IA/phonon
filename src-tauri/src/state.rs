use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Manager};

use crate::engines::{ModelManager, WhisperEngine};
use crate::storage::config;
use crate::types::{AppSettings, ModelSize};

pub struct AppState {
    pub is_recording: Arc<RwLock<bool>>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub sample_rate: Arc<RwLock<u32>>,
    pub engine: Arc<RwLock<Option<WhisperEngine>>>,
    pub model_manager: Arc<ModelManager>,
    pub resource_path: PathBuf,
    pub audio_buffer: Arc<RwLock<Option<(Vec<f32>, u32)>>>,
}

impl AppState {
    pub fn new(app_handle: &AppHandle) -> Result<Self, String> {
        let settings = config::load_settings();

        // Obtenir le chemin des ressources
        let resource_path = app_handle
            .path()
            .resource_dir()
            .map_err(|e| format!("Failed to get resource dir: {}", e))?;

        log::info!("Resource path from Tauri: {:?}", resource_path);

        // En mode développement, resource_dir() pointe vers target/debug/
        let resource_path = if resource_path.join("models").exists() {
            resource_path
        } else {
            let dev_path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|p| p.join("resources"));

            if let Some(ref path) = dev_path {
                if path.join("models").exists() {
                    log::info!("Using dev resource path: {:?}", path);
                    path.clone()
                } else {
                    resource_path
                }
            } else {
                resource_path
            }
        };

        log::info!("Final resource path: {:?}", resource_path);

        // Obtenir le dossier de données utilisateur
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        // Créer le ModelManager
        let model_manager = ModelManager::new(
            app_data_dir,
            Some(resource_path.join("models")),
        );

        // Charger le moteur Whisper si le modèle est disponible
        let engine = if let Some(model_path) = model_manager.get_model_path(settings.whisper_model) {
            let lang = if settings.auto_detect_language {
                None
            } else {
                Some(settings.transcription_language.clone())
            };

            match WhisperEngine::new(&model_path, lang, settings.whisper_model) {
                Ok(engine) => {
                    log::info!("Whisper engine initialized with model {:?}", settings.whisper_model);
                    Some(engine)
                }
                Err(e) => {
                    log::error!("Failed to initialize Whisper engine: {}", e);
                    None
                }
            }
        } else {
            log::warn!("Model {:?} not available, engine not initialized", settings.whisper_model);
            None
        };

        Ok(Self {
            is_recording: Arc::new(RwLock::new(false)),
            settings: Arc::new(RwLock::new(settings)),
            sample_rate: Arc::new(RwLock::new(16000)),
            engine: Arc::new(RwLock::new(engine)),
            model_manager: Arc::new(model_manager),
            resource_path,
            audio_buffer: Arc::new(RwLock::new(None)),
        })
    }

    /// Recharge le moteur avec un nouveau modèle
    pub fn reload_engine(&self, model_size: ModelSize, language: Option<String>) -> Result<(), String> {
        let model_path = self.model_manager
            .get_model_path(model_size)
            .ok_or_else(|| format!("Model {:?} not available", model_size))?;

        let new_engine = WhisperEngine::new(&model_path, language, model_size)?;

        let mut engine = self.engine.write().map_err(|e| e.to_string())?;
        *engine = Some(new_engine);

        log::info!("Engine reloaded with model {:?}", model_size);
        Ok(())
    }
}
