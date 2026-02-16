# Design: Wizard — Sélection micro avec waveform + Test de transcription

**Date:** 2026-02-16
**Statut:** Approuvé

## Contexte

Le wizard d'onboarding actuel (5 étapes) permet de sélectionner un micro mais sans feedback visuel. Pas d'étape de test de transcription.

## Modifications

### 1. Étape Micro améliorée (PermissionStep)

- Dropdown de sélection du micro (existant), mis en avant
- Micro par défaut pré-sélectionné
- **Waveform temps réel** : barres verticales animées réagissant au volume micro

**Backend (Rust) :**
- `start_mic_preview(device_id: Option<String>)` — thread dédié qui capture l'audio et émet des events `mic-level` toutes les ~50ms avec un Vec<f32> de ~32 amplitudes normalisées (0.0–1.0)
- `stop_mic_preview()` — arrête le thread de preview
- Réutilise `AudioCapture` existant + calcul RMS par bandes

**Frontend (React) :**
- Composant `<AudioWaveform />` — canvas avec barres verticales réactives
- Se lance dès que la permission micro est accordée
- Se stoppe quand on quitte l'étape
- Changement de micro → restart preview

### 2. Nouvelle étape "Test" (étape 6, après Shortcuts)

**Flow :**
1. Affiche une phrase exemple localisée (FR: "Le soleil brille aujourd'hui", EN: "The sun is shining today", etc.)
2. Bouton "Enregistrer" → countdown 3-2-1 → enregistrement ~5s avec waveform
3. Transcription automatique via moteur sélectionné
4. Affichage côte à côte : phrase attendue vs résultat
5. Indicateur de succès ou bouton "Réessayer"
6. Bouton "Terminer" pour finaliser le wizard

**Backend :** Réutilise `start_recording` / `stop_recording` existants.

### Approche technique

Events Tauri pour le preview audio (pas de Web Audio API) :
- Cohérent avec l'architecture existante
- Pas de double permission
- Fonctionne avec n'importe quel device cpal
