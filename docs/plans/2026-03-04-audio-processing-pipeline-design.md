# Audio Processing Pipeline Design

## Context

Phonon currently passes raw audio directly from CPAL capture to Whisper after resampling. No audio conditioning is performed, which leads to suboptimal transcription quality with varying input levels, background noise, and silence segments.

## Goal

Insert a complete audio processing pipeline between capture and transcription to improve Whisper input quality.

## Pipeline Order

```
Micro (CPAL) → Mono 48kHz → Noise Suppression → Resampling 16kHz → Soft Limiter → AGC → RMS Norm → VAD → Whisper
```

### Why this order

- **Noise suppression before resampling**: nnnoiseless (RNNoise) expects 48kHz input, operates on 480-sample frames
- **Soft limiter first after resampling**: protects downstream stages from clipped signal
- **AGC before normalization**: dynamic gain adjustment smooths level variations, then normalization locks to target
- **VAD last**: benefits from clean, normalized signal for accurate speech detection

## Components

### 1. Noise Suppression (nnnoiseless)

- Crate: `nnnoiseless` (pure Rust RNNoise port)
- Frame size: 480 samples at 48kHz
- Applied at capture sample rate before resampling
- Processes audio in 480-sample frames, pads last frame if needed

### 2. Soft Limiter

- Pure Rust implementation
- Threshold: -1 dBFS (0.891)
- Soft knee: 6 dB range
- Prevents hard clipping that destroys transcription quality
- Logs clipping events for diagnostics

### 3. AGC (Automatic Gain Control)

- Pure Rust implementation
- Target level: -20 dBFS RMS (~0.1 linear)
- Attack time: 10ms (fast response to loud signals)
- Release time: 100ms (smooth recovery)
- Max gain: 30 dB (prevent amplifying pure noise)
- Min gain: -10 dB (allow attenuation of very loud signals)
- Operates on short analysis windows (~20ms)

### 4. RMS Normalization

- Pure Rust implementation
- Target: -20 dBFS RMS
- Final safety net to ensure consistent level for Whisper
- Applied to entire buffer after AGC

### 5. VAD (Voice Activity Detection)

- Pure Rust implementation
- Energy-based: RMS threshold with adaptive noise floor
- Zero-crossing rate as secondary feature
- Minimum speech duration: 250ms (avoid detecting transients)
- Returns `has_speech: bool` to skip transcription on silence/noise-only segments
- Frame-based analysis (20ms frames)

## API

```rust
pub struct AudioProcessor {
    denoiser: Option<DenoiseState>,  // nnnoiseless, initialized for 48kHz
    agc_gain: f32,                    // current AGC gain state
    noise_floor: f32,                 // adaptive noise floor for VAD
}

impl AudioProcessor {
    pub fn new() -> Self;

    /// Full pipeline: returns (processed_audio, has_speech)
    /// audio is at capture sample rate, output is at capture sample rate
    /// Resampling is done externally after this call
    pub fn process_pre_resample(&mut self, audio: &[f32], sample_rate: u32) -> Vec<f32>;

    /// Post-resample processing at 16kHz: limiter + AGC + normalization + VAD
    pub fn process_post_resample(&mut self, audio: &[f32]) -> (Vec<f32>, bool);
}
```

## Integration Points

The processor is called in 4 locations:

1. **`stop_recording()`** in `transcription.rs` — full transcription after GUI recording
2. **`run_streaming_task()`** in `transcription.rs` — each streaming chunk
3. **`stop_ptt_and_paste()`** in `ptt.rs` — PTT final transcription
4. **`start_streaming_transcription()`** in `ptt.rs` — PTT streaming snapshots

Pattern at each site:
```rust
let processed = processor.process_pre_resample(&audio, sample_rate);
let resampled = resample_audio(&processed, sample_rate, TARGET_SAMPLE_RATE);
let (final_audio, has_speech) = processor.process_post_resample(&resampled);
if !has_speech { /* skip transcription */ }
```

## New Dependency

```toml
nnnoiseless = "0.5"
```

## Files to Create/Modify

- **Create**: `src-tauri/src/audio/processing.rs` — AudioProcessor with all 5 stages
- **Modify**: `src-tauri/src/audio/mod.rs` — add `pub mod processing`
- **Modify**: `src-tauri/Cargo.toml` — add nnnoiseless dependency
- **Modify**: `src-tauri/src/commands/transcription.rs` — integrate processor
- **Modify**: `src-tauri/src/ptt.rs` — integrate processor
- **Modify**: `src-tauri/src/state.rs` — add AudioProcessor to AppState
