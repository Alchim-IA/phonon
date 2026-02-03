use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmMode {
    Off,
    Basic,
    Smart,
    Contextual,
}

impl Default for LlmMode {
    fn default() -> Self {
        LlmMode::Off
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DictationMode {
    General,
    Email,
    Code,
    Notes,
}

impl Default for DictationMode {
    fn default() -> Self {
        DictationMode::General
    }
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub confidence: f32,
    pub duration_seconds: f32,
    pub processing_time_ms: u64,
    pub detected_language: Option<String>,
    pub timestamp: i64,
}

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
    pub whisper_model: ModelSize,
    pub llm_enabled: bool,
    pub llm_mode: LlmMode,
    pub voice_commands_enabled: bool,
    pub dictation_mode: DictationMode,
    pub streaming_enabled: bool,
    pub auto_paste_enabled: bool,
    pub floating_window_enabled: bool,
    pub floating_window_position: Option<(i32, i32)>,
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
            whisper_model: ModelSize::Tiny,
            llm_enabled: false,
            llm_mode: LlmMode::default(),
            voice_commands_enabled: true,
            dictation_mode: DictationMode::default(),
            streaming_enabled: true,
            auto_paste_enabled: true,
            floating_window_enabled: false,
            floating_window_position: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DictionaryData {
    pub words: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryData {
    pub transcriptions: Vec<TranscriptionResult>,
}
