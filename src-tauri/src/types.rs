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

/// Provider pour les fonctionnalités LLM (résumé, post-traitement)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    #[default]
    Groq,
    Local,
}

/// Taille des modèles LLM locaux
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum LocalLlmModel {
    #[serde(rename = "smollm2_360m")]
    SmolLM2_360M,    // ~386 MB - Ultra rapide, résumés basiques
    #[default]
    #[serde(rename = "phi3_mini")]
    Phi3Mini,        // ~2.2 GB - Excellent rapport qualité/taille (recommandé)
    #[serde(rename = "qwen2_5_3b")]
    Qwen2_5_3B,      // ~2.0 GB - Très bonne qualité
}

impl LocalLlmModel {
    pub fn file_name(&self) -> &'static str {
        match self {
            LocalLlmModel::SmolLM2_360M => "SmolLM2-360M-Instruct-Q8_0.gguf",
            LocalLlmModel::Phi3Mini => "Phi-3-mini-4k-instruct-Q4_K_M.gguf",
            LocalLlmModel::Qwen2_5_3B => "qwen2.5-3b-instruct-q4_k_m.gguf",
        }
    }

    pub fn download_url(&self) -> &'static str {
        match self {
            LocalLlmModel::SmolLM2_360M => "https://huggingface.co/bartowski/SmolLM2-360M-Instruct-GGUF/resolve/main/SmolLM2-360M-Instruct-Q8_0.gguf",
            LocalLlmModel::Phi3Mini => "https://huggingface.co/bartowski/Phi-3-mini-4k-instruct-GGUF/resolve/main/Phi-3-mini-4k-instruct-Q4_K_M.gguf",
            LocalLlmModel::Qwen2_5_3B => "https://huggingface.co/Qwen/Qwen2.5-3B-Instruct-GGUF/resolve/main/qwen2.5-3b-instruct-q4_k_m.gguf",
        }
    }

    pub fn size_bytes(&self) -> u64 {
        match self {
            LocalLlmModel::SmolLM2_360M => 386_000_000,    // ~386 MB
            LocalLlmModel::Phi3Mini => 2_200_000_000,      // ~2.2 GB
            LocalLlmModel::Qwen2_5_3B => 2_000_000_000,    // ~2.0 GB
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LocalLlmModel::SmolLM2_360M => "SmolLM2 360M (386 MB) - Rapide",
            LocalLlmModel::Phi3Mini => "Phi-3 Mini (2.2 GB) - Recommandé",
            LocalLlmModel::Qwen2_5_3B => "Qwen2.5 3B (2 GB) - Qualité",
        }
    }

    /// Format du prompt pour ce modèle avec une instruction personnalisée
    pub fn format_prompt(&self, instruction: &str, text: &str) -> String {
        match self {
            LocalLlmModel::SmolLM2_360M => {
                format!(
                    "<|im_start|>user\n{}\n\n{}<|im_end|>\n<|im_start|>assistant\n",
                    instruction, text
                )
            }
            LocalLlmModel::Phi3Mini => {
                format!(
                    "<|user|>\n{}\n\n{}<|end|>\n<|assistant|>\n",
                    instruction, text
                )
            }
            LocalLlmModel::Qwen2_5_3B => {
                format!(
                    "<|im_start|>user\n{}\n\n{}<|im_end|>\n<|im_start|>assistant\n",
                    instruction, text
                )
            }
        }
    }
}

// Alias pour compatibilité
pub type QwenModelSize = LocalLlmModel;

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
    #[serde(default = "default_hotkey_voice_action")]
    pub hotkey_voice_action: String,
    #[serde(default)]
    pub engine_type: EngineType,
    #[serde(default)]
    pub vosk_language: Option<VoskLanguage>,
    #[serde(default)]
    pub parakeet_model: ParakeetModelSize,
    #[serde(default)]
    pub groq_api_key: Option<String>,
    #[serde(default)]
    pub llm_provider: LlmProvider,
    #[serde(default)]
    pub local_llm_model: LocalLlmModel,
    #[serde(default)]
    pub onboarding_completed: bool,
    #[serde(default)]
    pub tour_completed: bool,
    #[serde(default)]
    pub system_commands_enabled: bool,
    #[serde(default)]
    pub subtitles_enabled: bool,
    #[serde(default = "default_subtitles_font_size")]
    pub subtitles_font_size: u32,
    #[serde(default = "default_true")]
    pub stats_tracking_enabled: bool,
    #[serde(default)]
    pub integrations: IntegrationConfig,
}

fn default_true() -> bool {
    true
}

fn default_translation_language() -> String {
    "en".to_string()
}

fn default_hotkey_translate() -> String {
    "Control+Alt+T".to_string()
}

fn default_hotkey_voice_action() -> String {
    "Control+Alt+A".to_string()
}

fn default_subtitles_font_size() -> u32 {
    20
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            microphone_id: None,
            hotkey_push_to_talk: "Control+Space".to_string(),
            hotkey_toggle_record: "Control+Shift+R".to_string(),
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
            hotkey_translate: "Control+Alt+T".to_string(),
            hotkey_voice_action: "Control+Alt+A".to_string(),
            engine_type: EngineType::default(),
            vosk_language: None,
            parakeet_model: ParakeetModelSize::default(),
            groq_api_key: None,
            llm_provider: LlmProvider::default(),
            local_llm_model: LocalLlmModel::default(),
            onboarding_completed: false,
            tour_completed: false,
            system_commands_enabled: false,
            subtitles_enabled: false,
            subtitles_font_size: 20,
            stats_tracking_enabled: true,
            integrations: IntegrationConfig::default(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub id: String,
    pub name: String,
    pub trigger: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SnippetsData {
    pub snippets: Vec<Snippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DailyStats {
    pub words: u64,
    pub transcriptions: u32,
    pub duration_secs: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageStats {
    pub total_words: u64,
    pub total_transcriptions: u64,
    pub total_duration_secs: f64,
    pub daily_stats: std::collections::HashMap<String, DailyStats>,
    pub languages_used: std::collections::HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntegrationConfig {
    #[serde(default)]
    pub apple_notes_enabled: bool,
    #[serde(default)]
    pub obsidian_enabled: bool,
    #[serde(default)]
    pub obsidian_vault_path: Option<String>,
}
