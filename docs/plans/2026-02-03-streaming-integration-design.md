# WakaScribe - Streaming Temps Réel et Intégration Système

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ajouter le streaming temps réel de la transcription et l'intégration système (coller auto, fenêtre flottante)

**Architecture:** Transcription par chunks + fenêtre flottante always-on-top + simulation clavier pour auto-paste

**Tech Stack:** Rust/Tauri, Whisper streaming, AppleScript (macOS), tauri multi-window

---

## 1. Streaming Temps Réel

### Principe
Whisper.cpp supporte le mode "streaming" via des segments de transcription incrémentaux. Au lieu d'attendre la fin de l'enregistrement, on transcrit par chunks de ~2-3 secondes.

### Flux technique
```
Audio buffer (continu) → Chunk 2-3s → Whisper transcription → Event Tauri → UI update
                              ↓
                     Accumulation du texte complet
```

### Nouveaux composants backend
- `src-tauri/src/audio/streaming.rs` - Buffer circulaire avec extraction de chunks
- Événement Tauri `transcription-chunk` émis toutes les 2-3 secondes
- Le texte final passe toujours par le pipeline LLM (après arrêt)

### Comportement
- Pendant l'enregistrement : texte brut affiché progressivement (sans LLM)
- À l'arrêt : texte complet envoyé au LLM pour post-traitement
- Le texte final remplace le texte streaming

### Latence estimée
- Chunk toutes les 2-3s
- Transcription Whisper tiny : ~200ms par chunk
- Affichage quasi temps réel

---

## 2. Fenêtre Flottante Adaptive

### Caractéristiques
- Toujours visible par-dessus les autres applications (`always_on_top`)
- Déplaçable par drag & drop
- Position sauvegardée entre les sessions

### État compact (par défaut ~300x40px)
```
┌─────────────────────────────────┐
│ ● PRÊT          [─] [×]        │
└─────────────────────────────────┘
```
- LED de statut + texte statut
- Boutons minimiser/fermer
- Click sur la barre = toggle enregistrement

### État étendu (pendant enregistrement ~400x150px)
```
┌─────────────────────────────────────────┐
│ 🔴 CAPTURE EN COURS        [─] [×]     │
├─────────────────────────────────────────┤
│                                         │
│ Bonjour, je dicte un texte en temps    │
│ réel et il s'affiche ici...            │
│                                         │
├─────────────────────────────────────────┤
│ Email · LLM                      2.3s  │
└─────────────────────────────────────────┘
```
- Texte streaming affiché en temps réel
- Mode actif + durée en footer

### Implémentation Tauri
- Nouvelle fenêtre avec `decorations: false`, `always_on_top: true`, `transparent: true`
- Communication avec fenêtre principale via événements Tauri
- Fichiers: `src/windows/floating.html`, `src/components/FloatingWindow.tsx`

---

## 3. Coller Automatique

### Mécanisme
Après la transcription (et post-traitement LLM si activé), le texte est automatiquement collé dans l'application active.

### Flux
```
Transcription terminée → Copie dans presse-papier → Simulation Cmd+V → Focus restauré
```

### Implémentation technique
- Utilise `tauri-plugin-clipboard-manager` (déjà présent) pour copier
- macOS: AppleScript via `tauri-plugin-shell`
  ```applescript
  tell application "System Events" to keystroke "v" using command down
  ```
- Windows: `SendInput` via l'API Win32

### Nouveau paramètre
```rust
auto_paste_to_active_app: bool  // true par défaut
```

### Gestion des cas particuliers
- Si l'app active est WakaScribe → ne pas coller (éviter boucle)
- Délai de 100ms avant le Cmd+V pour laisser le focus se stabiliser
- Option pour désactiver dans les paramètres

### Permissions macOS
- Nécessite "Accessibility" permission pour simuler les touches
- Prompt automatique au premier usage

---

## 4. Interface Utilisateur

### Nouveaux paramètres dans SettingsPanel
```
┌─ Intégration Système ──────────────────────────┐
│                                                 │
│  [Toggle] Coller automatiquement dans l'app    │
│           active après transcription           │
│                                                 │
│  [Toggle] Afficher la fenêtre flottante        │
│                                                 │
│  [Toggle] Streaming temps réel                 │
│           (afficher le texte pendant           │
│            l'enregistrement)                   │
│                                                 │
└─────────────────────────────────────────────────┘
```

### Modifications DictationPanel
- Zone de texte visible pendant l'enregistrement (streaming)
- Texte en italique/grisé pendant le streaming (indique "provisoire")
- Remplacé par texte final après post-traitement LLM

### Raccourci fenêtre flottante
- `Cmd+Shift+F` pour toggle la fenêtre flottante
- Aussi accessible via icône tray

### Indicateur dans footer App.tsx
```
Whisper.cpp · Email · LLM · Auto-paste
```

---

## 5. Types et paramètres

### Nouveaux champs AppSettings (Rust)
```rust
pub streaming_enabled: bool,        // true par défaut
pub auto_paste_enabled: bool,       // true par défaut
pub floating_window_enabled: bool,  // false par défaut
pub floating_window_position: Option<(i32, i32)>,  // sauvegarde position
```

### Nouveaux types TypeScript
```typescript
interface AppSettings {
  // ... existants ...
  streaming_enabled: boolean;
  auto_paste_enabled: boolean;
  floating_window_enabled: boolean;
}

interface StreamingChunk {
  text: string;
  is_final: boolean;
  duration_seconds: number;
}
```

---

## 6. Événements Tauri

| Événement | Payload | Description |
|-----------|---------|-------------|
| `transcription-chunk` | `StreamingChunk` | Nouveau chunk transcrit |
| `transcription-final` | `TranscriptionResult` | Transcription finale (après LLM) |
| `floating-window-toggle` | `bool` | Toggle fenêtre flottante |
| `recording-status` | `string` | Statut pour sync entre fenêtres |

---

## 7. Fichiers à créer/modifier

### Nouveaux fichiers
- `src-tauri/src/audio/streaming.rs` - Buffer circulaire et gestion chunks
- `src-tauri/src/commands/system_integration.rs` - Auto-paste, fenêtre flottante
- `src/components/FloatingWindow.tsx` - Composant fenêtre flottante
- `src/windows/floating.html` - Point d'entrée HTML fenêtre flottante
- `src/windows/floating.tsx` - Entry point React fenêtre flottante

### Fichiers à modifier
- `src-tauri/src/types.rs` - Nouveaux paramètres
- `src-tauri/src/storage/config.rs` - Valeurs par défaut
- `src-tauri/src/commands/transcription.rs` - Mode streaming
- `src-tauri/src/audio/mod.rs` - Export streaming
- `src-tauri/src/lib.rs` - Nouvelles commandes
- `src-tauri/tauri.conf.json` - Déclarer 2ème fenêtre
- `src/types/index.ts` - Nouveaux types TS
- `src/components/DictationPanel.tsx` - Affichage streaming
- `src/components/SettingsPanel.tsx` - Nouveaux toggles
- `src/App.tsx` - Footer dynamique
- `vite.config.ts` - Multi-page build
- `package.json` - Scripts build

---

## 8. Gestion des erreurs

| Erreur | Comportement |
|--------|--------------|
| Permission Accessibility refusée | Notification + désactive auto-paste |
| Fenêtre flottante fermée par erreur | Recréer au prochain toggle |
| Streaming chunk échoue | Ignorer, continuer avec prochain chunk |
| App active non détectable | Coller quand même (worst case = rien) |
| Whisper occupé par autre chunk | Queue les chunks, traiter séquentiellement |
