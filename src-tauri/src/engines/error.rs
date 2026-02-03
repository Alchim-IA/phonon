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
