export interface TranscriptionResult {
  text: string;
  confidence: number;
  duration_seconds: number;
  processing_time_ms: number;
  detected_language: string | null;
  timestamp: number;
  model_used: string | null;
}

export type ModelSize = 'tiny' | 'small' | 'medium';

export type EngineType = 'whisper' | 'parakeet' | 'vosk';

export type VoskLanguage = 'en' | 'fr' | 'de' | 'es' | 'it' | 'ru' | 'zh' | 'ja' | 'ko' | 'pt' | 'nl' | 'pl' | 'uk' | 'tr' | 'vi' | 'ar' | 'hi' | 'fa' | 'ca' | 'cs';

export type ParakeetModelSize = 'tdt06bv3';

export interface ParakeetModelInfo {
  size: ParakeetModelSize;
  display_name: string;
  available: boolean;
  size_bytes: number;
}

export type LlmMode = 'off' | 'basic' | 'smart' | 'contextual';

export type LlmProvider = 'groq' | 'local';

export type LocalLlmModel = 'smollm2_360m' | 'qwen2_5_1_5b' | 'phi3_mini' | 'qwen2_5_3b';

export interface LocalLlmModelInfo {
  size: LocalLlmModel;
  display_name: string;
  available: boolean;
  size_bytes: number;
}

// Aliases for backward compatibility
export type QwenModelSize = LocalLlmModel;

export type DictationMode = 'general' | 'email' | 'code' | 'notes';

export interface ModelInfo {
  size: ModelSize;
  display_name: string;
  available: boolean;
  size_bytes: number;
}

export interface DownloadProgress {
  downloaded: number;
  total: number;
  percent: number;
}

export interface AppSettings {
  microphone_id: string | null;
  hotkey_push_to_talk: string;
  hotkey_toggle_record: string;
  transcription_language: string;
  auto_detect_language: boolean;
  theme: 'light' | 'dark' | 'system';
  minimize_to_tray: boolean;
  auto_copy_to_clipboard: boolean;
  notification_on_complete: boolean;
  whisper_model: ModelSize;
  engine_type: EngineType;
  vosk_language: VoskLanguage | null;
  parakeet_model: ParakeetModelSize;
  groq_api_key: string | null;
  llm_provider: LlmProvider;
  local_llm_model: LocalLlmModel;
  llm_enabled: boolean;
  llm_mode: LlmMode;
  voice_commands_enabled: boolean;
  dictation_mode: DictationMode;
  streaming_enabled: boolean;
  auto_paste_enabled: boolean;
  floating_window_enabled: boolean;
  translation_enabled: boolean;
  translation_target_language: string;
  hotkey_translate: string;
  hotkey_voice_action: string;
  onboarding_completed: boolean;
  tour_completed: boolean;
  system_commands_enabled: boolean;
  subtitles_enabled: boolean;
  subtitles_font_size: number;
  stats_tracking_enabled: boolean;
  integrations: IntegrationConfig;
}

export interface VoskModelInfo {
  language: VoskLanguage;
  display_name: string;
  available: boolean;
}

export interface FileTranscriptionResult {
  file_path: string;
  file_name: string;
  transcription: TranscriptionResult | null;
  error: string | null;
}

export interface FileTranscriptionProgress {
  current: number;
  total: number;
  file_name: string;
  status: string;
}

export interface Snippet {
  id: string;
  name: string;
  trigger: string;
  content: string;
}

export interface DailyStats {
  words: number;
  transcriptions: number;
  duration_secs: number;
}

export interface UsageStats {
  total_words: number;
  total_transcriptions: number;
  total_duration_secs: number;
  daily_stats: Record<string, DailyStats>;
  languages_used: Record<string, number>;
}

export interface IntegrationConfig {
  apple_notes_enabled: boolean;
  obsidian_enabled: boolean;
  obsidian_vault_path: string | null;
}

export interface AudioDevice {
  id: string;
  name: string;
  is_default: boolean;
}

export type TranscriptionStatus = 'idle' | 'recording' | 'processing' | 'completed' | 'error';

export interface GroqQuota {
  limit_requests: number | null;
  remaining_requests: number | null;
  limit_tokens: number | null;
  remaining_tokens: number | null;
  reset_requests: string | null;
  reset_tokens: string | null;
}

export interface StreamingChunk {
  text: string;
  is_final: boolean;
  duration_seconds: number;
}

export interface LlmDownloadProgress {
  model: LocalLlmModel;
  downloaded: number;
  total: number;
  progress: number;
}
