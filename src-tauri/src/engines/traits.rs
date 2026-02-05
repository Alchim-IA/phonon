use crate::types::TranscriptionResult;

pub trait SpeechEngine: Send + Sync {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String>;
    fn name(&self) -> &str;
    fn model_display_name(&self) -> String;
}
