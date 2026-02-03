# Migration vers Whisper.cpp

**Date:** 2026-02-03
**Statut:** Approuvé

## Contexte

Remplacer le moteur OpenVINO/Parakeet par Whisper.cpp pour:
- Simplicité d'intégration (un seul binaire vs 4 modèles)
- Compatibilité cross-platform (CPU/GPU universel)
- Meilleure qualité de transcription
- Support multilangue (99 langues)

## Architecture

### Changements majeurs

```
AVANT (Parakeet/OpenVINO)          APRÈS (Whisper.cpp)
─────────────────────────          ────────────────────
4 modèles OpenVINO (.xml/.bin)  →  1 fichier .bin (ggml)
Pipeline TDT complexe           →  API whisper-rs simple
Intel uniquement optimisé       →  Cross-platform (CPU/GPU)
Anglais seulement               →  99 langues supportées
~500 MB de modèles              →  75 MB bundlé + 466 MB optionnel
```

### Dépendances Rust

```toml
# Cargo.toml - À ajouter
whisper-rs = "0.14"

# À supprimer
openvino = { version = "..." }
```

### Structure des fichiers

```
src-tauri/src/engines/
├── mod.rs           # Simplifié
├── traits.rs        # Inchangé (SpeechEngine trait)
├── error.rs         # Adapté pour Whisper
├── whisper.rs       # NOUVEAU - Remplace openvino.rs
└── model_manager.rs # NOUVEAU - Téléchargement des modèles
```

## Implémentation du moteur Whisper

### Structure WhisperEngine

```rust
// src-tauri/src/engines/whisper.rs

pub struct WhisperEngine {
    ctx: WhisperContext,           // Contexte whisper-rs (thread-safe)
    language: Option<String>,      // None = auto-détection, Some("fr") = forcé
    model_size: ModelSize,         // Tiny, Small, Medium, etc.
}

pub enum ModelSize {
    Tiny,    // 75 MB - bundlé par défaut
    Small,   // 466 MB - téléchargeable
    Medium,  // 1.5 GB - téléchargeable
}

impl SpeechEngine for WhisperEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        // 1. Resampler à 16kHz si nécessaire
        // 2. Créer WhisperParams avec langue (auto ou forcée)
        // 3. Appeler ctx.full() pour la transcription
        // 4. Récupérer le texte et la langue détectée
        // 5. Retourner TranscriptionResult
    }

    fn name(&self) -> &str {
        "Whisper"
    }
}
```

## Gestionnaire de modèles

### Structure ModelManager

```rust
// src-tauri/src/engines/model_manager.rs

pub struct ModelManager {
    models_dir: PathBuf,  // ~/.local/share/wakascribe/models/
}

impl ModelManager {
    /// Retourne le chemin du modèle, le télécharge si nécessaire
    pub async fn ensure_model(&self, size: ModelSize) -> Result<PathBuf, String>;

    /// Liste les modèles déjà téléchargés
    pub fn available_models(&self) -> Vec<ModelSize>;

    /// Supprime un modèle pour libérer de l'espace
    pub fn delete_model(&self, size: ModelSize) -> Result<(), String>;

    /// Taille du téléchargement restant
    pub fn download_size(&self, size: ModelSize) -> u64;
}
```

### Source des modèles

```
https://huggingface.co/ggerganov/whisper.cpp/resolve/main/
├── ggml-tiny.bin     (75 MB)   ← Bundlé dans l'app
├── ggml-small.bin    (466 MB)  ← Téléchargement optionnel
└── ggml-medium.bin   (1.5 GB)  ← Téléchargement optionnel
```

### Stockage des modèles

- **macOS**: `~/Library/Application Support/com.wakascribe.app/models/`
- **Windows**: `%APPDATA%\WakaScribe\models\`
- **Modèle tiny**: Copié depuis les ressources bundlées au premier lancement

### Commandes Tauri

```rust
#[tauri::command]
async fn download_model(size: String) -> Result<(), String>;

#[tauri::command]
fn get_available_models() -> Vec<ModelInfo>;

#[tauri::command]
fn get_current_model() -> String;
```

## Modifications Frontend

### Nouveaux champs Settings

```typescript
interface AppSettings {
    // ... existants ...
    whisperModel: 'tiny' | 'small' | 'medium';
    whisperLanguage: 'auto' | 'fr' | 'en' | 'de' | 'es' | ...;
}
```

### UI SettingsPanel

```
┌─────────────────────────────────────────────┐
│  Moteur de transcription                    │
├─────────────────────────────────────────────┤
│  Modèle Whisper                             │
│  ┌─────────────────────────────────────┐    │
│  │ ● Tiny (75 MB) - Rapide       ✓    │    │
│  │ ○ Small (466 MB) - Recommandé  ↓   │    │
│  │ ○ Medium (1.5 GB) - Précis    ↓   │    │
│  └─────────────────────────────────────┘    │
│  ↓ = Téléchargement requis                  │
│                                             │
│  Langue                                     │
│  ┌─────────────────────────────────────┐    │
│  │ Automatique (détection)        ▼   │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

## Nettoyage

### Fichiers à supprimer

```
src-tauri/src/engines/
├── openvino.rs      ❌
└── vocabulary.rs    ❌

src-tauri/resources/
├── openvino.zip     ❌
└── models/
    ├── parakeet_*.xml        ❌ (4 fichiers)
    ├── parakeet_*.bin        ❌ (4 fichiers)
    └── parakeet_v3_vocab.json ❌
```

### Ressources à ajouter

```
src-tauri/resources/
└── models/
    └── ggml-tiny.bin  ✅ (75 MB)
```

## Plan d'implémentation

1. **Ajouter whisper-rs** - Modifier `Cargo.toml`, vérifier la compilation
2. **Créer `WhisperEngine`** - Implémenter le nouveau moteur avec le trait `SpeechEngine`
3. **Créer `ModelManager`** - Gestion du téléchargement et stockage des modèles
4. **Modifier `AppState`** - Remplacer `Arc<OpenVINOEngine>` par `Arc<WhisperEngine>`
5. **Ajouter les commandes Tauri** - `download_model`, `get_available_models`, etc.
6. **Modifier les settings backend** - Nouveaux champs `whisperModel`, `whisperLanguage`
7. **Modifier le frontend** - `SettingsPanel.tsx` avec les nouvelles options
8. **Bundler le modèle tiny** - Ajouter `ggml-tiny.bin` aux ressources
9. **Supprimer le code OpenVINO** - Retirer fichiers, dépendances et ressources
10. **Mettre à jour la documentation** - `CLAUDE.md` et supprimer l'ancienne doc

## Estimation

- ~400 lignes de nouveau code Rust
- ~100 lignes de modifications TypeScript
- Suppression de ~600 lignes de code obsolète
