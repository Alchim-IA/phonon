use crate::engines::traits::SpeechEngine;
use crate::types::TranscriptionResult;
use chrono::Utc;
use serde::Deserialize;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize)]
struct SidecarOutput {
    text: String,
    confidence: f64,
    processing_time_ms: i64,
    error: Option<String>,
}

pub struct ParakeetCoreMLEngine {
    sidecar_path: PathBuf,
}

impl ParakeetCoreMLEngine {
    pub fn new(sidecar_path: PathBuf) -> Result<Self, String> {
        if !sidecar_path.exists() {
            return Err(format!(
                "Parakeet CoreML sidecar not found: {:?}",
                sidecar_path
            ));
        }

        log::info!("ParakeetCoreMLEngine initialized with sidecar: {:?}", sidecar_path);
        Ok(Self { sidecar_path })
    }

    fn write_temp_wav(&self, audio: &[f32], sample_rate: u32) -> Result<PathBuf, String> {
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("parakeet_input_{}.wav", std::process::id()));

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create(&temp_path, spec)
            .map_err(|e| format!("Failed to create temp WAV: {}", e))?;

        for &sample in audio {
            writer
                .write_sample(sample)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer
            .finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;

        Ok(temp_path)
    }
}

impl SpeechEngine for ParakeetCoreMLEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        let start_time = std::time::Instant::now();

        if sample_rate != 16000 {
            return Err(format!(
                "Invalid sample rate: {}Hz (expected 16000Hz)",
                sample_rate
            ));
        }

        let duration_seconds = audio.len() as f32 / sample_rate as f32;

        if duration_seconds < 0.1 {
            return Err("Audio too short".to_string());
        }

        // Write audio to temporary WAV file
        let temp_wav = self.write_temp_wav(audio, sample_rate)?;

        // Call the sidecar
        let output = Command::new(&self.sidecar_path)
            .arg(temp_wav.to_str().unwrap())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to run sidecar: {}", e))?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_wav);

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Sidecar failed: {}", stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: SidecarOutput = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse sidecar output: {} (output: {})", e, stdout))?;

        if let Some(error) = result.error {
            return Err(error);
        }

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        log::info!(
            "ParakeetCoreML transcription completed in {}ms: {} chars",
            processing_time_ms,
            result.text.len()
        );

        Ok(TranscriptionResult {
            text: result.text,
            confidence: result.confidence as f32,
            duration_seconds,
            processing_time_ms,
            detected_language: Some("auto".to_string()),
            timestamp: Utc::now().timestamp(),
            model_used: Some(self.model_display_name()),
        })
    }

    fn name(&self) -> &str {
        "Parakeet CoreML"
    }

    fn model_display_name(&self) -> String {
        "Parakeet TDT 0.6B v3 (CoreML)".to_string()
    }
}

unsafe impl Send for ParakeetCoreMLEngine {}
unsafe impl Sync for ParakeetCoreMLEngine {}
