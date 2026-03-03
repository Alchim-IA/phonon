use crate::engines::traits::SpeechEngine;
use crate::types::TranscriptionResult;
use chrono::Utc;
use serde::Deserialize;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
struct SidecarOutput {
    text: String,
    confidence: f64,
    processing_time_ms: i64,
    error: Option<String>,
    command: Option<String>,
}

struct DaemonProcess {
    child: Child,
    stdin: std::process::ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
}

pub struct ParakeetCoreMLEngine {
    sidecar_path: PathBuf,
    daemon: Mutex<Option<DaemonProcess>>,
}

impl ParakeetCoreMLEngine {
    pub fn new(sidecar_path: PathBuf) -> Result<Self, String> {
        if !sidecar_path.exists() {
            return Err(format!(
                "Parakeet CoreML sidecar not found: {:?}",
                sidecar_path
            ));
        }

        let mut engine = Self {
            sidecar_path,
            daemon: Mutex::new(None),
        };

        // Start the daemon and wait for it to be ready
        engine.start_daemon()?;

        log::info!("ParakeetCoreMLEngine initialized with persistent daemon");
        Ok(engine)
    }

    fn start_daemon(&mut self) -> Result<(), String> {
        let mut child = Command::new(&self.sidecar_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start sidecar daemon: {}", e))?;

        let stdin = child.stdin.take()
            .ok_or("Failed to get stdin of sidecar daemon")?;
        let stdout = child.stdout.take()
            .ok_or("Failed to get stdout of sidecar daemon")?;

        let mut reader = BufReader::new(stdout);

        // Wait for the "ready" message
        let mut line = String::new();
        reader.read_line(&mut line)
            .map_err(|e| format!("Failed to read ready message from daemon: {}", e))?;

        let output: SidecarOutput = serde_json::from_str(line.trim())
            .map_err(|e| format!("Failed to parse ready message: {} (got: {})", e, line.trim()))?;

        if output.command.as_deref() != Some("ready") {
            if let Some(error) = output.error {
                return Err(format!("Daemon failed to start: {}", error));
            }
            return Err(format!("Expected ready message, got: {}", line.trim()));
        }

        log::info!("Parakeet CoreML daemon is ready");

        *self.daemon.lock().map_err(|e| e.to_string())? = Some(DaemonProcess {
            child,
            stdin,
            reader,
        });

        Ok(())
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

        let result = {
            let mut daemon_guard = self.daemon.lock().map_err(|e| e.to_string())?;
            let daemon = daemon_guard.as_mut()
                .ok_or("Daemon not running")?;

            // Send the request as a JSON line
            let request = format!(
                "{{\"audio_path\":\"{}\"}}\n",
                temp_wav.to_str().unwrap().replace('\\', "\\\\").replace('"', "\\\"")
            );
            daemon.stdin.write_all(request.as_bytes())
                .map_err(|e| format!("Failed to write to daemon stdin: {}", e))?;
            daemon.stdin.flush()
                .map_err(|e| format!("Failed to flush daemon stdin: {}", e))?;

            // Read the response
            let mut response_line = String::new();
            daemon.reader.read_line(&mut response_line)
                .map_err(|e| format!("Failed to read daemon response: {}", e))?;

            let output: SidecarOutput = serde_json::from_str(response_line.trim())
                .map_err(|e| format!("Failed to parse daemon output: {} (output: {})", e, response_line.trim()))?;

            output
        };

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_wav);

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

impl Drop for ParakeetCoreMLEngine {
    fn drop(&mut self) {
        if let Ok(mut daemon_guard) = self.daemon.lock() {
            if let Some(mut daemon) = daemon_guard.take() {
                // Try to send quit command gracefully
                let quit_msg = b"{\"command\":\"quit\"}\n";
                let _ = daemon.stdin.write_all(quit_msg);
                let _ = daemon.stdin.flush();

                // Wait briefly for graceful shutdown, then kill
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = daemon.child.kill();
                let _ = daemon.child.wait();
                log::info!("Parakeet CoreML daemon stopped");
            }
        }
    }
}

unsafe impl Send for ParakeetCoreMLEngine {}
unsafe impl Sync for ParakeetCoreMLEngine {}
