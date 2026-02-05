use crate::audio::AudioDecoder;
use crate::state::AppState;
use crate::storage::history;
use crate::types::TranscriptionResult;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTranscriptionResult {
    pub file_path: String,
    pub file_name: String,
    pub transcription: Option<TranscriptionResult>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileTranscriptionProgress {
    pub current: usize,
    pub total: usize,
    pub file_name: String,
    pub status: String,
}

/// Transcribe multiple audio files using the currently configured engine
#[tauri::command]
pub async fn transcribe_files(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<Vec<FileTranscriptionResult>, String> {
    let mut results = Vec::new();
    let total = paths.len();

    for (index, path_str) in paths.into_iter().enumerate() {
        let path = std::path::Path::new(&path_str);
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Emit progress: decoding
        let _ = app.emit(
            "file-transcription-progress",
            FileTranscriptionProgress {
                current: index + 1,
                total,
                file_name: file_name.clone(),
                status: "decoding".to_string(),
            },
        );

        // Check if format is supported
        if !AudioDecoder::is_supported(path) {
            results.push(FileTranscriptionResult {
                file_path: path_str,
                file_name,
                transcription: None,
                error: Some("Unsupported audio format".to_string()),
            });
            continue;
        }

        // Decode audio
        let (audio, sample_rate) = match AudioDecoder::decode_file(path) {
            Ok(data) => data,
            Err(e) => {
                results.push(FileTranscriptionResult {
                    file_path: path_str,
                    file_name,
                    transcription: None,
                    error: Some(format!("Failed to decode: {}", e)),
                });
                continue;
            }
        };

        // Emit progress: transcribing
        let _ = app.emit(
            "file-transcription-progress",
            FileTranscriptionProgress {
                current: index + 1,
                total,
                file_name: file_name.clone(),
                status: "transcribing".to_string(),
            },
        );

        // Transcribe using current engine
        let transcription = {
            let engine_guard = state.engine.read().map_err(|e| e.to_string())?;
            if let Some(ref engine) = *engine_guard {
                engine.transcribe(&audio, sample_rate)
            } else {
                Err("No engine initialized".to_string())
            }
        };

        match transcription {
            Ok(result) => {
                // Save to history
                if let Err(e) = history::add_transcription(result.clone()) {
                    log::warn!("Failed to save transcription to history: {}", e);
                }

                results.push(FileTranscriptionResult {
                    file_path: path_str.clone(),
                    file_name,
                    transcription: Some(result),
                    error: None,
                });
            }
            Err(e) => {
                results.push(FileTranscriptionResult {
                    file_path: path_str.clone(),
                    file_name,
                    transcription: None,
                    error: Some(e),
                });
            }
        }
    }

    // Emit completion
    let _ = app.emit(
        "file-transcription-progress",
        FileTranscriptionProgress {
            current: total,
            total,
            file_name: String::new(),
            status: "completed".to_string(),
        },
    );

    Ok(results)
}

/// Get list of supported audio formats
#[tauri::command]
pub fn get_supported_audio_formats() -> Vec<String> {
    AudioDecoder::supported_formats()
        .into_iter()
        .map(|s| s.to_string())
        .collect()
}
