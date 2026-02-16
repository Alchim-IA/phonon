# Onboarding Wizard Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a 5-step first-launch wizard to WakaScribe that guides users through welcome, microphone permission, model selection/download, language choice, and keyboard shortcuts.

**Architecture:** State-driven conditional rendering in App.tsx. An `onboarding_completed` boolean flag in AppSettings (Rust + TypeScript) controls whether the wizard or main app is shown. The wizard is a set of React components in `src/components/onboarding/`.

**Tech Stack:** Rust/Tauri 2.x backend, React 19 + TypeScript + TailwindCSS frontend, Zustand store, existing Tauri commands for audio devices and model management.

---

### Task 1: Add `onboarding_completed` flag to Rust AppSettings

**Files:**
- Modify: `src-tauri/src/types.rs:308-351` (AppSettings struct)

**Step 1: Add the field to AppSettings struct**

In `src-tauri/src/types.rs`, add to the `AppSettings` struct after line 350 (`pub local_llm_model: LocalLlmModel,`):

```rust
    #[serde(default)]
    pub onboarding_completed: bool,
```

**Step 2: Add the field to Default impl**

In `src-tauri/src/types.rs`, add to the `Default for AppSettings` impl after line 399 (`local_llm_model: LocalLlmModel::default(),`):

```rust
            onboarding_completed: false,
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check 2>&1 | head -5`
Expected: no new errors (existing warnings OK)

**Step 4: Commit**

```bash
git add src-tauri/src/types.rs
git commit -m "feat(onboarding): add onboarding_completed flag to AppSettings"
```

---

### Task 2: Add `onboarding_completed` to TypeScript types and store

**Files:**
- Modify: `src/types/index.ts:57-85` (AppSettings interface)
- Modify: `src/stores/settingsStore.ts:19-47` (defaultSettings)

**Step 1: Add to TypeScript AppSettings interface**

In `src/types/index.ts`, add after line 84 (`hotkey_voice_action: string;`):

```typescript
  onboarding_completed: boolean;
```

**Step 2: Add to defaultSettings**

In `src/stores/settingsStore.ts`, add after line 46 (`hotkey_voice_action: 'Control+Alt+A',`):

```typescript
  onboarding_completed: false,
```

**Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | head -10`
Expected: no new errors

**Step 4: Commit**

```bash
git add src/types/index.ts src/stores/settingsStore.ts
git commit -m "feat(onboarding): add onboarding_completed to TS types and store"
```

---

### Task 3: Create OnboardingWizard container component

**Files:**
- Create: `src/components/onboarding/OnboardingWizard.tsx`
- Create: `src/components/onboarding/index.ts`

**Step 1: Create barrel export**

Create `src/components/onboarding/index.ts`:

```typescript
export { OnboardingWizard } from './OnboardingWizard';
```

**Step 2: Create OnboardingWizard.tsx**

Create `src/components/onboarding/OnboardingWizard.tsx`:

```tsx
import { useState } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';
import { WelcomeStep } from './WelcomeStep';
import { PermissionStep } from './PermissionStep';
import { ModelStep } from './ModelStep';
import { LanguageStep } from './LanguageStep';
import { ShortcutsStep } from './ShortcutsStep';

const STEPS = [
  { label: 'Bienvenue', component: WelcomeStep },
  { label: 'Microphone', component: PermissionStep },
  { label: 'Modele', component: ModelStep },
  { label: 'Langue', component: LanguageStep },
  { label: 'Raccourcis', component: ShortcutsStep },
];

export function OnboardingWizard() {
  const [currentStep, setCurrentStep] = useState(0);
  const { updateSettings } = useSettingsStore();
  const [stepValid, setStepValid] = useState(true);

  const StepComponent = STEPS[currentStep].component;

  const handleNext = () => {
    if (currentStep < STEPS.length - 1) {
      setCurrentStep(currentStep + 1);
      setStepValid(true);
    }
  };

  const handleBack = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
      setStepValid(true);
    }
  };

  const handleFinish = async () => {
    await updateSettings({ onboarding_completed: true });
  };

  const isLastStep = currentStep === STEPS.length - 1;

  return (
    <div className="h-screen flex flex-col overflow-hidden relative">
      <div className="mesh-gradient-bg" />
      <div className="noise-overlay" />

      <div className="relative z-10 h-full flex flex-col items-center justify-center p-6">
        {/* Stepper */}
        <div className="flex items-center gap-2 mb-8">
          {STEPS.map((step, index) => (
            <div key={step.label} className="flex items-center gap-2">
              <div className={`w-8 h-8 rounded-full flex items-center justify-center text-[0.75rem] font-medium border transition-all ${
                index === currentStep
                  ? 'bg-[var(--accent-primary)] border-[var(--accent-primary)] text-white'
                  : index < currentStep
                  ? 'bg-[var(--accent-success)] border-[var(--accent-success)] text-white'
                  : 'bg-[rgba(255,255,255,0.08)] border-[var(--glass-border)] text-[var(--text-muted)]'
              }`}>
                {index < currentStep ? (
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                ) : (
                  index + 1
                )}
              </div>
              <span className={`text-[0.7rem] hidden sm:inline ${
                index === currentStep ? 'text-[var(--text-primary)]' : 'text-[var(--text-muted)]'
              }`}>
                {step.label}
              </span>
              {index < STEPS.length - 1 && (
                <div className={`w-8 h-px ${
                  index < currentStep ? 'bg-[var(--accent-success)]' : 'bg-[var(--glass-border)]'
                }`} />
              )}
            </div>
          ))}
        </div>

        {/* Step content */}
        <div className="glass-panel w-full max-w-[700px] p-8 animate-fade-in">
          <StepComponent onValidChange={setStepValid} />
        </div>

        {/* Navigation buttons */}
        <div className="flex items-center gap-4 mt-6">
          {currentStep > 0 && (
            <button onClick={handleBack} className="btn-glass">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <path d="M19 12H5M12 19l-7-7 7-7" />
              </svg>
              Retour
            </button>
          )}
          <button
            onClick={isLastStep ? handleFinish : handleNext}
            disabled={!stepValid}
            className={`px-6 py-2.5 rounded-xl text-[0.85rem] font-medium transition-all ${
              stepValid
                ? 'bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] text-white hover:opacity-90'
                : 'bg-[rgba(255,255,255,0.08)] text-[var(--text-muted)] cursor-not-allowed'
            }`}
          >
            {isLastStep ? 'Terminer' : 'Suivant'}
            {!isLastStep && (
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="inline ml-2">
                <path d="M5 12h14M12 5l7 7-7 7" />
              </svg>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
```

**Step 3: Verify TypeScript compiles** (will fail until step components exist — expected)

**Step 4: Commit**

```bash
git add src/components/onboarding/
git commit -m "feat(onboarding): create OnboardingWizard container with stepper"
```

---

### Task 4: Create WelcomeStep component

**Files:**
- Create: `src/components/onboarding/WelcomeStep.tsx`

**Step 1: Create WelcomeStep.tsx**

```tsx
import logoSvg from '../../assets/logo.svg';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

export function WelcomeStep(_props: StepProps) {
  return (
    <div className="flex flex-col items-center text-center py-6">
      {/* Logo */}
      <div className="w-24 h-24 rounded-3xl bg-gradient-to-br from-[var(--accent-primary)] to-[var(--accent-secondary)] flex items-center justify-center shadow-lg mb-6 overflow-visible">
        <img src={logoSvg} alt="WakaScribe" className="w-96 h-96 invert" />
      </div>

      {/* Title */}
      <h1 className="font-display text-3xl tracking-tight mb-3">
        <span className="text-[var(--text-primary)]">Bienvenue sur </span>
        <span className="bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] bg-clip-text text-transparent">
          WakaScribe
        </span>
      </h1>

      <p className="text-[var(--text-secondary)] text-[0.95rem] mb-8 max-w-md">
        Dictee vocale locale, privee et rapide. Transformez votre voix en texte sans connexion Internet.
      </p>

      {/* Feature badges */}
      <div className="flex gap-4">
        <div className="glass-card px-5 py-3 flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-success)" strokeWidth="1.5">
            <rect width="18" height="11" x="3" y="11" rx="2" ry="2" />
            <path d="M7 11V7a5 5 0 0 1 10 0v4" />
          </svg>
          <span className="text-[0.8rem] text-[var(--text-primary)]">100% Offline</span>
        </div>
        <div className="glass-card px-5 py-3 flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" strokeWidth="1.5">
            <circle cx="12" cy="12" r="10" />
            <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
          </svg>
          <span className="text-[0.8rem] text-[var(--text-primary)]">Multi-langues</span>
        </div>
        <div className="glass-card px-5 py-3 flex items-center gap-3">
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-secondary)" strokeWidth="1.5">
            <path d="M12 2a4 4 0 0 1 4 4c0 1.95-1.4 3.58-3.25 3.93L12 22l-.75-12.07A4.001 4.001 0 0 1 12 2z" />
          </svg>
          <span className="text-[0.8rem] text-[var(--text-primary)]">IA integree</span>
        </div>
      </div>
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add src/components/onboarding/WelcomeStep.tsx
git commit -m "feat(onboarding): add WelcomeStep component"
```

---

### Task 5: Create PermissionStep component

**Files:**
- Create: `src/components/onboarding/PermissionStep.tsx`

**Step 1: Create PermissionStep.tsx**

This component calls `list_audio_devices` on mount which triggers the macOS microphone permission prompt. It shows the permission status and lets the user select a microphone if multiple are available.

```tsx
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AudioDevice } from '../../types';
import { useSettingsStore } from '../../stores/settingsStore';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

type PermissionStatus = 'checking' | 'granted' | 'denied';

export function PermissionStep({ onValidChange }: StepProps) {
  const [status, setStatus] = useState<PermissionStatus>('checking');
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const { updateSettings } = useSettingsStore();

  useEffect(() => {
    checkMicrophoneAccess();
  }, []);

  useEffect(() => {
    onValidChange(status === 'granted');
  }, [status, onValidChange]);

  const checkMicrophoneAccess = async () => {
    try {
      const deviceList = await invoke<AudioDevice[]>('list_audio_devices');
      if (deviceList.length > 0) {
        setDevices(deviceList);
        setStatus('granted');
        const defaultDevice = deviceList.find(d => d.is_default) || deviceList[0];
        setSelectedDevice(defaultDevice.id);
        await updateSettings({ microphone_id: defaultDevice.id });
      } else {
        setStatus('denied');
      }
    } catch {
      setStatus('denied');
    }
  };

  const handleDeviceChange = async (deviceId: string) => {
    setSelectedDevice(deviceId);
    await updateSettings({ microphone_id: deviceId });
  };

  return (
    <div className="flex flex-col items-center text-center py-6">
      {/* Microphone icon */}
      <div className={`w-20 h-20 rounded-full flex items-center justify-center mb-6 transition-all ${
        status === 'checking'
          ? 'bg-[rgba(255,255,255,0.08)]'
          : status === 'granted'
          ? 'bg-[rgba(122,239,178,0.15)]'
          : 'bg-[rgba(255,122,122,0.15)]'
      }`}>
        {status === 'checking' ? (
          <div className="w-8 h-8 border-2 border-[var(--text-muted)] border-t-[var(--accent-primary)] rounded-full animate-spin" />
        ) : status === 'granted' ? (
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="var(--accent-success)" strokeWidth="1.5">
            <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <line x1="12" x2="12" y1="19" y2="22" />
          </svg>
        ) : (
          <svg width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="var(--accent-danger)" strokeWidth="1.5">
            <line x1="2" y1="2" x2="22" y2="22" />
            <path d="M18.89 13.23A7.12 7.12 0 0 0 19 12v-2" />
            <path d="M5 10v2a7 7 0 0 0 12 5" />
            <path d="M15 9.34V5a3 3 0 0 0-5.68-1.33" />
            <path d="M9 9v3a3 3 0 0 0 5.12 2.12" />
            <line x1="12" x2="12" y1="19" y2="22" />
          </svg>
        )}
      </div>

      <h2 className="font-display text-xl text-[var(--text-primary)] mb-2">
        Acces au microphone
      </h2>

      {status === 'checking' && (
        <p className="text-[var(--text-secondary)] text-[0.85rem] mb-4">
          Verification de l'acces au microphone...
        </p>
      )}

      {status === 'granted' && (
        <>
          <p className="text-[var(--accent-success)] text-[0.85rem] mb-6">
            Microphone accessible
          </p>

          {devices.length > 1 && (
            <div className="w-full max-w-sm">
              <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block text-left">
                Choisir un microphone
              </label>
              <select
                value={selectedDevice || ''}
                onChange={(e) => handleDeviceChange(e.target.value)}
                className="w-full px-4 py-2.5 rounded-xl bg-[rgba(255,255,255,0.08)] border border-[var(--glass-border)] text-[var(--text-primary)] text-[0.85rem] focus:outline-none focus:border-[var(--accent-primary)]"
              >
                {devices.map(device => (
                  <option key={device.id} value={device.id}>
                    {device.name} {device.is_default ? '(Par defaut)' : ''}
                  </option>
                ))}
              </select>
            </div>
          )}
        </>
      )}

      {status === 'denied' && (
        <>
          <p className="text-[var(--accent-danger)] text-[0.85rem] mb-4">
            L'acces au microphone est requis pour la dictee vocale.
          </p>
          <p className="text-[var(--text-muted)] text-[0.75rem] mb-4">
            Autorisez WakaScribe dans Reglages Systeme &gt; Confidentialite et securite &gt; Microphone
          </p>
          <button
            onClick={checkMicrophoneAccess}
            className="btn-glass"
          >
            Reessayer
          </button>
        </>
      )}
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add src/components/onboarding/PermissionStep.tsx
git commit -m "feat(onboarding): add PermissionStep with auto mic permission trigger"
```

---

### Task 6: Create ModelStep component

**Files:**
- Create: `src/components/onboarding/ModelStep.tsx`

**Step 1: Create ModelStep.tsx**

Reuses existing Tauri commands `get_available_models`, `download_model` and events `model-download-progress`, `model-download-complete`.

```tsx
import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { ModelInfo, ModelSize, DownloadProgress } from '../../types';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

export function ModelStep({ onValidChange }: StepProps) {
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [downloading, setDownloading] = useState<ModelSize | null>(null);
  const [downloadProgress, setDownloadProgress] = useState<DownloadProgress | null>(null);
  const [downloadComplete, setDownloadComplete] = useState(false);

  useEffect(() => {
    loadModels();
  }, []);

  useEffect(() => {
    onValidChange(downloading === null);
  }, [downloading, onValidChange]);

  useEffect(() => {
    const unlistenProgress = listen<DownloadProgress>('model-download-progress', (event) => {
      setDownloadProgress(event.payload);
    });

    const unlistenComplete = listen<ModelSize>('model-download-complete', () => {
      setDownloading(null);
      setDownloadProgress(null);
      setDownloadComplete(true);
      loadModels();
    });

    return () => {
      unlistenProgress.then(fn => fn());
      unlistenComplete.then(fn => fn());
    };
  }, []);

  const loadModels = async () => {
    try {
      const result = await invoke<ModelInfo[]>('get_available_models');
      setModels(result);
    } catch (e) {
      console.error('Failed to load models:', e);
    }
  };

  const handleDownload = async (size: ModelSize) => {
    setDownloading(size);
    setDownloadProgress({ downloaded: 0, total: 1, percent: 0 });
    setDownloadComplete(false);
    try {
      await invoke('download_model', { size });
    } catch (e) {
      console.error('Download failed:', e);
      setDownloading(null);
      setDownloadProgress(null);
    }
  };

  const qualityLabels: Record<ModelSize, string> = {
    tiny: 'Basique',
    small: 'Bonne',
    medium: 'Tres bonne',
  };

  const qualityColors: Record<ModelSize, string> = {
    tiny: 'var(--text-muted)',
    small: 'var(--accent-primary)',
    medium: 'var(--accent-success)',
  };

  return (
    <div className="py-4">
      <div className="text-center mb-6">
        <h2 className="font-display text-xl text-[var(--text-primary)] mb-2">
          Modele de reconnaissance vocale
        </h2>
        <p className="text-[var(--text-secondary)] text-[0.85rem]">
          Choisissez la qualite de transcription. Le modele Tiny est deja inclus.
        </p>
      </div>

      <div className="space-y-3">
        {models.map((model) => (
          <div
            key={model.size}
            className={`glass-card p-5 transition-all ${
              model.available && model.size !== 'tiny' ? 'border-[var(--accent-success)]' : ''
            }`}
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-4">
                <div className={`w-10 h-10 rounded-xl flex items-center justify-center ${
                  model.size === 'tiny'
                    ? 'bg-[rgba(255,255,255,0.08)]'
                    : model.size === 'small'
                    ? 'bg-[rgba(124,138,255,0.15)]'
                    : 'bg-[rgba(122,239,178,0.15)]'
                }`}>
                  <span className="text-[0.85rem] font-medium" style={{ color: qualityColors[model.size] }}>
                    {model.size === 'tiny' ? 'T' : model.size === 'small' ? 'S' : 'M'}
                  </span>
                </div>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="text-[0.95rem] text-[var(--text-primary)] font-medium">
                      {model.display_name}
                    </span>
                    {model.size === 'tiny' && (
                      <span className="tag-frost text-[0.6rem]">Inclus</span>
                    )}
                    {model.size === 'small' && (
                      <span className="text-[0.65rem] text-[var(--accent-primary)]">Recommande</span>
                    )}
                  </div>
                  <span className="text-[0.75rem]" style={{ color: qualityColors[model.size] }}>
                    Qualite: {qualityLabels[model.size]}
                  </span>
                </div>
              </div>

              {downloading === model.size ? (
                <div className="flex items-center gap-3">
                  <div className="w-28 progress-frost">
                    <div className="bar" style={{ width: `${downloadProgress?.percent || 0}%` }} />
                  </div>
                  <span className="text-[0.75rem] text-[var(--text-muted)] w-12 text-right tabular-nums">
                    {Math.round(downloadProgress?.percent || 0)}%
                  </span>
                </div>
              ) : model.available ? (
                <span className="tag-frost success">Installe</span>
              ) : (
                <button
                  onClick={() => handleDownload(model.size)}
                  disabled={downloading !== null}
                  className="btn-glass text-[0.8rem]"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                    <polyline points="7 10 12 15 17 10" />
                    <line x1="12" y1="15" x2="12" y2="3" />
                  </svg>
                  Telecharger
                </button>
              )}
            </div>
          </div>
        ))}
      </div>

      {downloadComplete && (
        <p className="text-center text-[var(--accent-success)] text-[0.8rem] mt-4">
          Modele telecharge avec succes !
        </p>
      )}
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add src/components/onboarding/ModelStep.tsx
git commit -m "feat(onboarding): add ModelStep with in-wizard download"
```

---

### Task 7: Create LanguageStep component

**Files:**
- Create: `src/components/onboarding/LanguageStep.tsx`

**Step 1: Create LanguageStep.tsx**

```tsx
import { useEffect, useState } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

const LANGUAGES = [
  { code: 'fr', label: 'Francais', flag: '🇫🇷' },
  { code: 'en', label: 'English', flag: '🇬🇧' },
  { code: 'es', label: 'Espanol', flag: '🇪🇸' },
  { code: 'de', label: 'Deutsch', flag: '🇩🇪' },
  { code: 'it', label: 'Italiano', flag: '🇮🇹' },
  { code: 'pt', label: 'Portugues', flag: '🇵🇹' },
  { code: 'ja', label: '日本語', flag: '🇯🇵' },
  { code: 'zh', label: '中文', flag: '🇨🇳' },
  { code: 'ko', label: '한국어', flag: '🇰🇷' },
  { code: 'ru', label: 'Русский', flag: '🇷🇺' },
  { code: 'nl', label: 'Nederlands', flag: '🇳🇱' },
  { code: 'pl', label: 'Polski', flag: '🇵🇱' },
];

export function LanguageStep({ onValidChange }: StepProps) {
  const { settings, updateSettings } = useSettingsStore();
  const [selectedLanguage, setSelectedLanguage] = useState(settings?.transcription_language || 'fr');
  const [autoDetect, setAutoDetect] = useState(settings?.auto_detect_language || false);

  useEffect(() => {
    onValidChange(true);
  }, [onValidChange]);

  const handleLanguageSelect = async (code: string) => {
    setSelectedLanguage(code);
    await updateSettings({ transcription_language: code });
  };

  const handleAutoDetectToggle = async () => {
    const newValue = !autoDetect;
    setAutoDetect(newValue);
    await updateSettings({ auto_detect_language: newValue });
  };

  return (
    <div className="py-4">
      <div className="text-center mb-6">
        <h2 className="font-display text-xl text-[var(--text-primary)] mb-2">
          Langue de transcription
        </h2>
        <p className="text-[var(--text-secondary)] text-[0.85rem]">
          Choisissez la langue principale pour la reconnaissance vocale.
        </p>
      </div>

      {/* Auto-detect toggle */}
      <div className="glass-card p-4 mb-4 flex items-center justify-between">
        <div>
          <span className="text-[0.85rem] text-[var(--text-primary)]">Detection automatique</span>
          <p className="text-[0.7rem] text-[var(--text-muted)]">Detecte la langue automatiquement (peut ralentir)</p>
        </div>
        <button
          onClick={handleAutoDetectToggle}
          className={`w-11 h-6 rounded-full transition-all relative ${
            autoDetect ? 'bg-[var(--accent-primary)]' : 'bg-[rgba(255,255,255,0.15)]'
          }`}
        >
          <div className={`w-5 h-5 rounded-full bg-white absolute top-0.5 transition-all ${
            autoDetect ? 'left-[22px]' : 'left-0.5'
          }`} />
        </button>
      </div>

      {/* Language grid */}
      <div className={`grid grid-cols-3 gap-2 transition-opacity ${autoDetect ? 'opacity-40 pointer-events-none' : ''}`}>
        {LANGUAGES.map((lang) => (
          <button
            key={lang.code}
            onClick={() => handleLanguageSelect(lang.code)}
            className={`glass-card p-3 text-left transition-all ${
              selectedLanguage === lang.code
                ? 'border-[var(--accent-primary)] bg-[rgba(124,138,255,0.1)]'
                : 'hover:border-[var(--accent-primary)]'
            }`}
          >
            <span className="text-lg mr-2">{lang.flag}</span>
            <span className={`text-[0.8rem] ${
              selectedLanguage === lang.code ? 'text-[var(--text-primary)]' : 'text-[var(--text-secondary)]'
            }`}>
              {lang.label}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add src/components/onboarding/LanguageStep.tsx
git commit -m "feat(onboarding): add LanguageStep with language grid and auto-detect"
```

---

### Task 8: Create ShortcutsStep component

**Files:**
- Create: `src/components/onboarding/ShortcutsStep.tsx`

**Step 1: Create ShortcutsStep.tsx**

Reuses the existing `HotkeyInput` component from `src/components/HotkeyInput.tsx`.

```tsx
import { useEffect, useState } from 'react';
import { useSettingsStore } from '../../stores/settingsStore';
import { HotkeyInput } from '../HotkeyInput';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

function formatHotkey(hotkey: string): string {
  return hotkey
    .replace('CommandOrControl', '⌘')
    .replace('Command', '⌘')
    .replace('Control', 'Ctrl')
    .replace('Shift', '⇧')
    .replace('Alt', '⌥')
    .replace('Space', 'Espace')
    .replace(/\+/g, ' + ');
}

export function ShortcutsStep({ onValidChange }: StepProps) {
  const { settings, updateSettings } = useSettingsStore();
  const [editingPtt, setEditingPtt] = useState(false);
  const [editingToggle, setEditingToggle] = useState(false);

  useEffect(() => {
    onValidChange(true);
  }, [onValidChange]);

  return (
    <div className="py-4">
      <div className="text-center mb-6">
        <h2 className="font-display text-xl text-[var(--text-primary)] mb-2">
          Raccourcis clavier
        </h2>
        <p className="text-[var(--text-secondary)] text-[0.85rem]">
          Configurez les raccourcis pour controler la dictee vocale.
        </p>
      </div>

      <div className="space-y-4">
        {/* Push-to-talk */}
        <div className="glass-card p-5">
          <div className="flex items-center justify-between mb-2">
            <div>
              <span className="text-[0.9rem] text-[var(--text-primary)] font-medium">Push-to-talk</span>
              <p className="text-[0.7rem] text-[var(--text-muted)]">Maintenir pour dicter, relacher pour transcrire</p>
            </div>
            {!editingPtt && (
              <div className="flex items-center gap-3">
                <span className="kbd-frost">{formatHotkey(settings?.hotkey_push_to_talk || 'Control+Space')}</span>
                <button
                  onClick={() => setEditingPtt(true)}
                  className="text-[0.75rem] text-[var(--accent-primary)] hover:underline"
                >
                  Modifier
                </button>
              </div>
            )}
          </div>
          {editingPtt && (
            <div className="mt-3">
              <HotkeyInput
                value={settings?.hotkey_push_to_talk || 'Control+Space'}
                onChange={async (hotkey) => {
                  await updateSettings({ hotkey_push_to_talk: hotkey });
                  setEditingPtt(false);
                }}
              />
            </div>
          )}
        </div>

        {/* Toggle record */}
        <div className="glass-card p-5">
          <div className="flex items-center justify-between mb-2">
            <div>
              <span className="text-[0.9rem] text-[var(--text-primary)] font-medium">Toggle enregistrement</span>
              <p className="text-[0.7rem] text-[var(--text-muted)]">Appuyer pour demarrer/arreter l'enregistrement</p>
            </div>
            {!editingToggle && (
              <div className="flex items-center gap-3">
                <span className="kbd-frost">{formatHotkey(settings?.hotkey_toggle_record || 'Control+Shift+R')}</span>
                <button
                  onClick={() => setEditingToggle(true)}
                  className="text-[0.75rem] text-[var(--accent-primary)] hover:underline"
                >
                  Modifier
                </button>
              </div>
            )}
          </div>
          {editingToggle && (
            <div className="mt-3">
              <HotkeyInput
                value={settings?.hotkey_toggle_record || 'Control+Shift+R'}
                onChange={async (hotkey) => {
                  await updateSettings({ hotkey_toggle_record: hotkey });
                  setEditingToggle(false);
                }}
              />
            </div>
          )}
        </div>
      </div>

      <p className="text-center text-[var(--text-muted)] text-[0.75rem] mt-6">
        Vous pourrez modifier tous les raccourcis dans les parametres.
      </p>
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add src/components/onboarding/ShortcutsStep.tsx
git commit -m "feat(onboarding): add ShortcutsStep with hotkey display and editing"
```

---

### Task 9: Wire OnboardingWizard into App.tsx

**Files:**
- Modify: `src/App.tsx:1-9` (imports), `src/App.tsx:106-318` (render)

**Step 1: Add import**

In `src/App.tsx`, add after line 8 (`import { useTranscriptionStore } from './stores/transcriptionStore';`):

```typescript
import { OnboardingWizard } from './components/onboarding';
```

**Step 2: Add conditional rendering**

In `src/App.tsx`, replace the return statement (line 106-318) to wrap the main content:

Replace the opening `return (` block. After `const { settings, loadSettings } = useSettingsStore();` (line 34), the settings might still be loading. Handle this:

In the return block, right after `<div className="h-screen flex flex-col overflow-hidden relative">` (line 107), add a check: if settings are loaded and `onboarding_completed` is false, render the wizard instead.

The simplest approach: wrap the entire return in a condition. Replace lines 106-318 with:

```tsx
  if (settings && !settings.onboarding_completed) {
    return <OnboardingWizard />;
  }

  return (
    // ... existing JSX unchanged ...
  );
```

**Step 3: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | head -10`
Expected: no new errors

**Step 4: Commit**

```bash
git add src/App.tsx
git commit -m "feat(onboarding): wire wizard into App.tsx with conditional rendering"
```

---

### Task 10: Final verification

**Step 1: Verify Rust compiles**

Run: `cd src-tauri && cargo check 2>&1 | tail -5`
Expected: no new errors

**Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit 2>&1 | head -10`
Expected: no new errors

**Step 3: Commit all together if any loose changes**

```bash
git add -A
git commit -m "feat(onboarding): complete first-launch wizard implementation"
```
