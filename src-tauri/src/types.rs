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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineType {
    #[default]
    Whisper,
    Parakeet,
    Vosk,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ParakeetModelSize {
    #[default]
    Tdt06bV3,
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

impl EngineType {
    pub fn display_name(&self) -> &'static str {
        match self {
            EngineType::Whisper => "Whisper",
            EngineType::Parakeet => "Parakeet",
            EngineType::Vosk => "Vosk",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum VoskLanguage {
    En, Fr, De, Es, It, Ru, Zh, Ja, Ko, Pt, Nl, Pl, Uk, Tr, Vi, Ar, Hi, Fa, Ca, Cs,
}

impl VoskLanguage {
    pub fn model_name(&self) -> &'static str {
        match self {
            VoskLanguage::En => "vosk-model-small-en-us-0.15",
            VoskLanguage::Fr => "vosk-model-small-fr-0.22",
            VoskLanguage::De => "vosk-model-small-de-0.15",
            VoskLanguage::Es => "vosk-model-small-es-0.42",
            VoskLanguage::It => "vosk-model-small-it-0.22",
            VoskLanguage::Ru => "vosk-model-small-ru-0.22",
            VoskLanguage::Zh => "vosk-model-small-cn-0.22",
            VoskLanguage::Ja => "vosk-model-small-ja-0.22",
            VoskLanguage::Ko => "vosk-model-small-ko-0.22",
            VoskLanguage::Pt => "vosk-model-small-pt-0.3",
            VoskLanguage::Nl => "vosk-model-small-nl-0.22",
            VoskLanguage::Pl => "vosk-model-small-pl-0.22",
            VoskLanguage::Uk => "vosk-model-small-uk-v3-small",
            VoskLanguage::Tr => "vosk-model-small-tr-0.3",
            VoskLanguage::Vi => "vosk-model-small-vn-0.4",
            VoskLanguage::Ar => "vosk-model-ar-mgb2-0.4",
            VoskLanguage::Hi => "vosk-model-small-hi-0.22",
            VoskLanguage::Fa => "vosk-model-small-fa-0.5",
            VoskLanguage::Ca => "vosk-model-small-ca-0.4",
            VoskLanguage::Cs => "vosk-model-small-cs-0.4-rhasspy",
        }
    }

    pub fn download_url(&self) -> String {
        format!("https://alphacephei.com/vosk/models/{}.zip", self.model_name())
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            VoskLanguage::En => "English",
            VoskLanguage::Fr => "Français",
            VoskLanguage::De => "Deutsch",
            VoskLanguage::Es => "Español",
            VoskLanguage::It => "Italiano",
            VoskLanguage::Ru => "Русский",
            VoskLanguage::Zh => "中文",
            VoskLanguage::Ja => "日本語",
            VoskLanguage::Ko => "한국어",
            VoskLanguage::Pt => "Português",
            VoskLanguage::Nl => "Nederlands",
            VoskLanguage::Pl => "Polski",
            VoskLanguage::Uk => "Українська",
            VoskLanguage::Tr => "Türkçe",
            VoskLanguage::Vi => "Tiếng Việt",
            VoskLanguage::Ar => "العربية",
            VoskLanguage::Hi => "हिन्दी",
            VoskLanguage::Fa => "فارسی",
            VoskLanguage::Ca => "Català",
            VoskLanguage::Cs => "Čeština",
        }
    }

    pub fn from_language_code(code: &str) -> Option<Self> {
        match code.to_lowercase().as_str() {
            "en" | "english" => Some(VoskLanguage::En),
            "fr" | "french" => Some(VoskLanguage::Fr),
            "de" | "german" => Some(VoskLanguage::De),
            "es" | "spanish" => Some(VoskLanguage::Es),
            "it" | "italian" => Some(VoskLanguage::It),
            "ru" | "russian" => Some(VoskLanguage::Ru),
            "zh" | "chinese" => Some(VoskLanguage::Zh),
            "ja" | "japanese" => Some(VoskLanguage::Ja),
            "ko" | "korean" => Some(VoskLanguage::Ko),
            "pt" | "portuguese" => Some(VoskLanguage::Pt),
            "nl" | "dutch" => Some(VoskLanguage::Nl),
            "pl" | "polish" => Some(VoskLanguage::Pl),
            "uk" | "ukrainian" => Some(VoskLanguage::Uk),
            "tr" | "turkish" => Some(VoskLanguage::Tr),
            "vi" | "vietnamese" => Some(VoskLanguage::Vi),
            "ar" | "arabic" => Some(VoskLanguage::Ar),
            "hi" | "hindi" => Some(VoskLanguage::Hi),
            "fa" | "persian" => Some(VoskLanguage::Fa),
            "ca" | "catalan" => Some(VoskLanguage::Ca),
            "cs" | "czech" => Some(VoskLanguage::Cs),
            _ => None,
        }
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
    #[serde(default)]
    pub model_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
    #[serde(default = "default_true")]
    pub streaming_enabled: bool,
    #[serde(default = "default_true")]
    pub auto_paste_enabled: bool,
    #[serde(default)]
    pub floating_window_enabled: bool,
    #[serde(default)]
    pub floating_window_position: Option<(i32, i32)>,
    #[serde(default)]
    pub translation_enabled: bool,
    #[serde(default = "default_translation_language")]
    pub translation_target_language: String,
    #[serde(default = "default_hotkey_translate")]
    pub hotkey_translate: String,
    #[serde(default)]
    pub engine_type: EngineType,
    #[serde(default)]
    pub vosk_language: Option<VoskLanguage>,
    #[serde(default)]
    pub parakeet_model: ParakeetModelSize,
}

fn default_true() -> bool {
    true
}

fn default_translation_language() -> String {
    "en".to_string()
}

fn default_hotkey_translate() -> String {
    "CommandOrControl+Shift+T".to_string()
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
            translation_enabled: true,
            translation_target_language: "en".to_string(),
            hotkey_translate: "CommandOrControl+Shift+T".to_string(),
            engine_type: EngineType::default(),
            vosk_language: None,
            parakeet_model: ParakeetModelSize::default(),
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
