//! Voice command parser for Phonon
//!
//! Parses transcribed text to detect and replace punctuation commands,
//! extract editing actions, and handle contextual commands based on dictation mode.

use crate::types::DictationMode;
use crate::types::Snippet;

/// Actions that can be triggered by voice commands
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Delete the last word or selection
    Delete,
    /// Undo the last action
    Undo,
    /// Clear all text
    ClearAll,
    /// Convert selection to uppercase
    Uppercase,
    /// Copy selection to clipboard
    Copy,
    /// Stop dictation
    Stop,
    /// Insert email signature (Email mode)
    InsertSignature,
    /// Insert greeting/politeness formula (Email mode)
    InsertGreeting,
    /// Insert function template (Code mode)
    InsertFunction,
    /// Insert comment (Code mode)
    InsertComment,
    /// Insert bullet point (Notes mode)
    InsertBullet,
    /// Insert title/heading (Notes mode)
    InsertTitle,
    /// Open an application by name
    OpenApp(String),
    /// Insert a snippet by its trigger word
    InsertSnippet(String),
    /// Set system volume (0-100)
    SetVolume(u8),
    /// Toggle Do Not Disturb
    ToggleDND,
    /// Take a screenshot
    Screenshot,
    /// Lock the screen
    LockScreen,
    /// Format selection as bold
    FormatBold,
    /// Format selection as italic
    FormatItalic,
    /// Format selection as underline
    FormatUnderline,
}

/// Result of parsing voice commands from text
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The processed text with punctuation commands replaced
    pub text: String,
    /// Actions extracted from command phrases
    pub actions: Vec<Action>,
}

/// Spacing behavior for punctuation
#[derive(Debug, Clone, Copy, PartialEq)]
enum SpacingRule {
    /// Remove space before, add space after (e.g., ".", ",", ";", ":", "?", "!")
    CloseWithSpace,
    /// Remove space before, no space after (e.g., ")" and ">>" when followed by punctuation)
    CloseNoSpace,
    /// Keep space before, remove space after (e.g., "(", "<<")
    OpenNoSpace,
    /// Remove space before, no space after but newline (e.g., "\n")
    Newline,
}

/// Punctuation command with its replacement and spacing rule
struct PunctuationMapping {
    command: &'static str,
    replacement: &'static str,
    spacing: SpacingRule,
}

/// Punctuation command mappings (French) - ordered from longest to shortest to avoid partial matches
const PUNCTUATION_COMMANDS: &[PunctuationMapping] = &[
    PunctuationMapping { command: "point d'interrogation", replacement: "?", spacing: SpacingRule::CloseWithSpace },
    PunctuationMapping { command: "point d'exclamation", replacement: "!", spacing: SpacingRule::CloseWithSpace },
    PunctuationMapping { command: "nouveau paragraphe", replacement: "\n\n", spacing: SpacingRule::Newline },
    PunctuationMapping { command: "ouvrir parenthèse", replacement: "(", spacing: SpacingRule::OpenNoSpace },
    PunctuationMapping { command: "fermer parenthèse", replacement: ")", spacing: SpacingRule::CloseWithSpace },
    PunctuationMapping { command: "ouvrir guillemets", replacement: "\u{00AB}", spacing: SpacingRule::OpenNoSpace },
    PunctuationMapping { command: "fermer guillemets", replacement: "\u{00BB}", spacing: SpacingRule::CloseWithSpace },
    PunctuationMapping { command: "point virgule", replacement: ";", spacing: SpacingRule::CloseWithSpace },
    PunctuationMapping { command: "deux points", replacement: ":", spacing: SpacingRule::CloseWithSpace },
    PunctuationMapping { command: "à la ligne", replacement: "\n", spacing: SpacingRule::Newline },
    PunctuationMapping { command: "virgule", replacement: ",", spacing: SpacingRule::CloseWithSpace },
    PunctuationMapping { command: "point", replacement: ".", spacing: SpacingRule::CloseWithSpace },
];

/// Trigger words for opening apps (French, imperative forms only to avoid
/// conflicts with punctuation commands like "ouvrir parenthèse")
const APP_TRIGGERS: &[&str] = &[
    "ouvre",
    "lance",
    "mets",
    "démarre",
    "demarre",
];

/// Edit command mappings (prefixed with "commande")
const EDIT_COMMANDS: &[(&str, Action)] = &[
    ("commande tout effacer", Action::ClearAll),
    ("commande majuscules", Action::Uppercase),
    ("commande annuler", Action::Undo),
    ("commande efface", Action::Delete),
    ("commande copier", Action::Copy),
    ("commande stop", Action::Stop),
];

/// Email mode contextual commands
const EMAIL_COMMANDS: &[(&str, Action)] = &[
    ("commande formule politesse", Action::InsertGreeting),
    ("commande signature", Action::InsertSignature),
];

/// Code mode contextual commands
const CODE_COMMANDS: &[(&str, Action)] = &[
    ("commande commentaire", Action::InsertComment),
    ("commande fonction", Action::InsertFunction),
];

/// Notes mode contextual commands
const NOTES_COMMANDS: &[(&str, Action)] = &[
    ("commande titre", Action::InsertTitle),
    ("commande puce", Action::InsertBullet),
];

/// System commands (require system_commands_enabled)
const SYSTEM_COMMANDS: &[(&str, Action)] = &[
    ("commande screenshot", Action::Screenshot),
    ("commande capture ecran", Action::Screenshot),
    ("commande capture écran", Action::Screenshot),
    ("commande verrouille", Action::LockScreen),
    ("commande ne pas déranger", Action::ToggleDND),
    ("commande ne pas deranger", Action::ToggleDND),
];

/// Format commands
const FORMAT_COMMANDS: &[(&str, Action)] = &[
    ("mets en gras", Action::FormatBold),
    ("en gras", Action::FormatBold),
    ("mets en italique", Action::FormatItalic),
    ("en italique", Action::FormatItalic),
    ("commande souligne", Action::FormatUnderline),
    ("commande souligné", Action::FormatUnderline),
];

/// Parse voice commands from transcribed text
///
/// This function processes the input text to:
/// 1. Replace punctuation commands with their corresponding characters
/// 2. Extract editing commands into the actions vector
/// 3. Extract contextual commands based on the current dictation mode
///
/// # Arguments
///
/// * `text` - The transcribed text to parse
/// * `mode` - The current dictation mode (affects which contextual commands are recognized)
/// * `snippets` - Available snippets for insertion commands
/// * `system_commands_enabled` - Whether system commands (volume, screenshot, etc.) are enabled
///
/// # Returns
///
/// A `ParseResult` containing the processed text and any extracted actions
pub fn parse(text: &str, mode: DictationMode, snippets: &[Snippet], system_commands_enabled: bool) -> ParseResult {
    let mut result_text = text.to_string();
    let mut actions = Vec::new();

    // Get contextual commands based on mode
    let contextual_commands: &[(&str, Action)] = match mode {
        DictationMode::Email => EMAIL_COMMANDS,
        DictationMode::Code => CODE_COMMANDS,
        DictationMode::Notes => NOTES_COMMANDS,
        DictationMode::General => &[],
    };

    // Extract contextual commands first (they may contain "commande" prefix)
    for (command, action) in contextual_commands {
        result_text = extract_command(&result_text, command, action, &mut actions);
    }

    // Extract edit commands
    for (command, action) in EDIT_COMMANDS {
        result_text = extract_command(&result_text, command, action, &mut actions);
    }

    // Extract snippet insertion commands
    for snippet in snippets {
        let triggers = [
            format!("insere {}", snippet.trigger),
            format!("insère {}", snippet.trigger),
        ];
        for trigger in &triggers {
            let text_lower = result_text.to_lowercase();
            let trigger_lower = trigger.to_lowercase();
            if let Some(pos) = text_lower.find(&trigger_lower) {
                actions.push(Action::InsertSnippet(snippet.trigger.clone()));
                let mut new_text = String::new();
                let before = &result_text[..pos];
                new_text.push_str(before.trim_end());
                let after = &result_text[pos + trigger.len()..];
                if !new_text.is_empty() && !after.trim_start().is_empty() {
                    new_text.push(' ');
                }
                new_text.push_str(after.trim_start());
                result_text = new_text;
                break;
            }
        }
    }

    // Extract format commands (before app commands to avoid "mets" trigger conflict)
    for (command, action) in FORMAT_COMMANDS {
        result_text = extract_command(&result_text, command, action, &mut actions);
    }

    // Extract system commands if enabled
    if system_commands_enabled {
        for (command, action) in SYSTEM_COMMANDS {
            result_text = extract_command(&result_text, command, action, &mut actions);
        }

        // Volume command: "commande volume [à] XX"
        let text_lower = result_text.to_lowercase();
        if let Some(vol_pos) = text_lower.find("commande volume") {
            let after_volume = &result_text[vol_pos + "commande volume".len()..];
            let trimmed = after_volume.trim_start();
            // Skip "a " or "à " if present
            let prefix_len = if trimmed.starts_with("à ") {
                "à ".len()
            } else if trimmed.starts_with("a ") {
                "a ".len()
            } else {
                0
            };
            let num_part = &trimmed[prefix_len..];
            // Parse the number
            let num_str: String = num_part.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(level) = num_str.parse::<u8>() {
                let level = level.min(100);
                actions.push(Action::SetVolume(level));
                // Remove the entire "commande volume [à] XX" from text
                let end_pos = vol_pos + "commande volume".len() + (after_volume.len() - after_volume.trim_start().len()) + prefix_len + num_str.len();
                let mut new_text = String::new();
                new_text.push_str(result_text[..vol_pos].trim_end());
                let remaining = &result_text[end_pos..];
                if !new_text.is_empty() && !remaining.trim_start().is_empty() {
                    new_text.push(' ');
                }
                new_text.push_str(remaining.trim_start());
                result_text = new_text;
            }
        }
    }

    // Extract app open commands ("ouvre Safari", "lance Spotify", etc.)
    result_text = extract_app_commands(&result_text, &mut actions);

    // Replace punctuation commands (case-insensitive)
    for mapping in PUNCTUATION_COMMANDS {
        result_text = replace_punctuation_command(&result_text, mapping);
    }

    // Clean up extra whitespace
    result_text = clean_whitespace(&result_text);

    ParseResult {
        text: result_text,
        actions,
    }
}

/// Extract app open commands from text (e.g. "ouvre Safari", "lance Spotify")
/// Takes only the first word after the trigger as the app name.
/// Matches on word boundaries to avoid matching inside longer words (e.g. "ouvre" inside "ouvrir").
fn extract_app_commands(text: &str, actions: &mut Vec<Action>) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let words_lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();

    for trigger in APP_TRIGGERS {
        let trigger_lower = trigger.to_lowercase();
        for (i, word) in words_lower.iter().enumerate() {
            if *word == trigger_lower {
                // Found the trigger as a whole word; next word is the app name
                if i + 1 >= words.len() {
                    continue;
                }
                let app_name = words[i + 1];
                actions.push(Action::OpenApp(app_name.to_string()));

                // Rebuild text without trigger and app name
                let mut parts: Vec<&str> = Vec::new();
                for (j, w) in words.iter().enumerate() {
                    if j != i && j != i + 1 {
                        parts.push(w);
                    }
                }
                return parts.join(" ");
            }
        }
    }

    text.to_string()
}

/// Extract a command from text and add its action to the actions vector
fn extract_command(text: &str, command: &str, action: &Action, actions: &mut Vec<Action>) -> String {
    let text_lower = text.to_lowercase();
    let command_lower = command.to_lowercase();

    if let Some(pos) = text_lower.find(&command_lower) {
        actions.push(action.clone());
        let mut result = String::new();
        let before = &text[..pos];
        result.push_str(before.trim_end());
        let after = &text[pos + command.len()..];
        if !result.is_empty() && !after.trim_start().is_empty() {
            result.push(' ');
        }
        result.push_str(after.trim_start());
        result
    } else {
        text.to_string()
    }
}

/// Replace a punctuation command with its corresponding character (case-insensitive)
fn replace_punctuation_command(text: &str, mapping: &PunctuationMapping) -> String {
    let text_lower = text.to_lowercase();
    let command_lower = mapping.command.to_lowercase();

    let mut result = String::new();
    let mut last_end = 0;

    for (start, _) in text_lower.match_indices(&command_lower) {
        let before = &text[last_end..start];
        let end = start + mapping.command.len();
        let after_start = if text[end..].starts_with(' ') { end + 1 } else { end };

        match mapping.spacing {
            SpacingRule::CloseWithSpace => {
                // Remove space before, add space after (if there's more text)
                result.push_str(before.trim_end());
                result.push_str(mapping.replacement);
                if after_start < text.len() && !text[after_start..].is_empty() {
                    result.push(' ');
                }
            }
            SpacingRule::CloseNoSpace => {
                // Remove space before, no space after
                result.push_str(before.trim_end());
                result.push_str(mapping.replacement);
            }
            SpacingRule::OpenNoSpace => {
                // Keep space before, remove space after
                result.push_str(before);
                if !result.ends_with(' ') && !result.is_empty() {
                    result.push(' ');
                }
                result.push_str(mapping.replacement);
            }
            SpacingRule::Newline => {
                // Remove space before, the replacement is the newline
                result.push_str(before.trim_end());
                result.push_str(mapping.replacement);
            }
        }

        last_end = after_start;
    }

    // Add remaining text
    result.push_str(&text[last_end..]);
    result
}

/// Clean up extra whitespace in the result
fn clean_whitespace(text: &str) -> String {
    // Replace multiple spaces with single space
    let mut result = String::new();
    let mut prev_was_space = false;
    let mut prev_was_newline = false;

    for c in text.chars() {
        if c == ' ' {
            // Don't add space after newline or if previous was space
            if !prev_was_space && !prev_was_newline {
                result.push(c);
            }
            prev_was_space = true;
            prev_was_newline = false;
        } else if c == '\n' {
            // Remove trailing space before newline
            if prev_was_space {
                result.pop();
            }
            result.push(c);
            prev_was_space = false;
            prev_was_newline = true;
        } else {
            result.push(c);
            prev_was_space = false;
            prev_was_newline = false;
        }
    }

    // Trim leading and trailing whitespace
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punctuation_point() {
        let result = parse("Bonjour point", DictationMode::General, &[], false);
        assert_eq!(result.text, "Bonjour.");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_virgule() {
        let result = parse("un virgule deux virgule trois", DictationMode::General, &[], false);
        assert_eq!(result.text, "un, deux, trois");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_question() {
        let result = parse("Comment allez-vous point d'interrogation", DictationMode::General, &[], false);
        assert_eq!(result.text, "Comment allez-vous?");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_exclamation() {
        let result = parse("Super point d'exclamation", DictationMode::General, &[], false);
        assert_eq!(result.text, "Super!");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_deux_points() {
        let result = parse("Voici deux points la liste", DictationMode::General, &[], false);
        assert_eq!(result.text, "Voici: la liste");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_point_virgule() {
        let result = parse("premier point virgule second", DictationMode::General, &[], false);
        assert_eq!(result.text, "premier; second");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_parentheses() {
        let result = parse("texte ouvrir parenthèse note fermer parenthèse suite", DictationMode::General, &[], false);
        assert_eq!(result.text, "texte (note) suite");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_guillemets() {
        let result = parse("il a dit ouvrir guillemets bonjour fermer guillemets", DictationMode::General, &[], false);
        assert_eq!(result.text, "il a dit \u{00AB}bonjour\u{00BB}");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_a_la_ligne() {
        let result = parse("première ligne à la ligne deuxième ligne", DictationMode::General, &[], false);
        assert_eq!(result.text, "première ligne\ndeuxième ligne");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_punctuation_nouveau_paragraphe() {
        let result = parse("premier paragraphe nouveau paragraphe second paragraphe", DictationMode::General, &[], false);
        assert_eq!(result.text, "premier paragraphe\n\nsecond paragraphe");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_case_insensitive() {
        // Commands are case-insensitive, but the surrounding text keeps its original case
        let result = parse("Bonjour POINT comment allez-vous Point D'Interrogation", DictationMode::General, &[], false);
        assert_eq!(result.text, "Bonjour. comment allez-vous?");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_edit_command_efface() {
        let result = parse("texte commande efface", DictationMode::General, &[], false);
        assert_eq!(result.text, "texte");
        assert_eq!(result.actions, vec![Action::Delete]);
    }

    #[test]
    fn test_edit_command_annuler() {
        let result = parse("erreur commande annuler", DictationMode::General, &[], false);
        assert_eq!(result.text, "erreur");
        assert_eq!(result.actions, vec![Action::Undo]);
    }

    #[test]
    fn test_edit_command_tout_effacer() {
        let result = parse("commande tout effacer", DictationMode::General, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::ClearAll]);
    }

    #[test]
    fn test_edit_command_majuscules() {
        let result = parse("titre commande majuscules", DictationMode::General, &[], false);
        assert_eq!(result.text, "titre");
        assert_eq!(result.actions, vec![Action::Uppercase]);
    }

    #[test]
    fn test_edit_command_copier() {
        let result = parse("texte important commande copier", DictationMode::General, &[], false);
        assert_eq!(result.text, "texte important");
        assert_eq!(result.actions, vec![Action::Copy]);
    }

    #[test]
    fn test_edit_command_stop() {
        let result = parse("fini commande stop", DictationMode::General, &[], false);
        assert_eq!(result.text, "fini");
        assert_eq!(result.actions, vec![Action::Stop]);
    }

    #[test]
    fn test_email_mode_signature() {
        let result = parse("Cordialement commande signature", DictationMode::Email, &[], false);
        assert_eq!(result.text, "Cordialement");
        assert_eq!(result.actions, vec![Action::InsertSignature]);
    }

    #[test]
    fn test_email_mode_formule_politesse() {
        let result = parse("commande formule politesse", DictationMode::Email, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::InsertGreeting]);
    }

    #[test]
    fn test_email_commands_not_in_general_mode() {
        let result = parse("commande signature", DictationMode::General, &[], false);
        assert_eq!(result.text, "commande signature");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_code_mode_fonction() {
        let result = parse("commande fonction", DictationMode::Code, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::InsertFunction]);
    }

    #[test]
    fn test_code_mode_commentaire() {
        let result = parse("commande commentaire", DictationMode::Code, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::InsertComment]);
    }

    #[test]
    fn test_notes_mode_puce() {
        let result = parse("commande puce premier élément", DictationMode::Notes, &[], false);
        assert_eq!(result.text, "premier élément");
        assert_eq!(result.actions, vec![Action::InsertBullet]);
    }

    #[test]
    fn test_notes_mode_titre() {
        let result = parse("commande titre Introduction", DictationMode::Notes, &[], false);
        assert_eq!(result.text, "Introduction");
        assert_eq!(result.actions, vec![Action::InsertTitle]);
    }

    #[test]
    fn test_multiple_punctuation_and_command() {
        let result = parse("Bonjour point Comment ça va point d'interrogation commande copier", DictationMode::General, &[], false);
        assert_eq!(result.text, "Bonjour. Comment ça va?");
        assert_eq!(result.actions, vec![Action::Copy]);
    }

    #[test]
    fn test_complex_sentence() {
        let result = parse(
            "Cher Monsieur virgule à la ligne Je vous écris pour vous informer que ouvrir parenthèse voir détails ci-dessous fermer parenthèse point nouveau paragraphe Cordialement",
            DictationMode::General,
            &[],
            false,
        );
        assert_eq!(
            result.text,
            "Cher Monsieur,\nJe vous écris pour vous informer que (voir détails ci-dessous).\n\nCordialement"
        );
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_whitespace_cleanup() {
        let result = parse("texte   avec   espaces", DictationMode::General, &[], false);
        assert_eq!(result.text, "texte avec espaces");
    }

    #[test]
    fn test_empty_input() {
        let result = parse("", DictationMode::General, &[], false);
        assert_eq!(result.text, "");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_no_commands() {
        let result = parse("Texte normal sans commandes", DictationMode::General, &[], false);
        assert_eq!(result.text, "Texte normal sans commandes");
        assert!(result.actions.is_empty());
    }

    // App open command tests

    #[test]
    fn test_open_app_ouvre() {
        let result = parse("ouvre Safari", DictationMode::General, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::OpenApp("Safari".to_string())]);
    }

    #[test]
    fn test_open_app_lance() {
        let result = parse("lance Spotify", DictationMode::General, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::OpenApp("Spotify".to_string())]);
    }

    #[test]
    fn test_open_app_mets() {
        let result = parse("mets Spotify", DictationMode::General, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::OpenApp("Spotify".to_string())]);
    }

    #[test]
    fn test_open_app_with_surrounding_text() {
        let result = parse("je veux ouvre Safari merci", DictationMode::General, &[], false);
        assert_eq!(result.text, "je veux merci");
        assert_eq!(result.actions, vec![Action::OpenApp("Safari".to_string())]);
    }

    #[test]
    fn test_open_app_case_insensitive() {
        let result = parse("Ouvre safari", DictationMode::General, &[], false);
        assert_eq!(result.actions, vec![Action::OpenApp("safari".to_string())]);
    }

    #[test]
    fn test_open_app_demarre() {
        let result = parse("démarre Firefox", DictationMode::General, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::OpenApp("Firefox".to_string())]);
    }

    #[test]
    fn test_open_app_trigger_alone_no_crash() {
        let result = parse("ouvre", DictationMode::General, &[], false);
        assert_eq!(result.text, "ouvre");
        assert!(result.actions.is_empty());
    }

    // Format command tests

    #[test]
    fn test_format_bold() {
        let result = parse("texte mets en gras", DictationMode::General, &[], false);
        assert_eq!(result.text, "texte");
        assert_eq!(result.actions, vec![Action::FormatBold]);
    }

    #[test]
    fn test_format_italic() {
        let result = parse("texte en italique", DictationMode::General, &[], false);
        assert_eq!(result.text, "texte");
        assert_eq!(result.actions, vec![Action::FormatItalic]);
    }

    #[test]
    fn test_format_underline() {
        let result = parse("texte commande souligné", DictationMode::General, &[], false);
        assert_eq!(result.text, "texte");
        assert_eq!(result.actions, vec![Action::FormatUnderline]);
    }

    // System command tests

    #[test]
    fn test_system_screenshot_enabled() {
        let result = parse("commande screenshot", DictationMode::General, &[], true);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::Screenshot]);
    }

    #[test]
    fn test_system_screenshot_disabled() {
        let result = parse("commande screenshot", DictationMode::General, &[], false);
        assert_eq!(result.text, "commande screenshot");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_system_lock_screen() {
        let result = parse("commande verrouille", DictationMode::General, &[], true);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::LockScreen]);
    }

    #[test]
    fn test_system_dnd() {
        let result = parse("commande ne pas déranger", DictationMode::General, &[], true);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::ToggleDND]);
    }

    #[test]
    fn test_system_volume() {
        let result = parse("commande volume 50", DictationMode::General, &[], true);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::SetVolume(50)]);
    }

    #[test]
    fn test_system_volume_with_a() {
        let result = parse("commande volume à 75", DictationMode::General, &[], true);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::SetVolume(75)]);
    }

    #[test]
    fn test_system_volume_capped_at_100() {
        let result = parse("commande volume 200", DictationMode::General, &[], true);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::SetVolume(100)]);
    }

    #[test]
    fn test_system_volume_disabled() {
        let result = parse("commande volume 50", DictationMode::General, &[], false);
        assert_eq!(result.text, "commande volume 50");
        assert!(result.actions.is_empty());
    }

    #[test]
    fn test_format_bold_does_not_trigger_mets_app() {
        // "mets en gras" should trigger FormatBold, not OpenApp("en")
        let result = parse("mets en gras", DictationMode::General, &[], false);
        assert_eq!(result.text, "");
        assert_eq!(result.actions, vec![Action::FormatBold]);
    }
}
