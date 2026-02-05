use std::path::Path;

use crate::types::LocalLlmModel;

/// Moteur LLM local (stub - désactivé pour ARM64)
/// Utilise Groq à la place pour les fonctionnalités LLM
pub struct LocalLlmEngine {
    model_type: LocalLlmModel,
}

impl LocalLlmEngine {
    pub fn new(_model_path: &Path, model_type: LocalLlmModel) -> Result<Self, String> {
        log::warn!("Local LLM is not available in this build. Use Groq instead.");
        Ok(Self { model_type })
    }

    /// Génère un résumé du texte donné
    pub fn summarize(&self, _text: &str) -> Result<String, String> {
        Err("Local LLM not available in this build. Please use Groq (cloud) instead.".to_string())
    }

    pub fn model_type(&self) -> LocalLlmModel {
        self.model_type
    }

    pub fn display_name(&self) -> String {
        format!("Local LLM ({}) - Not Available", self.model_type.display_name())
    }
}

unsafe impl Send for LocalLlmEngine {}
unsafe impl Sync for LocalLlmEngine {}
