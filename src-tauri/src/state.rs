use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Manager};

use crate::engines::{ModelManager, ParakeetEngine, SpeechEngine, VoskEngine, WhisperEngine};
use crate::storage::config;
use crate::types::{AppSettings, EngineType, ModelSize, ParakeetModelSize, VoskLanguage};

pub struct AppState {
    pub is_recording: Arc<RwLock<bool>>,
    pub settings: Arc<RwLock<AppSettings>>,
    pub sample_rate: Arc<RwLock<u32>>,
    pub engine: Arc<RwLock<Option<Box<dyn SpeechEngine>>>>,
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

        // Load engine based on configured type
        let engine: Option<Box<dyn SpeechEngine>> = match settings.engine_type {
            EngineType::Whisper => {
                if let Some(model_path) = model_manager.get_model_path(settings.whisper_model) {
                    let lang = if settings.auto_detect_language {
                        None
                    } else {
                        Some(settings.transcription_language.clone())
                    };

                    match WhisperEngine::new(&model_path, lang, settings.whisper_model) {
                        Ok(engine) => {
                            log::info!("Whisper engine initialized with model {:?}", settings.whisper_model);
                            Some(Box::new(engine))
                        }
                        Err(e) => {
                            log::error!("Failed to initialize Whisper engine: {}", e);
                            None
                        }
                    }
                } else {
                    log::warn!("Whisper model {:?} not available, engine not initialized", settings.whisper_model);
                    None
                }
            }
            EngineType::Parakeet => {
                if let Some(model_path) = model_manager.get_parakeet_model_path(settings.parakeet_model) {
                    match ParakeetEngine::new(&model_path, settings.parakeet_model.into()) {
                        Ok(engine) => {
                            log::info!("Parakeet engine initialized with model {:?}", settings.parakeet_model);
                            Some(Box::new(engine))
                        }
                        Err(e) => {
                            log::error!("Failed to initialize Parakeet engine: {}", e);
                            None
                        }
                    }
                } else {
                    log::warn!("Parakeet model {:?} not available, engine not initialized", settings.parakeet_model);
                    None
                }
            }
            EngineType::Vosk => {
                // Find Vosk model matching configured language
                let vosk_lang = settings.vosk_language
                    .or_else(|| VoskLanguage::from_language_code(&settings.transcription_language));

                if let Some(lang) = vosk_lang {
                    if let Some(model_path) = model_manager.get_vosk_model_path(lang) {
                        match VoskEngine::new(&model_path, lang) {
                            Ok(engine) => {
                                log::info!("Vosk engine initialized for language {:?}", lang);
                                Some(Box::new(engine))
                            }
                            Err(e) => {
                                log::error!("Failed to initialize Vosk engine: {}", e);
                                None
                            }
                        }
                    } else {
                        log::warn!("Vosk model for {:?} not available", lang);
                        None
                    }
                } else {
                    log::warn!("No Vosk language configured");
                    None
                }
            }
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

    /// Recharge le moteur Whisper avec un nouveau modèle
    pub fn reload_engine(&self, model_size: ModelSize, language: Option<String>) -> Result<(), String> {
        let model_path = self.model_manager
            .get_model_path(model_size)
            .ok_or_else(|| format!("Model {:?} not available", model_size))?;

        let new_engine = WhisperEngine::new(&model_path, language, model_size)?;

        let mut engine = self.engine.write().map_err(|e| e.to_string())?;
        *engine = Some(Box::new(new_engine));

        log::info!("Whisper engine reloaded with model {:?}", model_size);
        Ok(())
    }

    /// Recharge le moteur Parakeet avec un nouveau modèle
    pub fn reload_parakeet_engine(&self, model_size: ParakeetModelSize) -> Result<(), String> {
        let model_path = self.model_manager
            .get_parakeet_model_path(model_size)
            .ok_or_else(|| format!("Parakeet model {:?} not available", model_size))?;

        let new_engine = ParakeetEngine::new(&model_path, model_size.into())?;

        let mut engine = self.engine.write().map_err(|e| e.to_string())?;
        *engine = Some(Box::new(new_engine));

        log::info!("Parakeet engine reloaded with model {:?}", model_size);
        Ok(())
    }

    /// Recharge le moteur Vosk avec une nouvelle langue
    pub fn reload_vosk_engine(&self, language: VoskLanguage) -> Result<(), String> {
        let model_path = self.model_manager
            .get_vosk_model_path(language)
            .ok_or_else(|| format!("Vosk model for {:?} not available", language))?;

        let new_engine = VoskEngine::new(&model_path, language)?;

        let mut engine = self.engine.write().map_err(|e| e.to_string())?;
        *engine = Some(Box::new(new_engine));

        log::info!("Vosk engine reloaded for language {:?}", language);
        Ok(())
    }

    /// Change le type de moteur (Whisper, Parakeet ou Vosk)
    pub fn switch_engine_type(&self, engine_type: EngineType) -> Result<(), String> {
        let settings = self.settings.read().map_err(|e| e.to_string())?;

        match engine_type {
            EngineType::Whisper => {
                let language = if settings.auto_detect_language {
                    None
                } else {
                    Some(settings.transcription_language.clone())
                };
                let model_size = settings.whisper_model;
                drop(settings);
                self.reload_engine(model_size, language)
            }
            EngineType::Parakeet => {
                let model_size = settings.parakeet_model;
                drop(settings);
                self.reload_parakeet_engine(model_size)
            }
            EngineType::Vosk => {
                let vosk_lang = settings.vosk_language
                    .or_else(|| VoskLanguage::from_language_code(&settings.transcription_language));
                drop(settings);

                if let Some(lang) = vosk_lang {
                    self.reload_vosk_engine(lang)
                } else {
                    Err("No Vosk language configured and current language not supported by Vosk".to_string())
                }
            }
        }
    }
}
