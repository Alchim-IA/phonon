# Onboarding Wizard - Design Document

**Date:** 2026-02-13
**Statut:** Validé

## Objectif

Ajouter un wizard de premier lancement (onboarding) à WakaScribe qui guide l'utilisateur à travers la configuration initiale : présentation, permissions, modèle STT, langue et raccourcis.

## Architecture

**Approche :** State-driven dans App.tsx (pas de routeur)

- Flag `onboarding_completed: boolean` dans `AppSettings` (défaut `false`)
- Persisté dans `config.json` (backend Rust + frontend Zustand)
- Rendu conditionnel dans `App.tsx` :
  - `false` → `<OnboardingWizard />`
  - `true` → contenu actuel (tabs + panels)

## Structure des fichiers

```
src/components/onboarding/
├── OnboardingWizard.tsx    # Conteneur + stepper + navigation
├── WelcomeStep.tsx         # Étape 1 : Bienvenue
├── PermissionStep.tsx      # Étape 2 : Permission micro
├── ModelStep.tsx           # Étape 3 : Choix & téléchargement modèle
├── LanguageStep.tsx        # Étape 4 : Langue de transcription
├── ShortcutsStep.tsx       # Étape 5 : Raccourcis clavier
└── index.ts
```

## Les 5 étapes

### Étape 1 — Bienvenue

- Logo WakaScribe + titre animé
- Description : "Dictée vocale locale, privée, rapide"
- 3 badges features : "100% Offline", "Multi-langues", "IA intégrée"
- Bouton "Commencer"

### Étape 2 — Permission Microphone

- Icône micro + explication
- Au montage : appel `list_audio_devices()` déclenche le prompt macOS
- Statut : spinner → check vert (accordé) ou warning rouge (refusé)
- Si refusé : message + lien Préférences Système
- Sélection du micro si plusieurs dispositifs

### Étape 3 — Choix du modèle STT

- 3 cartes : Tiny (installé), Small (466 MB), Medium (1.5 GB)
- Infos : taille, qualité, temps estimé
- Tiny pré-sélectionné avec badge "Inclus"
- Download in-wizard avec barre de progression (`model-download-progress`)
- Bouton "Passer" pour garder Tiny
- Utilise les commandes existantes `download_model`

### Étape 4 — Langue de transcription

- Grille de langues populaires (FR, EN, ES, DE, IT, PT, JA, ZH...)
- Toggle "Détection auto"
- FR pré-sélectionné

### Étape 5 — Raccourcis clavier

- Affiche Push-to-talk et Toggle record
- Touches stylisées (`.kbd-frost`)
- Bouton "Modifier" → champ de capture de raccourci
- Raccourcis additionnels non montrés ici

## Navigation & Stepper

- Stepper horizontal : 5 pastilles numérotées + labels
- Pastille courante en `--accent-primary`
- Boutons : "Retour" (gauche) + "Suivant" / "Terminer" (droite)
- Transition fade entre étapes
- "Suivant" désactivé si étape non validée

## Style visuel

- Full-screen, background `mesh-gradient-bg` + `noise-overlay`
- Contenu centré dans `glass-panel` (max-width ~700px)
- CSS variables existantes (`--glass-bg`, `--glass-border`, `--accent-primary`)
- Fonts Outfit (titres) + DM Sans (body)

## Modifications backend

- Ajouter `onboarding_completed: bool` à `AppSettings` (Rust)
- Défaut : `false`
- Mis à `true` à la fin du wizard via `update_settings`
