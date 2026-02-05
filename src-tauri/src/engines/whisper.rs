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
            .full_lang_id_from_state()
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
            model_used: Some(self.model_display_name()),
        })
    }

    fn name(&self) -> &str {
        "Whisper"
    }

    fn model_display_name(&self) -> String {
        format!("Whisper {}", self.model_size.display_name())
    }
}

unsafe impl Send for WhisperEngine {}
unsafe impl Sync for WhisperEngine {}
