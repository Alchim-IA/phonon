# Audio Processing Pipeline Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a 5-stage audio processing pipeline (noise suppression, soft limiter, AGC, RMS normalization, VAD) between audio capture and Whisper transcription to improve transcription quality.

**Architecture:** A single `AudioProcessor` struct in `src-tauri/src/audio/processing.rs` with two phases: `process_pre_resample()` for noise suppression at native sample rate (48kHz), and `process_post_resample()` for limiter + AGC + normalization + VAD at 16kHz. The processor is stored in `AppState` and passed to all transcription sites.

**Tech Stack:** Rust, nnnoiseless (RNNoise pure-Rust port), pure Rust DSP for limiter/AGC/normalization/VAD.

---

### Task 1: Add nnnoiseless dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add the dependency**

In `src-tauri/Cargo.toml`, add after the `rubato = "0.15"` line:

```toml
# Audio noise suppression (RNNoise pure Rust)
nnnoiseless = "0.5"
```

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully (nnnoiseless is pure Rust, no system deps)

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "feat: add nnnoiseless dependency for audio noise suppression"
```

---

### Task 2: Create AudioProcessor with noise suppression

**Files:**
- Create: `src-tauri/src/audio/processing.rs`
- Modify: `src-tauri/src/audio/mod.rs`

**Step 1: Create `processing.rs` with noise suppression**

Create `src-tauri/src/audio/processing.rs`:

```rust
use nnnoiseless::DenoiseState;

/// Audio processing pipeline for improving transcription quality.
///
/// Pipeline order:
/// 1. Noise suppression (nnnoiseless/RNNoise) — at capture sample rate (48kHz)
/// 2. [resampling happens externally to 16kHz]
/// 3. Soft limiter — prevents clipping
/// 4. AGC — dynamic gain adjustment
/// 5. RMS normalization — locks level to -20 dBFS
/// 6. VAD — detects speech presence
pub struct AudioProcessor {
    denoiser: Box<DenoiseState<'static>>,
    agc_gain: f32,
    noise_floor: f32,
}

/// Target RMS level: -20 dBFS ≈ 0.1 in linear scale
const TARGET_RMS: f32 = 0.1;

/// Soft limiter threshold: -1 dBFS ≈ 0.891
const LIMITER_THRESHOLD: f32 = 0.891;

/// Soft limiter knee width in linear scale (6 dB range)
const LIMITER_KNEE: f32 = 0.25;

/// AGC attack coefficient (10ms at 16kHz, ~160 samples per frame)
const AGC_ATTACK: f32 = 0.1;

/// AGC release coefficient (100ms at 16kHz)
const AGC_RELEASE: f32 = 0.01;

/// Maximum AGC gain in linear (~30 dB)
const AGC_MAX_GAIN: f32 = 31.62;

/// Minimum AGC gain in linear (~-10 dB)
const AGC_MIN_GAIN: f32 = 0.316;

/// VAD: minimum RMS to consider as speech (well above noise floor)
const VAD_ENERGY_THRESHOLD: f32 = 0.01;

/// VAD: minimum fraction of frames that must contain speech
const VAD_SPEECH_RATIO: f32 = 0.15;

/// VAD analysis frame size in samples (20ms at 16kHz)
const VAD_FRAME_SIZE: usize = 320;

impl AudioProcessor {
    pub fn new() -> Self {
        Self {
            denoiser: DenoiseState::new(),
            agc_gain: 1.0,
            noise_floor: 0.001,
        }
    }

    /// Phase 1: Pre-resample processing at capture sample rate.
    /// Applies noise suppression (requires 48kHz input).
    /// If sample_rate is not 48000, noise suppression is skipped.
    pub fn process_pre_resample(&mut self, audio: &[f32], sample_rate: u32) -> Vec<f32> {
        if sample_rate == 48000 {
            self.denoise(audio)
        } else {
            log::debug!(
                "Skipping noise suppression: sample rate {}Hz != 48000Hz",
                sample_rate
            );
            audio.to_vec()
        }
    }

    /// Phase 2: Post-resample processing at 16kHz.
    /// Applies: soft limiter → AGC → RMS normalization → VAD.
    /// Returns (processed_audio, has_speech).
    pub fn process_post_resample(&mut self, audio: &[f32]) -> (Vec<f32>, bool) {
        let mut output = self.soft_limit(audio);
        self.apply_agc(&mut output);
        self.normalize_rms(&mut output);
        let has_speech = self.detect_speech(&output);
        (output, has_speech)
    }

    /// Noise suppression using nnnoiseless (RNNoise).
    /// Expects 48kHz audio in [-1.0, 1.0] range.
    /// nnnoiseless expects i16-scale floats [-32768.0, 32767.0].
    fn denoise(&mut self, audio: &[f32]) -> Vec<f32> {
        let frame_size = DenoiseState::FRAME_SIZE; // 480 samples
        let mut output = Vec::with_capacity(audio.len());
        let mut frame_output = vec![0.0f32; frame_size];
        let mut first_frame = true;

        // Process complete frames
        let mut i = 0;
        while i + frame_size <= audio.len() {
            // Convert from [-1.0, 1.0] to [-32768.0, 32767.0]
            let frame_input: Vec<f32> = audio[i..i + frame_size]
                .iter()
                .map(|&s| s * 32767.0)
                .collect();

            self.denoiser
                .process_frame(&mut frame_output, &frame_input);

            if first_frame {
                // Discard first frame (RNNoise fade-in artifact)
                first_frame = false;
            } else {
                // Convert back from i16-scale to [-1.0, 1.0]
                output.extend(frame_output.iter().map(|&s| s / 32767.0));
            }

            i += frame_size;
        }

        // Handle remaining samples: pad with zeros
        if i < audio.len() {
            let mut padded = vec![0.0f32; frame_size];
            for (j, &sample) in audio[i..].iter().enumerate() {
                padded[j] = sample * 32767.0;
            }
            self.denoiser
                .process_frame(&mut frame_output, &padded);

            if first_frame {
                // Edge case: audio shorter than one frame
                // Still output it since we have nothing else
            }

            let remaining = audio.len() - i;
            output.extend(
                frame_output[..remaining]
                    .iter()
                    .map(|&s| s / 32767.0),
            );
        }

        output
    }

    /// Soft-knee limiter to prevent clipping.
    fn soft_limit(&self, audio: &[f32]) -> Vec<f32> {
        audio
            .iter()
            .map(|&sample| {
                let abs = sample.abs();
                if abs <= LIMITER_THRESHOLD - LIMITER_KNEE / 2.0 {
                    // Below knee: pass through
                    sample
                } else if abs >= LIMITER_THRESHOLD + LIMITER_KNEE / 2.0 {
                    // Above knee: hard limit
                    sample.signum() * LIMITER_THRESHOLD
                } else {
                    // In knee: smooth transition
                    let x = abs - (LIMITER_THRESHOLD - LIMITER_KNEE / 2.0);
                    let gain = 1.0 - x / (2.0 * LIMITER_KNEE);
                    sample.signum() * (abs * gain + LIMITER_THRESHOLD * (1.0 - gain))
                }
            })
            .collect()
    }

    /// Automatic Gain Control with attack/release envelope.
    fn apply_agc(&mut self, audio: &mut [f32]) {
        let frame_size = 320; // 20ms at 16kHz

        for frame in audio.chunks_mut(frame_size) {
            // Compute RMS of this frame
            let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();

            if rms > 0.0001 {
                let desired_gain = TARGET_RMS / rms;
                let desired_gain = desired_gain.clamp(AGC_MIN_GAIN, AGC_MAX_GAIN);

                // Smooth gain changes
                let coeff = if desired_gain < self.agc_gain {
                    AGC_ATTACK // Fast attack
                } else {
                    AGC_RELEASE // Slow release
                };

                self.agc_gain += coeff * (desired_gain - self.agc_gain);
            }

            // Apply gain
            for sample in frame.iter_mut() {
                *sample *= self.agc_gain;
            }
        }
    }

    /// RMS normalization to target level (-20 dBFS).
    fn normalize_rms(&self, audio: &mut [f32]) {
        if audio.is_empty() {
            return;
        }

        let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();

        if rms > 0.0001 {
            let gain = TARGET_RMS / rms;
            // Clamp gain to avoid extreme amplification
            let gain = gain.clamp(AGC_MIN_GAIN, AGC_MAX_GAIN);
            for sample in audio.iter_mut() {
                *sample *= gain;
                // Final safety clamp
                *sample = sample.clamp(-1.0, 1.0);
            }
        }
    }

    /// Voice Activity Detection based on energy and zero-crossing rate.
    /// Returns true if speech is detected in the audio.
    fn detect_speech(&mut self, audio: &[f32]) -> bool {
        if audio.is_empty() {
            return false;
        }

        let mut speech_frames = 0;
        let mut total_frames = 0;

        for frame in audio.chunks(VAD_FRAME_SIZE) {
            if frame.len() < VAD_FRAME_SIZE / 2 {
                break; // Skip very short trailing frames
            }

            total_frames += 1;

            // RMS energy
            let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();

            // Zero-crossing rate
            let zcr = frame
                .windows(2)
                .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
                .count() as f32
                / frame.len() as f32;

            // Update adaptive noise floor (slow adaptation)
            if rms < self.noise_floor * 2.0 {
                self.noise_floor = self.noise_floor * 0.99 + rms * 0.01;
            }

            // Speech detection: energy well above noise floor + reasonable ZCR
            let energy_above_noise = rms > self.noise_floor * 3.0 && rms > VAD_ENERGY_THRESHOLD;
            let zcr_in_speech_range = zcr > 0.02 && zcr < 0.5;

            if energy_above_noise && zcr_in_speech_range {
                speech_frames += 1;
            }
        }

        if total_frames == 0 {
            return false;
        }

        let speech_ratio = speech_frames as f32 / total_frames as f32;
        speech_ratio >= VAD_SPEECH_RATIO
    }

    /// Reset processor state (call between recordings).
    pub fn reset(&mut self) {
        self.denoiser = DenoiseState::new();
        self.agc_gain = 1.0;
        // Keep noise_floor as it adapts over time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_limiter_passthrough() {
        let processor = AudioProcessor::new();
        let quiet_audio: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin() * 0.5).collect();
        let limited = processor.soft_limit(&quiet_audio);
        // Quiet audio should pass through unchanged (below threshold)
        for (orig, lim) in quiet_audio.iter().zip(limited.iter()) {
            assert!((orig - lim).abs() < 0.01, "Quiet audio should pass through limiter");
        }
    }

    #[test]
    fn test_soft_limiter_clips_loud() {
        let processor = AudioProcessor::new();
        let loud_audio: Vec<f32> = vec![1.0, -1.0, 0.95, -0.95, 1.5, -1.5];
        let limited = processor.soft_limit(&loud_audio);
        for sample in &limited {
            assert!(
                sample.abs() <= LIMITER_THRESHOLD + 0.01,
                "Limiter should prevent clipping: got {}",
                sample
            );
        }
    }

    #[test]
    fn test_rms_normalization() {
        let processor = AudioProcessor::new();
        // Create a loud signal (~0.5 RMS)
        let mut audio: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.1).sin() * 0.7).collect();
        processor.normalize_rms(&mut audio);
        let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
        // Should be close to TARGET_RMS (0.1)
        assert!(
            (rms - TARGET_RMS).abs() < 0.02,
            "RMS should be near target: got {}",
            rms
        );
    }

    #[test]
    fn test_vad_detects_silence() {
        let mut processor = AudioProcessor::new();
        let silence: Vec<f32> = vec![0.0; 16000]; // 1 second of silence
        assert!(
            !processor.detect_speech(&silence),
            "VAD should not detect speech in silence"
        );
    }

    #[test]
    fn test_vad_detects_speech() {
        let mut processor = AudioProcessor::new();
        // Simulate speech-like signal: moderate energy with varying ZCR
        let speech: Vec<f32> = (0..16000)
            .map(|i| {
                let t = i as f32 / 16000.0;
                (t * 440.0 * std::f32::consts::TAU).sin() * 0.3
                    + (t * 880.0 * std::f32::consts::TAU).sin() * 0.1
            })
            .collect();
        assert!(
            processor.detect_speech(&speech),
            "VAD should detect speech-like signal"
        );
    }

    #[test]
    fn test_agc_amplifies_quiet() {
        let mut processor = AudioProcessor::new();
        // Very quiet signal
        let mut audio: Vec<f32> = (0..3200).map(|i| (i as f32 * 0.1).sin() * 0.01).collect();
        let original_rms =
            (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
        processor.apply_agc(&mut audio);
        let new_rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
        assert!(
            new_rms > original_rms,
            "AGC should amplify quiet audio: {} -> {}",
            original_rms,
            new_rms
        );
    }

    #[test]
    fn test_full_pipeline() {
        let mut processor = AudioProcessor::new();
        // Test post-resample pipeline with speech-like audio
        let audio: Vec<f32> = (0..16000)
            .map(|i| {
                let t = i as f32 / 16000.0;
                (t * 300.0 * std::f32::consts::TAU).sin() * 0.5
            })
            .collect();
        let (processed, has_speech) = processor.process_post_resample(&audio);
        assert_eq!(processed.len(), audio.len());
        assert!(has_speech, "Should detect speech in sinusoidal signal");
        // Check all samples are in valid range
        for sample in &processed {
            assert!(
                sample.abs() <= 1.0,
                "All samples should be in [-1.0, 1.0]"
            );
        }
    }
}
```

**Step 2: Update `mod.rs`**

In `src-tauri/src/audio/mod.rs`, add:

```rust
pub mod processing;
```

And add to the public re-exports:

```rust
pub use processing::AudioProcessor;
```

**Step 3: Run the tests**

Run: `cd src-tauri && cargo test --lib audio::processing`
Expected: All 7 tests pass

**Step 4: Commit**

```bash
git add src-tauri/src/audio/processing.rs src-tauri/src/audio/mod.rs
git commit -m "feat: add AudioProcessor with noise suppression, limiter, AGC, normalization, and VAD"
```

---

### Task 3: Add AudioProcessor to AppState

**Files:**
- Modify: `src-tauri/src/state.rs`

**Step 1: Add AudioProcessor field to AppState**

In `src-tauri/src/state.rs`, add import:

```rust
use crate::audio::AudioProcessor;
```

Add field to `AppState` struct (after `audio_buffer`):

```rust
pub audio_processor: Arc<RwLock<AudioProcessor>>,
```

In `AppState::new()`, add to the `Ok(Self { ... })` block:

```rust
audio_processor: Arc::new(RwLock::new(AudioProcessor::new())),
```

**Step 2: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src-tauri/src/state.rs
git commit -m "feat: add AudioProcessor to AppState"
```

---

### Task 4: Integrate processor into GUI transcription (stop_recording)

**Files:**
- Modify: `src-tauri/src/commands/transcription.rs`

**Step 1: Modify `stop_recording()` in `transcription.rs`**

After the line `let audio_buffer = audio_result.audio;` (line ~337) and before the resampling block, add pre-resample processing:

```rust
// Audio processing: pre-resample (noise suppression)
let audio_buffer = {
    let mut processor = state.audio_processor.write().map_err(|e| e.to_string())?;
    processor.process_pre_resample(&audio_buffer, sample_rate)
};
```

After the resampling block (after `let resampled_audio = ...`), add post-resample processing:

```rust
// Audio processing: post-resample (limiter + AGC + normalization + VAD)
let (resampled_audio, has_speech) = {
    let mut processor = state.audio_processor.write().map_err(|e| e.to_string())?;
    processor.process_post_resample(&resampled_audio)
};

if !has_speech {
    log::info!("VAD: no speech detected, skipping transcription");
    emit_recording_status(&app, "idle");
    return Err("No speech detected in recording".to_string());
}
```

Also reset the processor at the start of `start_recording()`, after the capture starts:

```rust
// Reset audio processor for new recording
if let Ok(mut processor) = state.audio_processor.write() {
    processor.reset();
}
```

**Step 2: Modify `run_streaming_task()` in `transcription.rs`**

The streaming task needs access to the processor. Change the function signature to accept the processor:

```rust
fn run_streaming_task(
    app: AppHandle,
    state: Arc<RwLock<Option<Box<dyn SpeechEngine>>>>,
    processor: Arc<RwLock<AudioProcessor>>,
)
```

Update the call site in `start_recording()`:

```rust
let processor_clone = state.audio_processor.clone();
std::thread::spawn(move || {
    run_streaming_task(app_clone, engine_clone, processor_clone);
});
```

Inside `run_streaming_task`, after getting `chunk_audio` and before resampling:

```rust
// Pre-resample processing
let chunk_audio = if let Ok(mut proc) = processor.write() {
    proc.process_pre_resample(chunk_audio, sample_rate)
} else {
    chunk_audio.to_vec()
};
```

After resampling, before transcription:

```rust
// Post-resample processing
let (resampled, has_speech) = if let Ok(mut proc) = processor.write() {
    proc.process_post_resample(&resampled)
} else {
    (resampled, true)
};

if !has_speech {
    log::debug!("Streaming chunk: no speech detected, skipping");
    continue;
}
```

**Step 3: Add the import**

At the top of `transcription.rs`, add:

```rust
use crate::audio::AudioProcessor;
```

**Step 4: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src-tauri/src/commands/transcription.rs
git commit -m "feat: integrate audio processor into GUI transcription pipeline"
```

---

### Task 5: Integrate processor into PTT transcription

**Files:**
- Modify: `src-tauri/src/ptt.rs`

**Step 1: Modify `stop_ptt_and_paste()` in `ptt.rs`**

After getting `audio_data` and `sample_rate`, before the resampling block:

```rust
// Audio processing: pre-resample
let audio_data = {
    let state: tauri::State<'_, AppState> = app.state();
    if let Ok(mut processor) = state.audio_processor.write() {
        processor.process_pre_resample(&audio_data, sample_rate)
    } else {
        audio_data
    }
};
```

After resampling, before transcription:

```rust
// Audio processing: post-resample
let (resampled_audio, has_speech) = {
    let state: tauri::State<'_, AppState> = app.state();
    if let Ok(mut processor) = state.audio_processor.write() {
        processor.process_post_resample(&resampled_audio)
    } else {
        (resampled_audio, true)
    }
};

if !has_speech {
    log::info!("PTT: no speech detected, skipping");
    return;
}
```

**Step 2: Modify `start_streaming_transcription()` in `ptt.rs`**

After resampling, before transcription:

```rust
// Audio processing: pre-resample
let audio_data = {
    let state: tauri::State<'_, AppState> = app.state();
    if let Ok(mut proc) = state.audio_processor.write() {
        proc.process_pre_resample(&audio_data, sample_rate)
    } else {
        audio_data
    }
};

let resampled = if sample_rate != TARGET_SAMPLE_RATE {
    crate::audio::resampling::resample_audio(&audio_data, sample_rate, TARGET_SAMPLE_RATE)
} else {
    audio_data
};

// Audio processing: post-resample
let (resampled, has_speech) = {
    let state: tauri::State<'_, AppState> = app.state();
    if let Ok(mut proc) = state.audio_processor.write() {
        proc.process_post_resample(&resampled)
    } else {
        (resampled, true)
    }
};

if !has_speech {
    continue;
}
```

**Step 3: Modify `stop_voice_action_and_execute()` in `ptt.rs`**

Same pattern — add pre-resample processing before resampling, and post-resample + VAD check after resampling, before transcription. If `!has_speech`, set tray idle and return early.

**Step 4: Reset processor on PTT start**

In `start_ptt_recording()` or at the beginning of `stop_ptt_and_paste()`, reset:

```rust
{
    let state: tauri::State<'_, AppState> = app.state();
    if let Ok(mut processor) = state.audio_processor.write() {
        processor.reset();
    }
}
```

Note: `start_ptt_recording()` doesn't have access to the app handle, so do the reset at the start of `stop_ptt_and_paste()` instead.

**Step 5: Verify compilation**

Run: `cd src-tauri && cargo check`
Expected: Compiles successfully

**Step 6: Commit**

```bash
git add src-tauri/src/ptt.rs
git commit -m "feat: integrate audio processor into PTT transcription pipeline"
```

---

### Task 6: Full build and manual test

**Step 1: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass (including the 7 new audio processing tests)

**Step 2: Build the full application**

Run: `cd src-tauri && cargo build`
Expected: Build succeeds

**Step 3: Manual test checklist**

- Start the app
- Record via GUI → check logs for "Audio processing" messages
- Record silence → should see "No speech detected" in logs
- Record speech → should transcribe normally
- Use PTT hotkey → same checks
- Check that streaming still works during PTT

**Step 4: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: audio processing pipeline adjustments from testing"
```
