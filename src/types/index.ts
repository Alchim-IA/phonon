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

export interface AudioDevice {
  id: string;
  name: string;
  is_default: boolean;
}

export type TranscriptionStatus = 'idle' | 'recording' | 'processing' | 'completed' | 'error';

export interface StreamingChunk {
  text: string;
  is_final: boolean;
  duration_seconds: number;
}
