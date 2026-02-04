use crate::engines::traits::SpeechEngine;
use crate::types::TranscriptionResult;
use chrono::Utc;
use std::path::Path;

/// Parakeet model size options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParakeetModelSize {
    #[default]
    Tdt06bV3, // parakeet-tdt-0.6b-v3 - multilingual
}

impl ParakeetModelSize {
    pub fn model_name(&self) -> &'static str {
        match self {
            ParakeetModelSize::Tdt06bV3 => "parakeet-tdt-0.6b-v3",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ParakeetModelSize::Tdt06bV3 => "Parakeet TDT 0.6B v3 (Multilingual)",
        }
    }
}

impl From<crate::types::ParakeetModelSize> for ParakeetModelSize {
    fn from(value: crate::types::ParakeetModelSize) -> Self {
        match value {
            crate::types::ParakeetModelSize::Tdt06bV3 => ParakeetModelSize::Tdt06bV3,
        }
    }
}

pub struct ParakeetEngine {
    model_path: std::path::PathBuf,
    language: Option<String>,
    model_size: ParakeetModelSize,
}

impl ParakeetEngine {
    pub fn new(model_path: &Path, model_size: ParakeetModelSize) -> Result<Self, String> {
        log::info!("Loading Parakeet model from {:?}", model_path);

        if !model_path.exists() {
            return Err(format!("Parakeet model not found: {:?}", model_path));
        }

        // Note: Full sherpa-rs integration requires native compilation
        // This is a placeholder that will be expanded when sherpa-rs is available

        log::info!("Parakeet model path verified: {:?}", model_path);

        Ok(Self {
            model_path: model_path.to_path_buf(),
            language: None,
            model_size,
        })
    }

    pub fn model_size(&self) -> ParakeetModelSize {
        self.model_size
    }

    pub fn set_language(&mut self, language: Option<String>) {
        self.language = language;
    }
}

impl SpeechEngine for ParakeetEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        let start_time = std::time::Instant::now();

        if sample_rate != 16000 {
            return Err(format!(
                "Invalid sample rate: {}Hz (expected 16000Hz)",
                sample_rate
            ));
        }

        let duration_seconds = audio.len() as f32 / sample_rate as f32;

        // Placeholder: sherpa-rs integration would go here
        // For now, return an error indicating the engine needs native compilation
        #[cfg(not(feature = "parakeet"))]
        {
            let _ = (start_time, duration_seconds);
            return Err(
                "Parakeet engine requires building with --features parakeet (needs CMake)"
                    .to_string(),
            );
        }

        #[cfg(feature = "parakeet")]
        {
            // TODO: Implement actual sherpa-rs transcription
            // This requires the sherpa-rs crate to be properly compiled with CMake
            let _ = audio;

            let processing_time_ms = start_time.elapsed().as_millis() as u64;

            Ok(TranscriptionResult {
                text: String::new(),
                confidence: 0.0,
                duration_seconds,
                processing_time_ms,
                detected_language: self.language.clone(),
                timestamp: Utc::now().timestamp(),
            })
        }
    }

    fn name(&self) -> &str {
        "Parakeet"
    }
}

unsafe impl Send for ParakeetEngine {}
unsafe impl Sync for ParakeetEngine {}
