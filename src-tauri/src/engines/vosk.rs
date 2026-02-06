use crate::engines::traits::SpeechEngine;
use crate::types::{TranscriptionResult, VoskLanguage};
use chrono::Utc;
use std::path::Path;
use std::sync::Mutex;
use vosk::{Model, Recognizer};

pub struct VoskEngine {
    model: Mutex<Model>,
    language: VoskLanguage,
}

impl VoskEngine {
    pub fn new(model_path: &Path, language: VoskLanguage) -> Result<Self, String> {
        log::info!("Loading Vosk model from {:?}", model_path);

        if !model_path.exists() {
            return Err(format!("Vosk model not found: {:?}", model_path));
        }

        let model = Model::new(model_path.to_str().ok_or("Invalid path")?)
            .ok_or("Failed to load Vosk model")?;

        log::info!("Vosk model loaded successfully");

        Ok(Self {
            model: Mutex::new(model),
            language,
        })
    }

    pub fn language(&self) -> VoskLanguage {
        self.language
    }
}

impl SpeechEngine for VoskEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        let start_time = std::time::Instant::now();

        if sample_rate != 16000 {
            return Err(format!(
                "Invalid sample rate: {}Hz (expected 16000Hz)",
                sample_rate
            ));
        }

        let duration_seconds = audio.len() as f32 / sample_rate as f32;

        let model = self.model.lock().map_err(|e| format!("Lock error: {}", e))?;

        let mut recognizer = Recognizer::new(&model, sample_rate as f32)
            .ok_or("Failed to create recognizer")?;

        recognizer.set_words(true);

        // Process audio in chunks to avoid full i16 buffer allocation
        const CHUNK_SIZE: usize = 4096;
        for chunk in audio.chunks(CHUNK_SIZE) {
            let chunk_i16: Vec<i16> = chunk
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect();
            recognizer.accept_waveform(&chunk_i16);
        }

        let result = recognizer.final_result();
        let text = result.single().map(|r| r.text.to_string()).unwrap_or_default();

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        log::info!(
            "Vosk transcription completed in {}ms: {} chars",
            processing_time_ms,
            text.len()
        );

        Ok(TranscriptionResult {
            text: text.trim().to_string(),
            confidence: 0.9,
            duration_seconds,
            processing_time_ms,
            detected_language: Some(format!("{:?}", self.language).to_lowercase()),
            timestamp: Utc::now().timestamp(),
            model_used: Some(self.model_display_name()),
        })
    }

    fn name(&self) -> &str {
        "Vosk"
    }

    fn model_display_name(&self) -> String {
        format!("Vosk {}", self.language.display_name())
    }
}

unsafe impl Send for VoskEngine {}
unsafe impl Sync for VoskEngine {}
