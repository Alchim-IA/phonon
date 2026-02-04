# Multi-Engine & File Transcription Design

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:writing-plans to create implementation plan from this design.

## Goal

Add Parakeet and Vosk speech engines alongside Whisper, plus audio file transcription with batch support.

## Key Decisions

- **Performance temps réel** : Modèles légers, latence minimale, téléchargeables à la demande
- **Transcription fichiers** : Choix du moteur/langue, traitement par lots
- **Formats audio** : WAV, MP3, M4A, FLAC, OGG, WEBM via symphonia
- **Multilingue complet** : Toutes les langues disponibles par moteur

---

## Architecture des moteurs

### Structure des fichiers

```
src-tauri/src/engines/
├── mod.rs              # Export des modules
├── traits.rs           # Trait SpeechEngine (existant)
├── whisper.rs          # WhisperEngine (existant)
├── parakeet.rs         # NOUVEAU - ParakeetEngine via sherpa-rs
├── vosk.rs             # NOUVEAU - VoskEngine via vosk-rs
└── model_manager.rs    # Étendre pour gérer tous les moteurs
```

### Trait existant

```rust
pub trait SpeechEngine: Send + Sync {
    fn transcribe(&self, audio: &[f32], sample_rate: u32)
        -> Result<TranscriptionResult, String>;
    fn name(&self) -> &str;
}
```

Chaque nouveau moteur implémente ce trait pour un switch transparent.

### Gestion des modèles

Le `ModelManager` gère le téléchargement à la demande et le cache local dans `~/.local/share/wakascribe/models/`.

---

## Moteurs et langues

### Parakeet (sherpa-onnx)

| Modèle | Taille | Langues | Mode |
|--------|--------|---------|------|
| parakeet-tdt-0.6b-v3 | ~600MB | 25 langues EU | Auto-détection ou langue forcée |

**Langues supportées :** EN, FR, DE, ES, IT, NL, RU, PL, UK, SK, BG, FI, RO, HR, CS, SV, ET, HU, LT, DA, MT, SL, LV, EL

**Source :** [nvidia/parakeet-tdt-0.6b-v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3)

### Vosk

Modèles par langue (~40MB chacun), téléchargés individuellement.

| Langue | Modèle |
|--------|--------|
| Anglais | vosk-model-small-en-us |
| Français | vosk-model-small-fr |
| Allemand | vosk-model-small-de |
| Espagnol | vosk-model-small-es |
| Italien | vosk-model-small-it |
| Russe | vosk-model-small-ru |
| Chinois | vosk-model-small-cn |
| + ~15 autres | vosk-model-small-* |

**Note :** Vosk n'a pas d'auto-détection, le modèle de langue doit être sélectionné.

---

## Transcription de fichiers

### Formats supportés

| Format | Crate |
|--------|-------|
| WAV | `hound` (existant) |
| MP3 | `symphonia` |
| M4A/AAC | `symphonia` |
| FLAC | `symphonia` |
| OGG | `symphonia` |
| WEBM | `symphonia` |

### Flow

1. Utilisateur sélectionne un ou plusieurs fichiers via dialog natif
2. Chaque fichier est décodé → resamplé à 16kHz mono
3. Transcription avec le moteur choisi (ou moteur actif par défaut)
4. Résultats ajoutés à l'historique avec métadonnées

### Commande Tauri

```rust
#[tauri::command]
async fn transcribe_files(
    paths: Vec<String>,
    engine: Option<EngineType>,
    language: Option<String>,
) -> Result<Vec<FileTranscriptionResult>, String>
```

### Progression

Événements Tauri émis pour chaque fichier (`file-transcription-progress`).

---

## Interface utilisateur

### Settings Panel - Section Moteurs

```
┌─ Moteurs de transcription ─────────────────────────┐
│                                                     │
│ Moteur actif: [Whisper ▼]                          │
│ Langue:       [Auto-détection ▼]                   │
│                                                     │
│ ── Whisper ──────────────────────────────────────  │
│ Modèle: [Small ▼]  [Télécharger Medium]            │
│ Status: ✓ Small installé (466 MB)                  │
│                                                     │
│ ── Parakeet ─────────────────────────────────────  │
│ Modèle: parakeet-tdt-0.6b-v3                       │
│ Status: ✗ Non installé  [Télécharger ~600 MB]      │
│                                                     │
│ ── Vosk ─────────────────────────────────────────  │
│ Modèles installés: FR, EN                          │
│ [+ Ajouter langue]  [Gérer...]                     │
│                                                     │
└─────────────────────────────────────────────────────┘
```

### Transcription de fichiers

```
┌─ Transcrire des fichiers ──────────────────────────┐
│                                                     │
│  [📁 Sélectionner fichiers...]                     │
│                                                     │
│  Moteur: [Utiliser moteur actif ▼]                 │
│  Langue: [Auto-détection ▼]                        │
│                                                     │
│  Fichiers sélectionnés:                            │
│  ├─ interview.mp3 (12:34)                          │
│  ├─ memo.m4a (2:15)                                │
│  └─ meeting.wav (45:02)                            │
│                                                     │
│  [Transcrire 3 fichiers]                           │
│                                                     │
│  ▓▓▓▓▓▓▓▓░░░░░░░░ 2/3 - memo.m4a                  │
│                                                     │
└─────────────────────────────────────────────────────┘
```

---

## Dépendances natives

| Moteur | Lib native | Stratégie |
|--------|-----------|-----------|
| Whisper | whisper.cpp | Compilé par `whisper-rs` (existant) |
| Parakeet | sherpa-onnx + onnxruntime | `sherpa-rs` télécharge automatiquement |
| Vosk | libvosk.dylib | Téléchargé par `build.rs` avec fix `@executable_path` |

---

## Gestion des erreurs

```rust
pub enum EngineError {
    ModelNotInstalled { engine: EngineType, model: String },
    DownloadFailed { url: String, reason: String },
    TranscriptionFailed { reason: String },
    UnsupportedAudioFormat { format: String },
    AudioDecodingFailed { path: String, reason: String },
    LanguageNotSupported { engine: EngineType, language: String },
}
```

### Fallback

- Moteur échoue → notification + option de basculer sur Whisper
- Fichier non décodable → skip + rapport d'erreur

### Timeout

- Transcription fichiers annulable
- Timeout: 10min/fichier par défaut

---

## Nouvelles dépendances Cargo.toml

```toml
# Moteurs STT
sherpa-rs = "0.1"
vosk = "0.2"

# Décodage audio multi-format
symphonia = { version = "0.5", features = ["mp3", "aac", "flac", "ogg", "vorbis"] }

# Resampling audio
rubato = "0.15"
```

---

## Fichiers à créer/modifier

### Créer

- `src-tauri/src/engines/parakeet.rs`
- `src-tauri/src/engines/vosk.rs`
- `src-tauri/src/audio/decoder.rs`
- `src-tauri/src/commands/file_transcription.rs`
- `src/components/FileTranscription.tsx`

### Modifier

- `src-tauri/Cargo.toml`
- `src-tauri/build.rs`
- `src-tauri/src/engines/mod.rs`
- `src-tauri/src/engines/model_manager.rs`
- `src-tauri/src/types.rs`
- `src-tauri/src/state.rs`
- `src-tauri/src/lib.rs`
- `src/components/SettingsPanel.tsx`

---

## Estimation

- ~1500 lignes Rust
- ~400 lignes TypeScript
