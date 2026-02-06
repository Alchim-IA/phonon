use crate::engines::traits::SpeechEngine;
use crate::types::TranscriptionResult;
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use tract_onnx::prelude::*;

type TractModel = SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>;

/// Parakeet model size options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParakeetModelSize {
    #[default]
    Tdt06bV3,
}

impl ParakeetModelSize {
    pub fn model_name(&self) -> &'static str {
        match self {
            ParakeetModelSize::Tdt06bV3 => "parakeet-tdt-0.6b-v3",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ParakeetModelSize::Tdt06bV3 => "Parakeet TDT 0.6B v3",
        }
    }
}

impl From<crate::types::ParakeetModelSize> for ParakeetModelSize {
    fn from(value: crate::types::ParakeetModelSize) -> Self {
        match value {
            crate::types::ParakeetModelSize::Tdt06bV3 => ParakeetModelSize::Tdt06bV3,
        }
    }
}

pub struct ParakeetEngine {
    encoder: Mutex<TractModel>,
    decoder_joint: Mutex<TractModel>,
    vocab: HashMap<i64, String>,
    model_size: ParakeetModelSize,
    blank_id: i64,
}

impl ParakeetEngine {
    pub fn new(model_path: &Path, model_size: ParakeetModelSize) -> Result<Self, String> {
        log::info!("Loading Parakeet model from {:?}", model_path);

        if !model_path.exists() {
            return Err(format!("Parakeet model not found: {:?}", model_path));
        }

        // HuggingFace istupakov/parakeet-tdt-0.6b-v3-onnx format (non-quantized)
        let encoder_file = model_path.join("encoder-model.onnx");
        let decoder_joint_file = model_path.join("decoder_joint-model.onnx");
        let vocab_file = model_path.join("vocab.txt");

        // Check all required files
        for (name, path) in [
            ("Encoder", &encoder_file),
            ("Decoder+Joiner", &decoder_joint_file),
            ("Vocab", &vocab_file),
        ] {
            if !path.exists() {
                return Err(format!("{} file not found: {:?}", name, path));
            }
        }

        // Load encoder with tract-onnx
        log::info!("Loading encoder with tract-onnx...");
        let encoder = tract_onnx::onnx()
            .model_for_path(&encoder_file)
            .map_err(|e| format!("Failed to load encoder model: {}", e))?
            .into_optimized()
            .map_err(|e| format!("Failed to optimize encoder: {}", e))?
            .into_runnable()
            .map_err(|e| format!("Failed to make encoder runnable: {}", e))?;

        // Load decoder+joiner with tract-onnx
        log::info!("Loading decoder+joiner with tract-onnx...");
        let decoder_joint = tract_onnx::onnx()
            .model_for_path(&decoder_joint_file)
            .map_err(|e| format!("Failed to load decoder+joiner model: {}", e))?
            .into_optimized()
            .map_err(|e| format!("Failed to optimize decoder+joiner: {}", e))?
            .into_runnable()
            .map_err(|e| format!("Failed to make decoder+joiner runnable: {}", e))?;

        // Load vocabulary
        let vocab_content = fs::read_to_string(&vocab_file)
            .map_err(|e| format!("Failed to read vocab file: {}", e))?;

        let mut vocab = HashMap::new();
        for (idx, line) in vocab_content.lines().enumerate() {
            let token = line.trim().to_string();
            vocab.insert(idx as i64, token);
        }

        let blank_id = 0i64;

        log::info!(
            "Parakeet model loaded successfully ({} tokens)",
            vocab.len()
        );

        Ok(Self {
            encoder: Mutex::new(encoder),
            decoder_joint: Mutex::new(decoder_joint),
            vocab,
            model_size,
            blank_id,
        })
    }

    /// Radix-2 Cooley-Tukey FFT (in-place, n must be power of 2)
    fn fft(buf: &mut [(f32, f32)]) {
        let n = buf.len();
        if n <= 1 {
            return;
        }

        // Bit-reversal permutation
        let mut j = 0usize;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            if i < j {
                buf.swap(i, j);
            }
        }

        // Butterfly passes
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let angle = -2.0 * std::f32::consts::PI / len as f32;
            let wn = (angle.cos(), angle.sin());
            for i in (0..n).step_by(len) {
                let mut w = (1.0f32, 0.0f32);
                for k in 0..half {
                    let u = buf[i + k];
                    let t = buf[i + k + half];
                    let v = (t.0 * w.0 - t.1 * w.1, t.0 * w.1 + t.1 * w.0);
                    buf[i + k] = (u.0 + v.0, u.1 + v.1);
                    buf[i + k + half] = (u.0 - v.0, u.1 - v.1);
                    w = (w.0 * wn.0 - w.1 * wn.1, w.0 * wn.1 + w.1 * wn.0);
                }
            }
            len <<= 1;
        }
    }

    /// Compute mel-spectrogram features (80 mel bins)
    fn compute_features(&self, audio: &[f32], sample_rate: u32) -> Vec<f32> {
        let n_fft = 512;
        let hop_length = 160;
        let n_mels = 80;
        let fmin = 0.0;
        let fmax = 8000.0;

        let num_frames = if audio.len() > n_fft {
            (audio.len() - n_fft) / hop_length + 1
        } else {
            1
        };

        let mut mel_spec = vec![0.0f32; n_mels * num_frames];
        let mel_filters = Self::create_mel_filterbank(n_fft, sample_rate, n_mels, fmin, fmax);

        // Pre-compute Hann window
        let window: Vec<f32> = (0..n_fft)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n_fft - 1) as f32).cos()))
            .collect();

        let mut fft_buf = vec![(0.0f32, 0.0f32); n_fft];

        for frame_idx in 0..num_frames {
            let start = frame_idx * hop_length;
            let end = (start + n_fft).min(audio.len());

            // Apply window and fill FFT buffer
            for i in 0..n_fft {
                fft_buf[i] = if start + i < end {
                    (audio[start + i] * window[i], 0.0)
                } else {
                    (0.0, 0.0)
                };
            }

            Self::fft(&mut fft_buf);

            // Compute magnitudes and apply mel filterbank
            for mel_idx in 0..n_mels {
                let mut energy = 0.0f32;
                let filter_offset = mel_idx * (n_fft / 2 + 1);
                for freq_idx in 0..=n_fft / 2 {
                    let (re, im) = fft_buf[freq_idx];
                    let mag = (re * re + im * im).sqrt();
                    energy += mag * mel_filters[filter_offset + freq_idx];
                }
                mel_spec[mel_idx * num_frames + frame_idx] = (energy.max(1e-10)).ln();
            }
        }

        // Normalize
        let mean: f32 = mel_spec.iter().sum::<f32>() / mel_spec.len() as f32;
        let variance: f32 = mel_spec.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / mel_spec.len() as f32;
        let std = variance.sqrt().max(1e-5);

        for val in &mut mel_spec {
            *val = (*val - mean) / std;
        }

        mel_spec
    }

    fn create_mel_filterbank(
        n_fft: usize,
        sample_rate: u32,
        n_mels: usize,
        fmin: f32,
        fmax: f32,
    ) -> Vec<f32> {
        let n_freqs = n_fft / 2 + 1;
        let mut filterbank = vec![0.0f32; n_mels * n_freqs];

        let hz_to_mel = |hz: f32| 2595.0 * (1.0 + hz / 700.0).log10();
        let mel_to_hz = |mel: f32| 700.0 * (10.0f32.powf(mel / 2595.0) - 1.0);

        let mel_min = hz_to_mel(fmin);
        let mel_max = hz_to_mel(fmax);

        let mel_points: Vec<f32> = (0..=n_mels + 1)
            .map(|i| mel_min + (mel_max - mel_min) * i as f32 / (n_mels + 1) as f32)
            .collect();

        let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();
        let bin_points: Vec<usize> = hz_points
            .iter()
            .map(|&hz| ((n_fft as f32 + 1.0) * hz / sample_rate as f32).floor() as usize)
            .collect();

        for m in 0..n_mels {
            let start = bin_points[m];
            let center = bin_points[m + 1];
            let end = bin_points[m + 2];

            for k in start..center {
                if k < n_freqs && center > start {
                    filterbank[m * n_freqs + k] = (k - start) as f32 / (center - start) as f32;
                }
            }

            for k in center..end {
                if k < n_freqs && end > center {
                    filterbank[m * n_freqs + k] = (end - k) as f32 / (end - center) as f32;
                }
            }
        }

        filterbank
    }

    fn decode_tokens(&self, token_ids: &[i64]) -> String {
        let mut text = String::new();

        for &id in token_ids {
            if id == self.blank_id {
                continue;
            }
            if let Some(token) = self.vocab.get(&id) {
                // Handle SentencePiece tokens (underscore = space)
                let token = token.replace("▁", " ");
                text.push_str(&token);
            }
        }

        text.trim().to_string()
    }

    fn greedy_decode(
        &self,
        encoder_out: &tract_ndarray::ArrayD<f32>,
    ) -> Result<Vec<i64>, String> {
        let shape = encoder_out.shape();
        let time_steps = shape[1];
        let encoder_dim = shape[2];

        let mut decoded_tokens: Vec<i64> = Vec::new();
        let decoder_joint = self.decoder_joint.lock().map_err(|e| e.to_string())?;

        // Initial decoder state
        let mut last_token = self.blank_id;

        for t in 0..time_steps {
            // Get encoder frame at timestep t
            let encoder_frame: Vec<f32> = (0..encoder_dim)
                .map(|d| encoder_out[[0, t, d]])
                .collect();

            // Create input tensors for decoder+joiner
            // Shape: [batch=1, time=1, features]
            let encoder_tensor: Tensor = tract_ndarray::Array3::from_shape_vec(
                (1, 1, encoder_dim),
                encoder_frame,
            )
            .map_err(|e| format!("Encoder tensor error: {}", e))?
            .into();

            // Decoder input: previous token
            let decoder_input: Tensor = tract_ndarray::Array2::from_shape_vec(
                (1, 1),
                vec![last_token],
            )
            .map_err(|e| format!("Decoder input error: {}", e))?
            .into();

            // Run decoder+joiner
            let inputs = tvec![encoder_tensor.into(), decoder_input.into()];
            let outputs = decoder_joint
                .run(inputs)
                .map_err(|e| format!("Decoder+joiner error: {}", e))?;

            let logits = outputs[0]
                .to_array_view::<f32>()
                .map_err(|e| format!("Output error: {}", e))?;

            // Find argmax
            let mut max_idx = 0i64;
            let mut max_val = f32::NEG_INFINITY;
            for (i, &val) in logits.iter().enumerate() {
                if val > max_val {
                    max_val = val;
                    max_idx = i as i64;
                }
            }

            if max_idx != self.blank_id {
                decoded_tokens.push(max_idx);
                last_token = max_idx;
            }
        }

        Ok(decoded_tokens)
    }
}

impl SpeechEngine for ParakeetEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        let start_time = std::time::Instant::now();

        if sample_rate != 16000 {
            return Err(format!(
                "Invalid sample rate: {}Hz (expected 16000Hz)",
                sample_rate
            ));
        }

        let duration_seconds = audio.len() as f32 / sample_rate as f32;

        if duration_seconds < 0.1 {
            return Err("Audio too short".to_string());
        }

        // Compute mel-spectrogram features
        let n_mels = 80;
        let features = self.compute_features(audio, sample_rate);
        let num_frames = features.len() / n_mels;

        // Create encoder input tensor [batch, time, features]
        // Note: features is [n_mels, num_frames], need to transpose to [num_frames, n_mels]
        let mut transposed = vec![0.0f32; num_frames * n_mels];
        for mel in 0..n_mels {
            for frame in 0..num_frames {
                transposed[frame * n_mels + mel] = features[mel * num_frames + frame];
            }
        }

        let features_tensor: Tensor = tract_ndarray::Array3::from_shape_vec(
            (1, num_frames, n_mels),
            transposed,
        )
        .map_err(|e| format!("Failed to create features tensor: {}", e))?
        .into();

        // Run encoder
        let encoder = self.encoder.lock().map_err(|e| e.to_string())?;
        let encoder_outputs = encoder
            .run(tvec![features_tensor.into()])
            .map_err(|e| format!("Encoder error: {}", e))?;

        let encoder_out = encoder_outputs[0]
            .to_array_view::<f32>()
            .map_err(|e| format!("Encoder output error: {}", e))?;

        let encoder_out_owned = encoder_out.to_owned().into_dyn();

        drop(encoder);

        // Greedy decode
        let token_ids = self.greedy_decode(&encoder_out_owned)?;
        let text = self.decode_tokens(&token_ids);

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        log::info!(
            "Parakeet transcription completed in {}ms: {} chars",
            processing_time_ms,
            text.len()
        );

        Ok(TranscriptionResult {
            text,
            confidence: 0.9,
            duration_seconds,
            processing_time_ms,
            detected_language: Some("auto".to_string()),
            timestamp: Utc::now().timestamp(),
            model_used: Some(self.model_display_name()),
        })
    }

    fn name(&self) -> &str {
        "Parakeet"
    }

    fn model_display_name(&self) -> String {
        format!("Parakeet {}", self.model_size.display_name())
    }
}

unsafe impl Send for ParakeetEngine {}
unsafe impl Sync for ParakeetEngine {}
