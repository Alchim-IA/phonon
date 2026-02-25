use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tauri::{AppHandle, Manager};

use crate::engines::{ModelManager, ParakeetCoreMLEngine, ParakeetEngine, SpeechEngine, VoskEngine, WhisperEngine};
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
        if let Ok(exe) = std::env::current_exe() {
            log::info!("Current executable: {:?}", exe);
        }

        // Trouver le chemin des modèles bundled
        // Probe several possible locations for the whisper model
        let candidates = [
            // Production: Contents/Resources/resources/models/
            resource_path.join("resources").join("models"),
            // Alternative flat structure
            resource_path.join("models"),
            // Direct in resource dir (if Tauri flattens)
            resource_path.clone(),
        ];

        let mut bundled_models_path: Option<PathBuf> = None;
        for candidate in &candidates {
            let model_file = candidate.join("ggml-tiny.bin");
            log::info!("Probing model path: {:?} exists={}", model_file, model_file.exists());
            if model_file.exists() {
                log::info!("Found bundled models at: {:?}", candidate);
                bundled_models_path = Some(candidate.clone());
                break;
            }
        }

        // Dev mode fallback - walk back from the executable
        if bundled_models_path.is_none() {
            let dev_path = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                .map(|p| p.join("resources").join("models"));

            if let Some(ref path) = dev_path {
                let model_file = path.join("ggml-tiny.bin");
                log::info!("Probing dev model path: {:?} exists={}", model_file, model_file.exists());
                if model_file.exists() {
                    log::info!("Using dev bundled models path: {:?}", path);
                    bundled_models_path = Some(path.clone());
                }
            }
        }

        log::info!("Bundled models path: {:?}", bundled_models_path);

        // Obtenir le dossier de données utilisateur
        let app_data_dir = app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;

        // Créer le ModelManager
        let model_manager = ModelManager::new(
            app_data_dir,
            bundled_models_path,
        );

        // Helper to load Whisper engine
        let load_whisper = |model_manager: &ModelManager, settings: &AppSettings| -> Option<Box<dyn SpeechEngine>> {
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
                log::warn!("Whisper model {:?} not available", settings.whisper_model);
                None
            }
        };

        // Load engine based on configured type
        let engine: Option<Box<dyn SpeechEngine>> = match settings.engine_type {
            EngineType::Whisper => load_whisper(&model_manager, &settings),
            EngineType::Parakeet => {
                // On macOS, use CoreML-based Parakeet engine
                #[cfg(target_os = "macos")]
                {
                    // Find sidecar path
                    let sidecar_path = Self::find_parakeet_sidecar(&resource_path);
                    if let Some(path) = sidecar_path {
                        match ParakeetCoreMLEngine::new(path) {
                            Ok(engine) => {
                                log::info!("Parakeet CoreML engine initialized");
                                Some(Box::new(engine))
                            }
                            Err(e) => {
                                log::error!("Failed to initialize Parakeet CoreML engine: {}, falling back to Whisper", e);
                                load_whisper(&model_manager, &settings)
                            }
                        }
                    } else {
                        log::warn!("Parakeet CoreML sidecar not found, falling back to Whisper");
                        load_whisper(&model_manager, &settings)
                    }
                }
                // On other platforms, use ONNX-based Parakeet engine
                #[cfg(not(target_os = "macos"))]
                {
                    if let Some(model_path) = model_manager.get_parakeet_model_path(settings.parakeet_model) {
                        match ParakeetEngine::new(&model_path, settings.parakeet_model.into()) {
                            Ok(engine) => {
                                log::info!("Parakeet engine initialized with model {:?}", settings.parakeet_model);
                                Some(Box::new(engine))
                            }
                            Err(e) => {
                                log::error!("Failed to initialize Parakeet engine: {}, falling back to Whisper", e);
                                load_whisper(&model_manager, &settings)
                            }
                        }
                    } else {
                        log::warn!("Parakeet model {:?} not available, falling back to Whisper", settings.parakeet_model);
                        load_whisper(&model_manager, &settings)
                    }
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

    /// Find the Parakeet CoreML sidecar binary
    #[cfg(target_os = "macos")]
    fn find_parakeet_sidecar(resource_path: &PathBuf) -> Option<PathBuf> {
        // Determine target triple
        #[cfg(target_arch = "x86_64")]
        let target = "x86_64-apple-darwin";
        #[cfg(target_arch = "aarch64")]
        let target = "aarch64-apple-darwin";

        // Check in binaries directory (development)
        let dev_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .map(|p| p.join("binaries").join(format!("parakeet-coreml-{}", target)));

        if let Some(ref path) = dev_path {
            if path.exists() {
                log::info!("Found Parakeet sidecar at dev path: {:?}", path);
                return Some(path.clone());
            }
        }

        // Check in resource directory (production)
        let prod_path = resource_path.join("binaries").join(format!("parakeet-coreml-{}", target));
        if prod_path.exists() {
            log::info!("Found Parakeet sidecar at resource path: {:?}", prod_path);
            return Some(prod_path);
        }

        // Check alongside the executable
        let exe_path = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .map(|p| p.join(format!("parakeet-coreml-{}", target)));

        if let Some(ref path) = exe_path {
            if path.exists() {
                log::info!("Found Parakeet sidecar alongside exe: {:?}", path);
                return Some(path.clone());
            }
        }

        log::warn!("Parakeet sidecar not found for target {}", target);
        None
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
    pub fn reload_parakeet_engine(&self, _model_size: ParakeetModelSize) -> Result<(), String> {
        // On macOS, use CoreML-based Parakeet engine
        #[cfg(target_os = "macos")]
        {
            let sidecar_path = Self::find_parakeet_sidecar(&self.resource_path)
                .ok_or_else(|| "Parakeet CoreML sidecar not found".to_string())?;

            let new_engine = ParakeetCoreMLEngine::new(sidecar_path)?;

            let mut engine = self.engine.write().map_err(|e| e.to_string())?;
            *engine = Some(Box::new(new_engine));

            log::info!("Parakeet CoreML engine reloaded");
            Ok(())
        }
        // On other platforms, use ONNX-based Parakeet engine
        #[cfg(not(target_os = "macos"))]
        {
            let model_path = self.model_manager
                .get_parakeet_model_path(_model_size)
                .ok_or_else(|| format!("Parakeet model {:?} not available", _model_size))?;

            let new_engine = ParakeetEngine::new(&model_path, _model_size.into())?;

            let mut engine = self.engine.write().map_err(|e| e.to_string())?;
            *engine = Some(Box::new(new_engine));

            log::info!("Parakeet engine reloaded with model {:?}", _model_size);
            Ok(())
        }
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
