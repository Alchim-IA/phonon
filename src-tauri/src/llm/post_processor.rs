use crate::types::{DictationMode, LlmMode};

use super::groq_client;

const PROMPT_BASIC: &str = "Tu es un correcteur de texte. Corrige uniquement la ponctuation, les majuscules et les fautes de grammaire évidentes. Ne modifie pas le sens ni le style. Retourne uniquement le texte corrigé, sans explication.";

const PROMPT_SMART: &str = "Tu es un assistant d'écriture. Corrige la ponctuation et la grammaire, supprime les hésitations (euh, hum, ben) et les répétitions inutiles. Reformule légèrement pour plus de clarté si nécessaire. Retourne uniquement le texte amélioré.";

const PROMPT_EMAIL: &str = "Tu es un assistant d'écriture professionnelle. Transforme ce texte dicté en email professionnel. Ajoute les formules de politesse appropriées si absentes. Garde un ton formel mais naturel. Retourne uniquement l'email formaté.";

const PROMPT_CODE: &str = "Tu es un assistant technique. Formate ce texte en documentation de code ou commentaire technique. Utilise la terminologie appropriée. Structure clairement. Retourne uniquement le texte formaté.";

const PROMPT_NOTES: &str = "Tu es un assistant de prise de notes. Organise ce texte en notes structurées avec puces si approprié. Garde les points essentiels, supprime le superflu. Retourne uniquement les notes formatées.";

fn get_prompt(llm_mode: LlmMode, dictation_mode: DictationMode) -> &'static str {
    match llm_mode {
        LlmMode::Off => "",
        LlmMode::Basic => PROMPT_BASIC,
        LlmMode::Smart => PROMPT_SMART,
        LlmMode::Contextual => match dictation_mode {
            DictationMode::Email => PROMPT_EMAIL,
            DictationMode::Code => PROMPT_CODE,
            DictationMode::Notes => PROMPT_NOTES,
            DictationMode::General => PROMPT_SMART,
        },
    }
}

pub async fn process(
    text: &str,
    llm_mode: LlmMode,
    dictation_mode: DictationMode,
    api_key: &str,
) -> Result<String, String> {
    // If LLM mode is Off, return text as-is
    if llm_mode == LlmMode::Off {
        return Ok(text.to_string());
    }

    let prompt = get_prompt(llm_mode, dictation_mode);
    let user_message = format!("Texte: {}", text);

    match groq_client::send_completion(api_key, prompt, &user_message).await {
        Ok(processed_text) => Ok(processed_text),
        Err(e) => {
            // Log the error and return original text (graceful fallback)
            log::error!("LLM post-processing failed: {}. Returning original text.", e);
            Ok(text.to_string())
        }
    }
}
