# Migration vers Whisper.cpp - Plan d'implémentation

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remplacer le moteur OpenVINO/Parakeet par Whisper.cpp via whisper-rs, avec gestion des modèles (tiny bundlé, small/medium téléchargeables) et détection automatique de langue.

**Architecture:** Nouveau `WhisperEngine` implémentant le trait `SpeechEngine` existant. `ModelManager` pour télécharger et gérer les modèles. Modèle tiny bundlé dans l'app, autres modèles téléchargeables depuis Hugging Face.

**Tech Stack:** whisper-rs 0.14, reqwest (déjà dispo via Tauri), tokio async

---

## Task 1: Ajouter whisper-rs dans Cargo.toml

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Modifier Cargo.toml**

Remplacer la dépendance `openvino` par `whisper-rs`:

```toml
# Supprimer cette ligne:
openvino = { version = "0.8", features = ["runtime-linking"] }

# Ajouter ces lignes:
whisper-rs = "0.14"
reqwest = { version = "0.11", features = ["stream"] }
futures-util = "0.3"
```

**Step 2: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK (avec warnings sur code non utilisé)

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "build: replace openvino with whisper-rs dependency"
```

---

## Task 2: Créer le type ModelSize et adapter EngineError

**Files:**
- Modify: `src-tauri/src/engines/error.rs`
- Modify: `src-tauri/src/types.rs`

**Step 1: Ajouter ModelSize dans types.rs**

Ajouter après `HistoryData`:

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModelSize {
    Tiny,
    Small,
    Medium,
}

impl ModelSize {
    pub fn file_name(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "ggml-tiny.bin",
            ModelSize::Small => "ggml-small.bin",
            ModelSize::Medium => "ggml-medium.bin",
        }
    }

    pub fn download_url(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
            ModelSize::Small => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
            ModelSize::Medium => "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        }
    }

    pub fn size_bytes(&self) -> u64 {
        match self {
            ModelSize::Tiny => 75_000_000,
            ModelSize::Small => 466_000_000,
            ModelSize::Medium => 1_500_000_000,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ModelSize::Tiny => "Tiny (75 MB)",
            ModelSize::Small => "Small (466 MB)",
            ModelSize::Medium => "Medium (1.5 GB)",
        }
    }
}

impl Default for ModelSize {
    fn default() -> Self {
        ModelSize::Tiny
    }
}
```

**Step 2: Ajouter whisper_model dans AppSettings**

Modifier la struct `AppSettings`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub microphone_id: Option<String>,
    pub hotkey_push_to_talk: String,
    pub hotkey_toggle_record: String,
    pub transcription_language: String,
    pub auto_detect_language: bool,
    pub theme: String,
    pub minimize_to_tray: bool,
    pub auto_copy_to_clipboard: bool,
    pub notification_on_complete: bool,
    pub whisper_model: ModelSize,  // NOUVEAU
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            microphone_id: None,
            hotkey_push_to_talk: "CommandOrControl+Shift+Space".to_string(),
            hotkey_toggle_record: "CommandOrControl+Shift+R".to_string(),
            transcription_language: "fr".to_string(),
            auto_detect_language: false,
            theme: "system".to_string(),
            minimize_to_tray: true,
            auto_copy_to_clipboard: true,
            notification_on_complete: true,
            whisper_model: ModelSize::Tiny,  // NOUVEAU
        }
    }
}
```

**Step 3: Adapter EngineError pour Whisper**

Remplacer le contenu de `error.rs`:

```rust
use std::fmt;

#[derive(Debug)]
pub enum EngineError {
    WhisperInitFailed(String),
    ModelLoadFailed(String),
    ModelNotFound(String),
    InferenceError(String),
    AudioTooShort,
    InvalidSampleRate(u32),
    DownloadError(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EngineError::WhisperInitFailed(msg) => write!(f, "Whisper initialization failed: {}", msg),
            EngineError::ModelLoadFailed(msg) => write!(f, "Model loading failed: {}", msg),
            EngineError::ModelNotFound(path) => write!(f, "Model not found: {}", path),
            EngineError::InferenceError(msg) => write!(f, "Inference error: {}", msg),
            EngineError::AudioTooShort => write!(f, "Audio too short (minimum 0.5 seconds)"),
            EngineError::InvalidSampleRate(rate) => write!(f, "Invalid sample rate: {}Hz (expected 16000Hz)", rate),
            EngineError::DownloadError(msg) => write!(f, "Download error: {}", msg),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<EngineError> for String {
    fn from(err: EngineError) -> String {
        err.to_string()
    }
}
```

**Step 4: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK

**Step 5: Commit**

```bash
git add src-tauri/src/types.rs src-tauri/src/engines/error.rs
git commit -m "feat: add ModelSize enum and adapt EngineError for Whisper"
```

---

## Task 3: Créer le ModelManager

**Files:**
- Create: `src-tauri/src/engines/model_manager.rs`
- Modify: `src-tauri/src/engines/mod.rs`

**Step 1: Créer model_manager.rs**

```rust
use crate::types::ModelSize;
use futures_util::StreamExt;
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;

pub struct ModelManager {
    models_dir: PathBuf,
    bundled_model_path: Option<PathBuf>,
}

impl ModelManager {
    pub fn new(app_data_dir: PathBuf, bundled_model_path: Option<PathBuf>) -> Self {
        let models_dir = app_data_dir.join("models");
        Self {
            models_dir,
            bundled_model_path,
        }
    }

    /// Retourne le chemin du modèle s'il existe
    pub fn get_model_path(&self, size: ModelSize) -> Option<PathBuf> {
        // Pour tiny, vérifier d'abord le bundled
        if size == ModelSize::Tiny {
            if let Some(ref bundled) = self.bundled_model_path {
                let bundled_model = bundled.join(size.file_name());
                if bundled_model.exists() {
                    return Some(bundled_model);
                }
            }
        }

        // Sinon, vérifier dans le dossier utilisateur
        let user_model = self.models_dir.join(size.file_name());
        if user_model.exists() {
            Some(user_model)
        } else {
            None
        }
    }

    /// Vérifie si un modèle est disponible
    pub fn is_model_available(&self, size: ModelSize) -> bool {
        self.get_model_path(size).is_some()
    }

    /// Liste les modèles disponibles
    pub fn available_models(&self) -> Vec<ModelSize> {
        [ModelSize::Tiny, ModelSize::Small, ModelSize::Medium]
            .into_iter()
            .filter(|&size| self.is_model_available(size))
            .collect()
    }

    /// Télécharge un modèle depuis Hugging Face
    pub async fn download_model<F>(
        &self,
        size: ModelSize,
        progress_callback: F,
    ) -> Result<PathBuf, String>
    where
        F: Fn(u64, u64) + Send + 'static,
    {
        // Créer le dossier models si nécessaire
        fs::create_dir_all(&self.models_dir)
            .await
            .map_err(|e| format!("Failed to create models directory: {}", e))?;

        let dest_path = self.models_dir.join(size.file_name());
        let url = size.download_url();

        log::info!("Downloading model {} from {}", size.file_name(), url);

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to start download: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Download failed with status: {}", response.status()));
        }

        let total_size = response.content_length().unwrap_or(size.size_bytes());
        let mut downloaded: u64 = 0;

        let mut file = fs::File::create(&dest_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download error: {}", e))?;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Write error: {}", e))?;
            downloaded += chunk.len() as u64;
            progress_callback(downloaded, total_size);
        }

        file.flush()
            .await
            .map_err(|e| format!("Flush error: {}", e))?;

        log::info!("Model {} downloaded successfully", size.file_name());
        Ok(dest_path)
    }

    /// Supprime un modèle téléchargé
    pub async fn delete_model(&self, size: ModelSize) -> Result<(), String> {
        let path = self.models_dir.join(size.file_name());
        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| format!("Failed to delete model: {}", e))?;
        }
        Ok(())
    }
}
```

**Step 2: Mettre à jour mod.rs**

Remplacer le contenu de `engines/mod.rs`:

```rust
pub mod error;
pub mod model_manager;
pub mod traits;

pub use error::EngineError;
pub use model_manager::ModelManager;
pub use traits::SpeechEngine;
```

**Step 3: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK (warnings sur imports non utilisés)

**Step 4: Commit**

```bash
git add src-tauri/src/engines/model_manager.rs src-tauri/src/engines/mod.rs
git commit -m "feat: add ModelManager for Whisper model downloads"
```

---

## Task 4: Créer le WhisperEngine

**Files:**
- Create: `src-tauri/src/engines/whisper.rs`
- Modify: `src-tauri/src/engines/mod.rs`

**Step 1: Créer whisper.rs**

```rust
use crate::engines::traits::SpeechEngine;
use crate::types::{ModelSize, TranscriptionResult};
use chrono::Utc;
use std::path::Path;
use std::sync::Mutex;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub struct WhisperEngine {
    ctx: Mutex<WhisperContext>,
    language: Option<String>,
    model_size: ModelSize,
}

impl WhisperEngine {
    pub fn new(model_path: &Path, language: Option<String>, model_size: ModelSize) -> Result<Self, String> {
        log::info!("Loading Whisper model from {:?}", model_path);

        if !model_path.exists() {
            return Err(format!("Model file not found: {:?}", model_path));
        }

        let ctx = WhisperContext::new_with_params(
            model_path.to_str().ok_or("Invalid model path")?,
            WhisperContextParameters::default(),
        )
        .map_err(|e| format!("Failed to load Whisper model: {}", e))?;

        log::info!("Whisper model loaded successfully");

        Ok(Self {
            ctx: Mutex::new(ctx),
            language,
            model_size,
        })
    }

    pub fn model_size(&self) -> ModelSize {
        self.model_size
    }

    pub fn set_language(&mut self, language: Option<String>) {
        self.language = language;
    }
}

impl SpeechEngine for WhisperEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        let start_time = std::time::Instant::now();

        if sample_rate != 16000 {
            return Err(format!(
                "Invalid sample rate: {}Hz (expected 16000Hz)",
                sample_rate
            ));
        }

        let duration_seconds = audio.len() as f32 / sample_rate as f32;
        if duration_seconds < 0.5 {
            return Err("Audio too short (minimum 0.5 seconds)".to_string());
        }

        let ctx = self.ctx.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Configurer la langue
        if let Some(ref lang) = self.language {
            if lang != "auto" {
                params.set_language(Some(lang));
            }
        }

        // Optimisations
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(true);
        params.set_no_context(true);

        // Créer un état pour cette transcription
        let mut state = ctx
            .create_state()
            .map_err(|e| format!("Failed to create state: {}", e))?;

        // Exécuter la transcription
        state
            .full(params, audio)
            .map_err(|e| format!("Transcription failed: {}", e))?;

        // Récupérer le résultat
        let num_segments = state.full_n_segments().map_err(|e| format!("Error: {}", e))?;
        let mut text = String::new();

        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                text.push_str(&segment);
            }
        }

        let detected_language = state
            .full_lang_id()
            .ok()
            .and_then(|id| whisper_rs::get_lang_str(id).map(|s| s.to_string()));

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        log::info!(
            "Transcription completed in {}ms: {} chars",
            processing_time_ms,
            text.len()
        );

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            confidence: 0.95,
            duration_seconds,
            processing_time_ms,
            detected_language,
            timestamp: Utc::now().timestamp(),
        })
    }

    fn name(&self) -> &str {
        "Whisper"
    }
}

unsafe impl Send for WhisperEngine {}
unsafe impl Sync for WhisperEngine {}
```

**Step 2: Mettre à jour mod.rs**

```rust
pub mod error;
pub mod model_manager;
pub mod traits;
pub mod whisper;

pub use error::EngineError;
pub use model_manager::ModelManager;
pub use traits::SpeechEngine;
pub use whisper::WhisperEngine;
```

**Step 3: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK

**Step 4: Commit**

```bash
git add src-tauri/src/engines/whisper.rs src-tauri/src/engines/mod.rs
git commit -m "feat: add WhisperEngine implementing SpeechEngine trait"
```

---

## Task 5: Adapter AppState pour Whisper

**Files:**
- Modify: `src-tauri/src/state.rs`

**Step 1: Remplacer le contenu de state.rs**

```rust
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
```

**Step 2: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK

**Step 3: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "feat: adapt AppState for WhisperEngine and ModelManager"
```

---

## Task 6: Adapter les commandes de transcription

**Files:**
- Modify: `src-tauri/src/commands/transcription.rs`

**Step 1: Mettre à jour transcription.rs**

Modifier `stop_recording` pour utiliser le nouveau moteur:

```rust
use tauri::State;
use crate::engines::SpeechEngine;
use crate::state::AppState;
use crate::storage::history;
use crate::types::TranscriptionResult;
use crate::audio::AudioCapture;
use std::cell::RefCell;

/// Taux d'échantillonnage requis par Whisper
const TARGET_SAMPLE_RATE: u32 = 16000;

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

    let mut capture = AudioCapture::new(device_id.as_deref())?;
    capture.start(device_id.as_deref())?;

    {
        let mut sr = state.sample_rate.write().map_err(|e| e.to_string())?;
        *sr = capture.sample_rate();
    }

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

    let (resampled_audio, final_sample_rate) = if sample_rate != TARGET_SAMPLE_RATE {
        log::info!("Resampling audio from {}Hz to {}Hz", sample_rate, TARGET_SAMPLE_RATE);
        let resampled = resample_audio(&audio_buffer, sample_rate, TARGET_SAMPLE_RATE);
        (resampled, TARGET_SAMPLE_RATE)
    } else {
        (audio_buffer, sample_rate)
    };

    // Utiliser le moteur Whisper
    let engine_guard = state.engine.read().map_err(|e| e.to_string())?;
    let engine = engine_guard
        .as_ref()
        .ok_or("Whisper engine not initialized. Please download a model first.")?;

    let result = engine.transcribe(&resampled_audio, final_sample_rate)?;

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
```

**Step 2: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK

**Step 3: Commit**

```bash
git add src-tauri/src/commands/transcription.rs
git commit -m "feat: adapt transcription commands for WhisperEngine"
```

---

## Task 7: Ajouter les commandes de gestion des modèles

**Files:**
- Create: `src-tauri/src/commands/models.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Créer commands/models.rs**

```rust
use tauri::{AppHandle, State};
use crate::state::AppState;
use crate::types::ModelSize;
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

    // Progress tracking
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

    // Émettre l'événement de fin
    let _ = app.emit("model-download-complete", size);

    Ok(())
}

#[tauri::command]
pub async fn delete_model(state: State<'_, AppState>, size: ModelSize) -> Result<(), String> {
    state.model_manager.delete_model(size).await
}

#[tauri::command]
pub fn switch_model(state: State<'_, AppState>, size: ModelSize) -> Result<(), String> {
    // Vérifier que le modèle est disponible
    if !state.model_manager.is_model_available(size) {
        return Err(format!("Model {:?} is not available. Please download it first.", size));
    }

    // Recharger le moteur
    let settings = state.settings.read().map_err(|e| e.to_string())?;
    let language = if settings.auto_detect_language {
        None
    } else {
        Some(settings.transcription_language.clone())
    };
    drop(settings);

    state.reload_engine(size, language)?;

    // Mettre à jour les settings
    let mut settings = state.settings.write().map_err(|e| e.to_string())?;
    settings.whisper_model = size;
    drop(settings);

    // Sauvegarder
    let settings = state.settings.read().map_err(|e| e.to_string())?;
    crate::storage::config::save_settings(&settings)?;

    Ok(())
}

#[tauri::command]
pub fn is_engine_ready(state: State<'_, AppState>) -> bool {
    state.engine.read().map(|e| e.is_some()).unwrap_or(false)
}
```

**Step 2: Mettre à jour commands/mod.rs**

Ajouter le module:

```rust
pub mod audio;
pub mod models;
pub mod settings;
pub mod transcription;
```

**Step 3: Enregistrer les commandes dans lib.rs**

Ajouter les nouvelles commandes dans l'invocation de `invoke_handler`:

```rust
.invoke_handler(tauri::generate_handler![
    commands::transcription::start_recording,
    commands::transcription::stop_recording,
    commands::transcription::get_history,
    commands::transcription::clear_history,
    commands::transcription::get_recording_status,
    commands::settings::get_settings,
    commands::settings::update_settings,
    commands::settings::get_dictionary,
    commands::settings::add_dictionary_word,
    commands::settings::remove_dictionary_word,
    commands::audio::get_audio_devices,
    commands::models::get_available_models,
    commands::models::get_current_model,
    commands::models::download_model,
    commands::models::delete_model,
    commands::models::switch_model,
    commands::models::is_engine_ready,
])
```

**Step 4: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK

**Step 5: Commit**

```bash
git add src-tauri/src/commands/models.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat: add model management commands (download, switch, delete)"
```

---

## Task 8: Supprimer le code OpenVINO

**Files:**
- Delete: `src-tauri/src/engines/openvino.rs`
- Delete: `src-tauri/src/engines/vocabulary.rs`

**Step 1: Supprimer les fichiers**

```bash
rm src-tauri/src/engines/openvino.rs
rm src-tauri/src/engines/vocabulary.rs
```

**Step 2: Vérifier la compilation**

Run: `cd src-tauri && cargo check`
Expected: Compilation OK

**Step 3: Commit**

```bash
git add -A
git commit -m "refactor: remove OpenVINO engine code"
```

---

## Task 9: Mettre à jour le frontend - Types TypeScript

**Files:**
- Modify: `src/types/index.ts` (ou créer si n'existe pas)

**Step 1: Ajouter les types dans le frontend**

Créer/modifier `src/types/index.ts`:

```typescript
export type ModelSize = 'tiny' | 'small' | 'medium';

export interface ModelInfo {
  size: ModelSize;
  display_name: string;
  available: boolean;
  size_bytes: number;
}

export interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export interface AppSettings {
  microphone_id: string | null;
  hotkey_push_to_talk: string;
  hotkey_toggle_record: string;
  transcription_language: string;
  auto_detect_language: boolean;
  theme: string;
  minimize_to_tray: boolean;
  auto_copy_to_clipboard: boolean;
  notification_on_complete: boolean;
  whisper_model: ModelSize;
}
```

**Step 2: Commit**

```bash
git add src/types/index.ts
git commit -m "feat: add TypeScript types for model management"
```

---

## Task 10: Mettre à jour le SettingsPanel

**Files:**
- Modify: `src/components/SettingsPanel.tsx`

**Step 1: Ajouter la section de gestion des modèles**

Ajouter les imports et états nécessaires en haut du fichier, puis ajouter la section Moteur entre Audio et Transcription:

```tsx
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useSettingsStore } from '../stores/settingsStore';
import { HotkeyInput } from './HotkeyInput';
import { ModelSize, ModelInfo, DownloadProgress } from '../types';

interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export function SettingsPanel({ isOpen, onClose }: SettingsPanelProps) {
  const { settings, devices, dictionary, loadSettings, loadDevices, loadDictionary, updateSettings, addWord, removeWord } = useSettingsStore();
  const [newWord, setNewWord] = useState('');
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<ModelSize | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);

  useEffect(() => {
    if (isOpen) {
      loadSettings();
      loadDevices();
      loadDictionary();
      loadModels();
    }
  }, [isOpen, loadSettings, loadDevices, loadDictionary]);

  useEffect(() => {
    const unlistenProgress = listen<DownloadProgress>('model-download-progress', (event) => {
      setDownloadProgress(event.payload);
    });

    const unlistenComplete = listen<ModelSize>('model-download-complete', () => {
      setDownloading(null);
      setDownloadProgress(null);
      loadModels();
    });

    return () => {
      unlistenProgress.then(fn => fn());
      unlistenComplete.then(fn => fn());
    };
  }, []);

  const loadModels = async () => {
    try {
      const result = await invoke<ModelInfo[]>('get_available_models');
      setModels(result);
    } catch (e) {
      console.error('Failed to load models:', e);
    }
  };

  const handleDownloadModel = async (size: ModelSize) => {
    setDownloading(size);
    setDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    try {
      await invoke('download_model', { size });
    } catch (e) {
      console.error('Download failed:', e);
      setDownloading(null);
      setDownloadProgress(null);
    }
  };

  const handleSwitchModel = async (size: ModelSize) => {
    try {
      await invoke('switch_model', { size });
      await loadSettings();
    } catch (e) {
      console.error('Switch failed:', e);
    }
  };

  const handleAddWord = async () => {
    if (newWord.trim()) {
      await addWord(newWord.trim());
      setNewWord('');
    }
  };

  if (!isOpen || !settings) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={onClose}
      />

      {/* Panel */}
      <div className="settings-panel relative w-full max-w-md h-full bg-[var(--bg-panel)] border-l border-[var(--border-subtle)] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex-shrink-0 px-5 py-4 bg-[var(--bg-elevated)] border-b border-[var(--border-subtle)] flex justify-between items-center">
          <div className="flex items-center gap-3">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="var(--accent-cyan)" strokeWidth="1.5">
              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
            <h2 className="font-display font-semibold text-[var(--text-primary)] tracking-tight">
              Configuration
            </h2>
          </div>
          <button
            onClick={onClose}
            className="w-8 h-8 flex items-center justify-center rounded border border-[var(--border-subtle)] hover:border-[var(--accent-red)] hover:text-[var(--accent-red)] transition-colors"
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5 space-y-6 scrollbar-thin">
          {/* Audio Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-cyan)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-cyan)]/30" />
              Audio
            </h3>

            <div className="space-y-3">
              <label className="block">
                <span className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Microphone
                </span>
                <select
                  value={settings.microphone_id || ''}
                  onChange={(e) => updateSettings({ microphone_id: e.target.value || null })}
                  className="select-field w-full"
                >
                  <option value="">Par défaut</option>
                  {devices.map((device) => (
                    <option key={device.id} value={device.id}>
                      {device.name} {device.is_default ? '(défaut)' : ''}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </section>

          {/* Engine Section - NEW */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-green)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-green)]/30" />
              Moteur Whisper
            </h3>

            <div className="space-y-2">
              {models.map((model) => (
                <div
                  key={model.size}
                  className={`flex items-center justify-between p-3 border rounded ${
                    settings.whisper_model === model.size
                      ? 'border-[var(--accent-green)] bg-[var(--accent-green)]/5'
                      : 'border-[var(--border-subtle)]'
                  }`}
                >
                  <div className="flex items-center gap-3">
                    <div className={`w-2 h-2 rounded-full ${
                      settings.whisper_model === model.size
                        ? 'bg-[var(--accent-green)]'
                        : 'bg-[var(--border-subtle)]'
                    }`} />
                    <div>
                      <div className="text-sm text-[var(--text-primary)]">
                        {model.display_name}
                      </div>
                      {model.size === 'small' && (
                        <div className="text-[0.6rem] text-[var(--accent-cyan)]">Recommandé</div>
                      )}
                    </div>
                  </div>

                  {downloading === model.size ? (
                    <div className="flex items-center gap-2">
                      <div className="w-20 h-1.5 bg-[var(--bg-elevated)] rounded overflow-hidden">
                        <div
                          className="h-full bg-[var(--accent-cyan)] transition-all"
                          style={{ width: `${downloadProgress?.percent || 0}%` }}
                        />
                      </div>
                      <span className="text-[0.6rem] text-[var(--text-muted)] w-10">
                        {Math.round(downloadProgress?.percent || 0)}%
                      </span>
                    </div>
                  ) : model.available ? (
                    settings.whisper_model === model.size ? (
                      <span className="text-[0.6rem] text-[var(--accent-green)] uppercase">Actif</span>
                    ) : (
                      <button
                        onClick={() => handleSwitchModel(model.size)}
                        className="text-[0.6rem] text-[var(--accent-cyan)] hover:underline uppercase"
                      >
                        Utiliser
                      </button>
                    )
                  ) : (
                    <button
                      onClick={() => handleDownloadModel(model.size)}
                      className="text-[0.6rem] text-[var(--text-muted)] hover:text-[var(--accent-cyan)] uppercase flex items-center gap-1"
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                        <polyline points="7 10 12 15 17 10" />
                        <line x1="12" y1="15" x2="12" y2="3" />
                      </svg>
                      Télécharger
                    </button>
                  )}
                </div>
              ))}
            </div>
          </section>

          {/* Transcription Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-magenta)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-magenta)]/30" />
              Transcription
            </h3>

            <div className="space-y-3">
              <label className="block">
                <span className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Langue
                </span>
                <select
                  value={settings.auto_detect_language ? 'auto' : settings.transcription_language}
                  onChange={(e) => {
                    if (e.target.value === 'auto') {
                      updateSettings({ auto_detect_language: true });
                    } else {
                      updateSettings({
                        transcription_language: e.target.value,
                        auto_detect_language: false
                      });
                    }
                  }}
                  className="select-field w-full"
                >
                  <option value="auto">Automatique (détection)</option>
                  <option value="fr">Français</option>
                  <option value="en">English</option>
                  <option value="de">Deutsch</option>
                  <option value="es">Español</option>
                  <option value="it">Italiano</option>
                  <option value="pt">Português</option>
                  <option value="nl">Nederlands</option>
                  <option value="pl">Polski</option>
                  <option value="ru">Русский</option>
                  <option value="ja">日本語</option>
                  <option value="zh">中文</option>
                  <option value="ko">한국어</option>
                </select>
              </label>
            </div>
          </section>

          {/* Rest of the sections remain the same... */}
          {/* Appearance Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-green)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-green)]/30" />
              Apparence
            </h3>

            <div className="space-y-3">
              <label className="block">
                <span className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Thème
                </span>
                <select
                  value={settings.theme}
                  onChange={(e) => updateSettings({ theme: e.target.value as 'light' | 'dark' | 'system' })}
                  className="select-field w-full"
                >
                  <option value="system">Système</option>
                  <option value="light">Clair</option>
                  <option value="dark">Sombre</option>
                </select>
              </label>
            </div>
          </section>

          {/* Options Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--text-secondary)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--border-subtle)]" />
              Options
            </h3>

            <div className="space-y-3">
              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.auto_copy_to_clipboard}
                  onChange={(e) => updateSettings({ auto_copy_to_clipboard: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Copier automatiquement dans le presse-papier
                </span>
              </label>

              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.notification_on_complete}
                  onChange={(e) => updateSettings({ notification_on_complete: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Notification à la fin de la transcription
                </span>
              </label>

              <label className="checkbox-field">
                <input
                  type="checkbox"
                  checked={settings.minimize_to_tray}
                  onChange={(e) => updateSettings({ minimize_to_tray: e.target.checked })}
                />
                <span className="checkmark" />
                <span className="text-sm text-[var(--text-secondary)]">
                  Minimiser dans la barre système
                </span>
              </label>
            </div>
          </section>

          {/* Shortcuts Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-cyan)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-cyan)]/30" />
              Raccourcis
            </h3>

            <div className="space-y-3">
              <div>
                <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Push-to-talk (maintenir)
                </label>
                <HotkeyInput
                  value={settings.hotkey_push_to_talk}
                  onChange={(hotkey) => updateSettings({ hotkey_push_to_talk: hotkey })}
                />
              </div>
              <div>
                <label className="text-[0.7rem] uppercase tracking-wider text-[var(--text-muted)] mb-2 block">
                  Toggle enregistrement
                </label>
                <HotkeyInput
                  value={settings.hotkey_toggle_record}
                  onChange={(hotkey) => updateSettings({ hotkey_toggle_record: hotkey })}
                />
              </div>
            </div>
            <p className="text-[0.6rem] text-[var(--text-muted)]">
              Les raccourcis sont appliqués immédiatement.
            </p>
          </section>

          {/* Dictionary Section */}
          <section className="space-y-4">
            <h3 className="text-[0.65rem] uppercase tracking-[0.2em] text-[var(--accent-magenta)] font-medium flex items-center gap-2">
              <span className="w-8 h-px bg-[var(--accent-magenta)]/30" />
              Dictionnaire
            </h3>

            <div className="flex gap-2">
              <input
                type="text"
                value={newWord}
                onChange={(e) => setNewWord(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleAddWord()}
                placeholder="Ajouter un mot..."
                className="input-field flex-1"
              />
              <button
                onClick={handleAddWord}
                className="btn-panel px-3 text-[var(--accent-cyan)] border-[var(--accent-cyan)]/30 hover:bg-[var(--accent-cyan)]/10"
              >
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                  <line x1="12" y1="5" x2="12" y2="19" />
                  <line x1="5" y1="12" x2="19" y2="12" />
                </svg>
              </button>
            </div>

            {dictionary.length > 0 && (
              <div className="flex flex-wrap gap-2">
                {dictionary.map((word) => (
                  <span
                    key={word}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 bg-[var(--bg-elevated)] border border-[var(--border-subtle)] text-sm text-[var(--text-secondary)] group"
                  >
                    {word}
                    <button
                      onClick={() => removeWord(word)}
                      className="opacity-40 hover:opacity-100 hover:text-[var(--accent-red)] transition-opacity"
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <line x1="18" y1="6" x2="6" y2="18" />
                        <line x1="6" y1="6" x2="18" y2="18" />
                      </svg>
                    </button>
                  </span>
                ))}
              </div>
            )}
          </section>
        </div>

        {/* Footer */}
        <div className="flex-shrink-0 px-5 py-3 bg-[var(--bg-elevated)] border-t border-[var(--border-subtle)]">
          <p className="text-[0.6rem] text-[var(--text-muted)] text-center uppercase tracking-wider">
            WakaScribe v1.0.0 · Whisper.cpp
          </p>
        </div>
      </div>
    </div>
  );
}
```

**Step 2: Vérifier la compilation frontend**

Run: `npm run build`
Expected: Build OK

**Step 3: Commit**

```bash
git add src/components/SettingsPanel.tsx src/types/index.ts
git commit -m "feat: add Whisper model selection UI in SettingsPanel"
```

---

## Task 11: Mettre à jour tauri.conf.json

**Files:**
- Modify: `src-tauri/tauri.conf.json`

**Step 1: Mettre à jour les ressources**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "wakascribe-temp",
  "version": "0.1.0",
  "identifier": "com.cyprien.wakascribe-temp",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "wakascribe-temp",
        "width": 800,
        "height": 600
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "resources": [
      "resources/models/ggml-tiny.bin"
    ]
  }
}
```

**Step 2: Commit**

```bash
git add src-tauri/tauri.conf.json
git commit -m "build: update resources for Whisper model bundling"
```

---

## Task 12: Télécharger le modèle tiny et nettoyer les ressources

**Files:**
- Delete: `src-tauri/resources/openvino/`
- Delete: `src-tauri/resources/models/parakeet_*`
- Create: `src-tauri/resources/models/ggml-tiny.bin`

**Step 1: Nettoyer les anciennes ressources**

```bash
rm -rf src-tauri/resources/openvino
rm -rf src-tauri/resources/openvino.zip
rm -f src-tauri/resources/models/parakeet_*
rm -f src-tauri/resources/models/*.json
```

**Step 2: Télécharger le modèle tiny**

```bash
mkdir -p src-tauri/resources/models
curl -L -o src-tauri/resources/models/ggml-tiny.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin
```

**Step 3: Commit**

```bash
git add -A
git commit -m "build: replace Parakeet models with Whisper ggml-tiny"
```

---

## Task 13: Mettre à jour la documentation

**Files:**
- Modify: `CLAUDE.md`
- Delete: `docs/OPENVINO_STT_IMPLEMENTATION.md`

**Step 1: Mettre à jour CLAUDE.md**

Remplacer la section "Architecture des moteurs STT":

```markdown
## Architecture du moteur STT

WakaScribe utilise Whisper.cpp via les bindings whisper-rs pour la reconnaissance vocale:

| Modèle   | Taille   | Qualité    | Disponibilité |
|----------|----------|------------|---------------|
| Tiny     | 75 MB    | Basique    | Bundlé        |
| Small    | 466 MB   | Bonne      | Téléchargeable |
| Medium   | 1.5 GB   | Très bonne | Téléchargeable |

- **Tiny** est inclus dans l'application par défaut
- Les modèles Small et Medium peuvent être téléchargés depuis les paramètres
- Support de 99 langues avec détection automatique
```

**Step 2: Supprimer l'ancienne documentation**

```bash
rm -f docs/OPENVINO_STT_IMPLEMENTATION.md
```

**Step 3: Commit**

```bash
git add CLAUDE.md
git add -A
git commit -m "docs: update documentation for Whisper.cpp migration"
```

---

## Task 14: Test final et vérification

**Step 1: Build complet**

Run: `npm run tauri build`
Expected: Build successful

**Step 2: Lancer l'application**

Run: `npm run tauri dev`
Expected: L'application démarre avec le moteur Whisper

**Step 3: Tester la transcription**

1. Ouvrir l'application
2. Cliquer sur le bouton d'enregistrement
3. Parler quelques secondes
4. Arrêter l'enregistrement
5. Vérifier que le texte est transcrit

**Step 4: Tester le changement de modèle**

1. Ouvrir les paramètres
2. Télécharger le modèle "Small"
3. Basculer vers le modèle "Small"
4. Refaire une transcription

**Step 5: Commit final**

```bash
git add -A
git commit -m "feat: complete Whisper.cpp migration"
```

---

Plan complet et sauvegardé dans `docs/plans/2026-02-03-whisper-migration.md`. Deux options d'exécution:

**1. Subagent-Driven (cette session)** - Je dispatche un subagent frais par tâche, review entre les tâches, itération rapide

**2. Session Parallèle (séparée)** - Ouvrir une nouvelle session avec executing-plans, exécution par batch avec checkpoints

**Quelle approche ?**
