use crate::engines::traits::SpeechEngine;
use crate::types::TranscriptionResult;
use std::time::Instant;

pub struct OpenVINOEngine {
    language: String,
}

impl OpenVINOEngine {
    pub fn new(language: &str) -> Result<Self, String> {
        log::info!("Initializing OpenVINO engine for language: {}", language);
        Ok(Self {
            language: language.to_string(),
        })
    }

    pub fn mock() -> Self {
        Self {
            language: "fr".to_string(),
        }
    }
}

impl SpeechEngine for OpenVINOEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        let start = Instant::now();
        let duration_seconds = audio.len() as f32 / sample_rate as f32;

        // TODO: Implémenter la vraie transcription OpenVINO
        let text = format!(
            "[OpenVINO Mock] Audio de {:.1} secondes transcrit en {}",
            duration_seconds, self.language
        );

        Ok(TranscriptionResult {
            text,
            confidence: 0.92,
            duration_seconds,
            processing_time_ms: start.elapsed().as_millis() as u64 + 100,
            detected_language: Some(self.language.clone()),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    fn name(&self) -> &str {
        "OpenVINO"
    }
}
