# WakaScribe - Intelligence LLM et Commandes Vocales

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ajouter le post-traitement LLM via Groq et les commandes vocales contextuelles

**Architecture:** Pipeline Whisper → Détecteur commandes → LLM Groq (optionnel) → Résultat

**Tech Stack:** Rust/Tauri, Groq API (Llama 3.1 70B), keyring pour stockage sécurisé

---

## 1. Architecture globale

### Flux de traitement
```
Audio → Whisper (transcription brute) → Détecteur de commandes → LLM Groq (si activé) → Résultat final
```

### Nouveaux composants backend (Rust)
- `src-tauri/src/llm/groq_client.rs` - Client HTTP pour l'API Groq
- `src-tauri/src/llm/post_processor.rs` - Logique de post-traitement avec modes
- `src-tauri/src/llm/mod.rs` - Module LLM
- `src-tauri/src/voice_commands/parser.rs` - Parseur de commandes vocales
- `src-tauri/src/voice_commands/mod.rs` - Module commandes vocales

### Nouveaux types
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmMode {
    Off,        // Transcription brute uniquement
    Basic,      // Ponctuation + majuscules + grammaire
    Smart,      // Basic + suppression hésitations + reformulation
    Contextual, // Smart + adaptation au mode de dictée
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DictationMode {
    General,
    Email,
    Code,
    Notes,
}
```

### Nouveaux paramètres AppSettings
```rust
// LLM
llm_enabled: bool,              // false par défaut
llm_mode: LlmMode,              // Basic par défaut si activé

// Commandes vocales
voice_commands_enabled: bool,   // true par défaut
dictation_mode: DictationMode,  // General par défaut
```

Note: La clé API Groq est stockée via keyring, pas dans AppSettings.

---

## 2. Commandes vocales

### Commandes de ponctuation (détection automatique)
| Voix | Résultat |
|------|----------|
| "point" | `.` |
| "virgule" | `,` |
| "point d'interrogation" | `?` |
| "point d'exclamation" | `!` |
| "deux points" | `:` |
| "point virgule" | `;` |
| "ouvrir parenthèse" / "fermer parenthèse" | `(` / `)` |
| "ouvrir guillemets" / "fermer guillemets" | `«` / `»` |
| "à la ligne" | `\n` |
| "nouveau paragraphe" | `\n\n` |

### Commandes d'édition (préfixe "commande")
| Voix | Action |
|------|--------|
| "commande efface" | Supprime le dernier mot/phrase |
| "commande annuler" | Annule la dernière action |
| "commande tout effacer" | Vide le texte courant |
| "commande majuscules" | Met en majuscules la dernière phrase |
| "commande copier" | Copie dans le presse-papier |
| "commande stop" | Arrête l'enregistrement |

### Commandes contextuelles par mode
- **Email** : "commande signature", "commande formule politesse"
- **Code** : "commande fonction", "commande commentaire"
- **Notes** : "commande puce", "commande titre"

### Détection hybride
- Ponctuation : détection automatique basée sur le contexte (pause, position)
- Actions d'édition : requiert le préfixe "commande"
- Le LLM en mode Smart/Contextual corrige les faux positifs éventuels

---

## 3. Intégration Groq

### Configuration API
- Endpoint : `https://api.groq.com/openai/v1/chat/completions`
- Modèle : `llama-3.1-70b-versatile`
- Timeout : 5 secondes
- Fallback : transcription brute si erreur/timeout

### Prompts par mode

**Mode Basic:**
```
Tu es un correcteur de texte. Corrige uniquement la ponctuation, les majuscules et les fautes de grammaire évidentes. Ne modifie pas le sens ni le style. Retourne uniquement le texte corrigé, sans explication.

Texte: {transcription}
```

**Mode Smart:**
```
Tu es un assistant d'écriture. Corrige la ponctuation et la grammaire, supprime les hésitations (euh, hum, ben) et les répétitions inutiles. Reformule légèrement pour plus de clarté si nécessaire. Retourne uniquement le texte amélioré.

Texte: {transcription}
```

**Mode Contextual - Email:**
```
Tu es un assistant d'écriture professionnelle. Transforme ce texte dicté en email professionnel. Ajoute les formules de politesse appropriées si absentes. Garde un ton formel mais naturel. Retourne uniquement l'email formaté.

Texte: {transcription}
```

**Mode Contextual - Code:**
```
Tu es un assistant technique. Formate ce texte en documentation de code ou commentaire technique. Utilise la terminologie appropriée. Structure clairement. Retourne uniquement le texte formaté.

Texte: {transcription}
```

**Mode Contextual - Notes:**
```
Tu es un assistant de prise de notes. Organise ce texte en notes structurées avec puces si approprié. Garde les points essentiels, supprime le superflu. Retourne uniquement les notes formatées.

Texte: {transcription}
```

### Stockage clé API
- Utilisation de `keyring` (déjà dans le projet)
- Service : `wakascribe`
- Compte : `groq_api_key`
- Commandes Tauri : `set_groq_api_key`, `get_groq_api_key`, `validate_groq_api_key`

### Latence estimée
- Whisper small : ~2s
- Groq : ~200-500ms
- Total : ~2.5s (acceptable)

---

## 4. Interface utilisateur

### SettingsPanel - Nouvelle section "Intelligence (LLM)"
```
┌─ Intelligence (LLM) ───────────────────────────────┐
│                                                     │
│  [Toggle] Activer le post-traitement LLM           │
│                                                     │
│  Clé API Groq:                                     │
│  [••••••••••••••••] [👁] [Obtenir une clé →]       │
│  (lien vers https://console.groq.com/keys)         │
│                                                     │
│  Mode de correction:                               │
│  ○ Basique - ponctuation et grammaire              │
│  ○ Intelligent - reformulation claire              │
│  ○ Contextuel - adapté au mode de dictée           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### SettingsPanel - Nouvelle section "Mode de dictée"
```
┌─ Mode de dictée ────────────────────────────────────┐
│                                                     │
│  [Général] [Email] [Code] [Notes]                  │
│                                                     │
│  [Toggle] Commandes vocales activées               │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### DictationPanel - Indicateurs
- Badge "LLM" cyan quand activé (à côté du statut)
- Processing en 2 étapes : "Transcription..." puis "Amélioration..."
- LED différente pour l'étape LLM (cyan au lieu de magenta)

### Footer App.tsx
- Affiche le mode actif : "Whisper.cpp · Email · LLM" ou "Whisper.cpp · General"

### Types TypeScript
```typescript
type LlmMode = 'off' | 'basic' | 'smart' | 'contextual';
type DictationMode = 'general' | 'email' | 'code' | 'notes';

interface AppSettings {
  // ... existants ...
  llm_enabled: boolean;
  llm_mode: LlmMode;
  voice_commands_enabled: boolean;
  dictation_mode: DictationMode;
}
```

---

## 5. Gestion des erreurs

### Erreurs Groq
| Erreur | Comportement |
|--------|--------------|
| Clé invalide (401) | Message d'erreur, désactive LLM |
| Rate limit (429) | Fallback transcription brute, notification discrète |
| Timeout (>5s) | Fallback transcription brute |
| Hors-ligne | Fallback transcription brute, indicateur "offline" |

### Validation clé API
- Test avec requête minimale à la sauvegarde
- Affiche indicateur vert (✓) ou rouge (✗)
- Message d'erreur explicite si invalide

### Commandes vocales - Faux positifs
- Le LLM en mode Smart/Contextual corrige naturellement
- En mode Basic/Off : détection contextuelle (pauses, position dans phrase)
- Possibilité de désactiver les commandes vocales si trop de faux positifs

---

## 6. Résumé des fichiers à créer/modifier

### Nouveaux fichiers
- `src-tauri/src/llm/mod.rs`
- `src-tauri/src/llm/groq_client.rs`
- `src-tauri/src/llm/post_processor.rs`
- `src-tauri/src/voice_commands/mod.rs`
- `src-tauri/src/voice_commands/parser.rs`
- `src-tauri/src/commands/llm.rs`

### Fichiers à modifier
- `src-tauri/src/types.rs` - Nouveaux enums et champs settings
- `src-tauri/src/lib.rs` - Enregistrer nouveaux modules et commandes
- `src-tauri/src/commands/transcription.rs` - Intégrer pipeline LLM
- `src-tauri/src/storage/config.rs` - Nouveaux champs par défaut
- `src-tauri/Cargo.toml` - Pas de nouvelles dépendances (reqwest déjà présent)
- `src/types/index.ts` - Nouveaux types TypeScript
- `src/components/SettingsPanel.tsx` - Nouvelles sections UI
- `src/components/DictationPanel.tsx` - Indicateurs LLM
- `src/App.tsx` - Footer avec mode actif
