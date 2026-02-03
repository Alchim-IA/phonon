use keyring::Entry;

use crate::llm::groq_client;

const SERVICE_NAME: &str = "wakascribe";
const ACCOUNT_NAME: &str = "groq_api_key";

/// Stocke la clé API Groq dans le keyring sécurisé du système
#[tauri::command]
pub fn set_groq_api_key(key: String) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    entry
        .set_password(&key)
        .map_err(|e| format!("Failed to store API key: {}", e))
}

/// Récupère la clé API Groq depuis le keyring (pour validation interne)
#[tauri::command]
pub fn get_groq_api_key() -> Option<String> {
    get_groq_api_key_internal()
}

/// Récupère la clé API Groq depuis le keyring (usage interne sans attribut tauri::command)
pub fn get_groq_api_key_internal() -> Option<String> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME).ok()?;
    entry.get_password().ok()
}

/// Vérifie si une clé API Groq existe dans le keyring
#[tauri::command]
pub fn has_groq_api_key() -> bool {
    let entry = match Entry::new(SERVICE_NAME, ACCOUNT_NAME) {
        Ok(e) => e,
        Err(_) => return false,
    };

    entry.get_password().is_ok()
}

/// Valide une clé API Groq en effectuant une requête de test à l'API
#[tauri::command]
pub async fn validate_groq_api_key(key: String) -> bool {
    // Envoie un message simple pour vérifier que la clé fonctionne
    match groq_client::send_completion(&key, "Reply with OK", "test").await {
        Ok(_) => {
            log::info!("Groq API key validated successfully");
            true
        }
        Err(groq_client::GroqError::InvalidApiKey) => {
            log::warn!("Groq API key is invalid (401 Unauthorized)");
            false
        }
        Err(groq_client::GroqError::RateLimit) => {
            // Rate limit signifie que la clé est valide mais on a trop de requêtes
            log::info!("Groq API key valid (rate limited)");
            true
        }
        Err(e) => {
            // Autres erreurs (réseau, timeout, etc.)
            // On considère la clé valide si c'est juste un problème réseau
            log::warn!("Groq API validation error: {}. Assuming key is valid.", e);
            true
        }
    }
}

/// Supprime la clé API Groq du keyring
#[tauri::command]
pub fn delete_groq_api_key() -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, ACCOUNT_NAME)
        .map_err(|e| format!("Failed to access keyring entry: {}", e))?;

    entry
        .delete_credential()
        .map_err(|e| format!("Failed to delete API key: {}", e))
}

/// Traduit un texte vers une langue cible via Groq
#[tauri::command]
pub async fn translate_text(text: String, target_language: String) -> Result<String, String> {
    let api_key = get_groq_api_key_internal()
        .ok_or_else(|| "No Groq API key configured".to_string())?;

    let language_name = match target_language.as_str() {
        "fr" => "French",
        "en" => "English",
        "de" => "German",
        "es" => "Spanish",
        "it" => "Italian",
        "pt" => "Portuguese",
        "nl" => "Dutch",
        "ru" => "Russian",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        "ar" => "Arabic",
        _ => &target_language,
    };

    let system_prompt = format!(
        "You are a professional translator. Translate the following text to {}. \
         Only output the translation, nothing else. Preserve the original formatting, \
         punctuation and tone. If the text is already in {}, return it unchanged.",
        language_name, language_name
    );

    match groq_client::send_completion(&api_key, &system_prompt, &text).await {
        Ok(translated) => {
            log::info!("Translation successful: {} -> {}", text.len(), translated.len());
            Ok(translated.trim().to_string())
        }
        Err(e) => {
            log::error!("Translation failed: {}", e);
            Err(format!("Translation failed: {}", e))
        }
    }
}
