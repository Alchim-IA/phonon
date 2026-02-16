# Wizard — Mic Waveform + Transcription Test — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add real-time audio waveform visualization to the mic selection step, and a new transcription test step at the end of the onboarding wizard.

**Architecture:** Two new Tauri commands (`start_mic_preview`/`stop_mic_preview`) emit `mic-level` events with amplitude data from a dedicated preview thread. A new `<AudioWaveform>` React component renders bars on a canvas. A new `TestStep` component reuses `start_recording`/`stop_recording` for a phrase-repeat transcription test.

**Tech Stack:** Rust/cpal (audio capture), Tauri events, React canvas, existing transcription pipeline.

---

### Task 1: Add `start_mic_preview` / `stop_mic_preview` Tauri commands

**Files:**
- Modify: `src-tauri/src/commands/audio.rs`
- Modify: `src-tauri/src/lib.rs` (register commands in `invoke_handler`)

**Step 1: Add preview commands to `src-tauri/src/commands/audio.rs`**

Replace the entire file with:

```rust
use crate::audio::AudioCapture;
use crate::types::AudioDevice;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use serde::Serialize;

static MIC_PREVIEW_ACTIVE: AtomicBool = AtomicBool::new(false);
static MIC_PREVIEW_DEVICE: Mutex<Option<String>> = Mutex::new(None);

#[derive(Clone, Serialize)]
struct MicLevelEvent {
    levels: Vec<f32>,
}

#[tauri::command]
pub fn list_audio_devices() -> Result<Vec<AudioDevice>, String> {
    AudioCapture::list_devices()
}

#[tauri::command]
pub fn start_mic_preview(app: AppHandle, device_id: Option<String>) -> Result<(), String> {
    // Stop any existing preview
    MIC_PREVIEW_ACTIVE.store(false, Ordering::SeqCst);
    std::thread::sleep(std::time::Duration::from_millis(100));

    MIC_PREVIEW_ACTIVE.store(true, Ordering::SeqCst);
    if let Ok(mut guard) = MIC_PREVIEW_DEVICE.lock() {
        *guard = device_id.clone();
    }

    std::thread::spawn(move || {
        let mut capture = match AudioCapture::new(device_id.as_deref()) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Mic preview: failed to create capture: {}", e);
                return;
            }
        };

        if let Err(e) = capture.start(device_id.as_deref()) {
            log::error!("Mic preview: failed to start capture: {}", e);
            return;
        }

        log::info!("Mic preview started");
        let num_bars: usize = 32;

        while MIC_PREVIEW_ACTIVE.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(50));

            let (buffer, _sample_rate) = capture.get_audio_snapshot();
            let len = buffer.len();

            // Take only the last ~2400 samples (~50ms at 48kHz)
            let recent_start = len.saturating_sub(2400);
            let recent = &buffer[recent_start..];

            let mut levels = Vec::with_capacity(num_bars);
            if recent.is_empty() {
                levels.resize(num_bars, 0.0);
            } else {
                let chunk_size = recent.len() / num_bars;
                if chunk_size == 0 {
                    levels.resize(num_bars, 0.0);
                } else {
                    for i in 0..num_bars {
                        let start = i * chunk_size;
                        let end = (start + chunk_size).min(recent.len());
                        let rms: f32 = (recent[start..end]
                            .iter()
                            .map(|s| s * s)
                            .sum::<f32>()
                            / (end - start) as f32)
                            .sqrt();
                        // Normalize: typical mic RMS is 0.0-0.3, amplify for visual
                        levels.push((rms * 5.0).min(1.0));
                    }
                }
            }

            let _ = app.emit("mic-level", MicLevelEvent { levels });
        }

        // Cleanup
        let _ = capture.stop();
        log::info!("Mic preview stopped");
    });

    Ok(())
}

#[tauri::command]
pub fn stop_mic_preview() -> Result<(), String> {
    MIC_PREVIEW_ACTIVE.store(false, Ordering::SeqCst);
    Ok(())
}
```

**Step 2: Register commands in `src-tauri/src/lib.rs`**

In `invoke_handler`, add after `commands::list_audio_devices,`:
```rust
commands::start_mic_preview,
commands::stop_mic_preview,
```

**Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles with no new errors (pre-existing warnings OK)

**Step 4: Commit**

```bash
git add src-tauri/src/commands/audio.rs src-tauri/src/lib.rs
git commit -m "feat(audio): add start_mic_preview/stop_mic_preview commands with mic-level events"
```

---

### Task 2: Create `<AudioWaveform>` React component

**Files:**
- Create: `src/components/AudioWaveform.tsx`

**Step 1: Create the component**

```tsx
import { useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

interface AudioWaveformProps {
  width?: number;
  height?: number;
  barCount?: number;
  barColor?: string;
  active?: boolean;
}

export function AudioWaveform({
  width = 320,
  height = 80,
  barCount = 32,
  barColor = 'var(--accent-primary)',
  active = true,
}: AudioWaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const levelsRef = useRef<number[]>(new Array(barCount).fill(0));
  const animFrameRef = useRef<number>(0);

  useEffect(() => {
    if (!active) return;

    const unlisten = listen<{ levels: number[] }>('mic-level', (event) => {
      levelsRef.current = event.payload.levels;
    });

    const draw = () => {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;

      const dpr = window.devicePixelRatio || 1;
      canvas.width = width * dpr;
      canvas.height = height * dpr;
      ctx.scale(dpr, dpr);

      ctx.clearRect(0, 0, width, height);

      const levels = levelsRef.current;
      const gap = 2;
      const barWidth = (width - gap * (barCount - 1)) / barCount;
      const minBarHeight = 2;

      for (let i = 0; i < barCount; i++) {
        const level = levels[i] || 0;
        const barHeight = Math.max(minBarHeight, level * height * 0.9);
        const x = i * (barWidth + gap);
        const y = (height - barHeight) / 2;

        ctx.fillStyle = barColor;
        ctx.globalAlpha = 0.4 + level * 0.6;
        ctx.beginPath();
        ctx.roundRect(x, y, barWidth, barHeight, 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;

      animFrameRef.current = requestAnimationFrame(draw);
    };

    animFrameRef.current = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(animFrameRef.current);
      unlisten.then((fn) => fn());
    };
  }, [active, width, height, barCount, barColor]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width, height }}
      className="rounded-lg"
    />
  );
}
```

**Step 2: Verify TypeScript**

Run: `npx tsc --noEmit`
Expected: no new errors

**Step 3: Commit**

```bash
git add src/components/AudioWaveform.tsx
git commit -m "feat(ui): add AudioWaveform canvas component for real-time mic visualization"
```

---

### Task 3: Enhance PermissionStep with waveform and always-visible mic selector

**Files:**
- Modify: `src/components/onboarding/PermissionStep.tsx`

**Step 1: Rewrite PermissionStep**

Replace the full file content with:

```tsx
import { useEffect, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AudioDevice } from '../../types';
import { useSettingsStore } from '../../stores/settingsStore';
import { AudioWaveform } from '../AudioWaveform';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

type PermissionStatus = 'checking' | 'granted' | 'denied';

export function PermissionStep({ onValidChange }: StepProps) {
  const [status, setStatus] = useState<PermissionStatus>('checking');
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  const [previewActive, setPreviewActive] = useState(false);
  const { updateSettings } = useSettingsStore();

  const startPreview = useCallback(async (deviceId: string | null) => {
    try {
      await invoke('start_mic_preview', { deviceId });
      setPreviewActive(true);
    } catch (e) {
      console.error('Failed to start mic preview:', e);
    }
  }, []);

  const stopPreview = useCallback(async () => {
    try {
      await invoke('stop_mic_preview');
      setPreviewActive(false);
    } catch (e) {
      console.error('Failed to stop mic preview:', e);
    }
  }, []);

  useEffect(() => {
    checkMicrophoneAccess();
    return () => {
      invoke('stop_mic_preview').catch(() => {});
    };
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
        startPreview(defaultDevice.id);
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
    await stopPreview();
    startPreview(deviceId);
  };

  return (
    <div className="flex flex-col items-center text-center py-6">
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

          <div className="w-full max-w-sm mb-6">
            <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block text-left">
              Microphone
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

          <div className="w-full max-w-sm">
            <label className="text-[0.8rem] text-[var(--text-muted)] mb-2 block text-left">
              Test du microphone
            </label>
            <div className="glass-card p-4 flex items-center justify-center">
              <AudioWaveform active={previewActive} width={280} height={60} />
            </div>
            <p className="text-[0.7rem] text-[var(--text-muted)] mt-2">
              Parlez pour verifier que le microphone fonctionne
            </p>
          </div>
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
          <button onClick={checkMicrophoneAccess} className="btn-glass">
            Reessayer
          </button>
        </>
      )}
    </div>
  );
}
```

**Step 2: Verify TypeScript**

Run: `npx tsc --noEmit`
Expected: no new errors

**Step 3: Commit**

```bash
git add src/components/onboarding/PermissionStep.tsx
git commit -m "feat(onboarding): add real-time waveform and always-visible mic selector to PermissionStep"
```

---

### Task 4: Create TestStep component

**Files:**
- Create: `src/components/onboarding/TestStep.tsx`

**Step 1: Create the test step component**

```tsx
import { useEffect, useState, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useSettingsStore } from '../../stores/settingsStore';
import { AudioWaveform } from '../AudioWaveform';

interface StepProps {
  onValidChange: (valid: boolean) => void;
}

const TEST_PHRASES: Record<string, string> = {
  fr: 'Le soleil brille aujourd\'hui',
  en: 'The sun is shining today',
  es: 'El sol brilla hoy',
  de: 'Die Sonne scheint heute',
  it: 'Il sole splende oggi',
  pt: 'O sol brilha hoje',
  ja: '今日は太陽が輝いています',
  zh: '今天阳光灿烂',
  ko: '오늘 태양이 빛나고 있어요',
  ru: 'Сегодня светит солнце',
  nl: 'De zon schijnt vandaag',
  pl: 'Słońce świeci dzisiaj',
};

type TestStatus = 'idle' | 'countdown' | 'recording' | 'processing' | 'done';

export function TestStep({ onValidChange }: StepProps) {
  const { settings } = useSettingsStore();
  const [status, setStatus] = useState<TestStatus>('idle');
  const [countdown, setCountdown] = useState(3);
  const [result, setResult] = useState<string | null>(null);
  const [recordingTime, setRecordingTime] = useState(0);
  const [previewActive, setPreviewActive] = useState(false);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const recordTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const lang = settings?.transcription_language || 'fr';
  const testPhrase = TEST_PHRASES[lang] || TEST_PHRASES['fr'];

  useEffect(() => {
    onValidChange(true);
  }, [onValidChange]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
      if (recordTimerRef.current) clearInterval(recordTimerRef.current);
      invoke('stop_mic_preview').catch(() => {});
    };
  }, []);

  const startTest = useCallback(async () => {
    setResult(null);
    setStatus('countdown');
    setCountdown(3);

    let count = 3;
    timerRef.current = setInterval(() => {
      count -= 1;
      if (count <= 0) {
        if (timerRef.current) clearInterval(timerRef.current);
        beginRecording();
      } else {
        setCountdown(count);
      }
    }, 1000);
  }, []);

  const beginRecording = async () => {
    setStatus('recording');
    setRecordingTime(0);

    // Start mic preview for waveform
    try {
      await invoke('start_mic_preview', { deviceId: settings?.microphone_id || null });
      setPreviewActive(true);
    } catch (e) {
      console.error('Failed to start mic preview:', e);
    }

    // Start actual recording
    try {
      await invoke('start_recording');
    } catch (e) {
      console.error('Failed to start recording:', e);
      setStatus('idle');
      return;
    }

    // Timer for recording duration
    let elapsed = 0;
    recordTimerRef.current = setInterval(() => {
      elapsed += 0.1;
      setRecordingTime(elapsed);
      if (elapsed >= 5) {
        if (recordTimerRef.current) clearInterval(recordTimerRef.current);
        finishRecording();
      }
    }, 100);
  };

  const finishRecording = async () => {
    setStatus('processing');
    setPreviewActive(false);
    await invoke('stop_mic_preview').catch(() => {});

    try {
      const res = await invoke<{ text: string }>('stop_recording');
      setResult(res.text.trim());
      setStatus('done');
    } catch (e) {
      console.error('Transcription failed:', e);
      setResult(null);
      setStatus('idle');
    }
  };

  const similarity = result ? computeSimilarity(testPhrase.toLowerCase(), result.toLowerCase()) : 0;
  const isGoodMatch = similarity >= 0.5;

  return (
    <div className="py-4">
      <div className="text-center mb-6">
        <h2 className="font-display text-xl text-[var(--text-primary)] mb-2">
          Test de transcription
        </h2>
        <p className="text-[var(--text-secondary)] text-[0.85rem]">
          Verifiez que tout fonctionne en repetant la phrase ci-dessous.
        </p>
      </div>

      {/* Target phrase */}
      <div className="glass-card p-5 mb-6 text-center">
        <p className="text-[0.75rem] text-[var(--text-muted)] mb-2">Phrase a repeter :</p>
        <p className="text-[1.1rem] text-[var(--text-primary)] font-medium">
          &laquo; {testPhrase} &raquo;
        </p>
      </div>

      {/* Action area */}
      <div className="flex flex-col items-center gap-4">
        {status === 'idle' && (
          <button
            onClick={startTest}
            className="px-8 py-3 rounded-xl bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] text-white text-[0.9rem] font-medium hover:opacity-90 transition-all flex items-center gap-2"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
              <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
              <line x1="12" x2="12" y1="19" y2="22" />
            </svg>
            Enregistrer
          </button>
        )}

        {status === 'countdown' && (
          <div className="flex flex-col items-center gap-3">
            <div className="w-20 h-20 rounded-full bg-[rgba(255,255,255,0.08)] border-2 border-[var(--accent-primary)] flex items-center justify-center">
              <span className="text-3xl font-bold text-[var(--accent-primary)]">{countdown}</span>
            </div>
            <p className="text-[0.85rem] text-[var(--text-secondary)]">Preparez-vous...</p>
          </div>
        )}

        {status === 'recording' && (
          <div className="flex flex-col items-center gap-3 w-full max-w-sm">
            <div className="flex items-center gap-2 mb-1">
              <div className="w-3 h-3 rounded-full bg-[var(--accent-danger)] animate-pulse" />
              <span className="text-[0.85rem] text-[var(--accent-danger)] font-medium">
                Enregistrement... {recordingTime.toFixed(1)}s / 5s
              </span>
            </div>
            <div className="glass-card p-3 w-full flex items-center justify-center">
              <AudioWaveform active={previewActive} width={280} height={50} barColor="var(--accent-danger)" />
            </div>
            {/* Progress bar */}
            <div className="w-full h-1.5 rounded-full bg-[rgba(255,255,255,0.08)] overflow-hidden">
              <div
                className="h-full rounded-full bg-[var(--accent-danger)] transition-all duration-100"
                style={{ width: `${(recordingTime / 5) * 100}%` }}
              />
            </div>
          </div>
        )}

        {status === 'processing' && (
          <div className="flex flex-col items-center gap-3">
            <div className="w-8 h-8 border-2 border-[var(--text-muted)] border-t-[var(--accent-primary)] rounded-full animate-spin" />
            <p className="text-[0.85rem] text-[var(--text-secondary)]">Transcription en cours...</p>
          </div>
        )}

        {status === 'done' && result !== null && (
          <div className="w-full max-w-sm space-y-4">
            {/* Result comparison */}
            <div className="glass-card p-5">
              <div className="flex items-center gap-2 mb-3">
                {isGoodMatch ? (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-success)" strokeWidth="2">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <polyline points="22 4 12 14.01 9 11.01" />
                  </svg>
                ) : (
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--accent-warning, #f0a030)" strokeWidth="2">
                    <circle cx="12" cy="12" r="10" />
                    <line x1="12" y1="8" x2="12" y2="12" />
                    <line x1="12" y1="16" x2="12.01" y2="16" />
                  </svg>
                )}
                <span className={`text-[0.85rem] font-medium ${isGoodMatch ? 'text-[var(--accent-success)]' : 'text-[#f0a030]'}`}>
                  {isGoodMatch ? 'Transcription reussie !' : 'Resultat partiel'}
                </span>
              </div>
              <div className="space-y-2">
                <div>
                  <p className="text-[0.7rem] text-[var(--text-muted)] mb-1">Attendu :</p>
                  <p className="text-[0.85rem] text-[var(--text-secondary)]">{testPhrase}</p>
                </div>
                <div>
                  <p className="text-[0.7rem] text-[var(--text-muted)] mb-1">Obtenu :</p>
                  <p className="text-[0.85rem] text-[var(--text-primary)] font-medium">{result || '(aucun texte)'}</p>
                </div>
              </div>
            </div>

            <button
              onClick={() => { setStatus('idle'); setResult(null); }}
              className="btn-glass w-full"
            >
              Reessayer
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

/** Simple word-overlap similarity (0-1) */
function computeSimilarity(a: string, b: string): number {
  const wordsA = new Set(a.split(/\s+/).filter(Boolean));
  const wordsB = new Set(b.split(/\s+/).filter(Boolean));
  if (wordsA.size === 0) return 0;
  let matches = 0;
  for (const w of wordsA) {
    if (wordsB.has(w)) matches++;
  }
  return matches / wordsA.size;
}
```

**Step 2: Verify TypeScript**

Run: `npx tsc --noEmit`
Expected: no new errors

**Step 3: Commit**

```bash
git add src/components/onboarding/TestStep.tsx
git commit -m "feat(onboarding): add TestStep with phrase-repeat transcription test"
```

---

### Task 5: Wire TestStep into OnboardingWizard

**Files:**
- Modify: `src/components/onboarding/OnboardingWizard.tsx`

**Step 1: Add TestStep import and STEPS entry**

At the top of the file, add import:
```tsx
import { TestStep } from './TestStep';
```

Add to the STEPS array after ShortcutsStep:
```tsx
{ label: 'Test', component: TestStep },
```

So STEPS becomes:
```tsx
const STEPS = [
  { label: 'Bienvenue', component: WelcomeStep },
  { label: 'Microphone', component: PermissionStep },
  { label: 'Modele', component: ModelStep },
  { label: 'Langue', component: LanguageStep },
  { label: 'Raccourcis', component: ShortcutsStep },
  { label: 'Test', component: TestStep },
];
```

**Step 2: Verify TypeScript**

Run: `npx tsc --noEmit`
Expected: no new errors

**Step 3: Commit**

```bash
git add src/components/onboarding/OnboardingWizard.tsx
git commit -m "feat(onboarding): add Test step (step 6) to wizard flow"
```

---

### Task 6: Final verification

**Step 1: Verify Rust compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles OK

**Step 2: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: no new errors

**Step 3: Commit all if any remaining changes**

```bash
git status
# If clean, done. Otherwise commit remaining.
```
