use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::engines::EngineError;

pub struct Vocabulary {
    tokens: HashMap<u32, String>,
    blank_token_id: u32,
}

impl Vocabulary {
    pub fn load(vocab_path: &Path) -> Result<Self, EngineError> {
        let content = fs::read_to_string(vocab_path)
            .map_err(|e| EngineError::VocabularyError(format!("Failed to read vocab file: {}", e)))?;

        // Le fichier JSON est un mapping string -> token_id
        let vocab_data: HashMap<String, u32> = serde_json::from_str(&content)
            .map_err(|e| EngineError::VocabularyError(format!("Failed to parse vocab JSON: {}", e)))?;

        // Inverser le mapping: token_id -> token_string
        let tokens: HashMap<u32, String> = vocab_data
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect();

        // Le blank token est généralement 8192 pour Parakeet v3
        let blank_token_id = 8192;

        log::info!("Loaded vocabulary with {} tokens, blank_id={}", tokens.len(), blank_token_id);

        Ok(Self {
            tokens,
            blank_token_id,
        })
    }

    pub fn blank_token_id(&self) -> u32 {
        self.blank_token_id
    }

    pub fn decode(&self, token_ids: &[u32]) -> String {
        let mut result = String::new();

        for &token_id in token_ids {
            // Skip blank tokens
            if token_id == self.blank_token_id {
                continue;
            }

            if let Some(token) = self.tokens.get(&token_id) {
                // Parakeet utilise des tokens SentencePiece avec ▁ pour les espaces
                let text = token.replace('▁', " ");
                result.push_str(&text);
            }
        }

        result.trim().to_string()
    }

    pub fn vocab_size(&self) -> usize {
        self.tokens.len()
    }
}
