use std::num::NonZeroU32;
use std::path::Path;
use std::sync::OnceLock;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use crate::types::LocalLlmModel;

/// Global singleton backend — LlamaBackend::init() can only succeed once per process.
static LLAMA_BACKEND: OnceLock<LlamaBackend> = OnceLock::new();

fn get_backend() -> &'static LlamaBackend {
    LLAMA_BACKEND.get_or_init(|| {
        LlamaBackend::init().expect("Failed to initialize llama backend")
    })
}

/// Number of physical CPU cores (for optimal thread count)
fn physical_cores() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32 / 2) // available_parallelism returns logical cores
        .unwrap_or(4)
        .max(1)
}

/// Moteur LLM local via llama.cpp (GGUF)
pub struct LocalLlmEngine {
    model: LlamaModel,
    model_type: LocalLlmModel,
}

impl LocalLlmEngine {
    pub fn new(model_path: &Path, model_type: LocalLlmModel) -> Result<Self, String> {
        log::info!(
            "Initializing Local LLM engine: {:?} from {:?}",
            model_type,
            model_path
        );

        let backend = get_backend();

        // Apple Silicon: GPU offload. Intel Mac: CPU-only (Metal too slow on AMD GPU).
        let gpu_layers = if Self::is_apple_silicon() { 1000 } else { 0 };
        log::info!("GPU layers: {} (Apple Silicon: {})", gpu_layers, Self::is_apple_silicon());
        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(gpu_layers);

        let model = LlamaModel::load_from_file(backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load model from {:?}: {}", model_path, e))?;

        log::info!(
            "Local LLM loaded: {} params, vocab size {}, {} physical cores",
            model.n_params(),
            model.n_vocab(),
            physical_cores()
        );

        Ok(Self { model, model_type })
    }

    pub fn summarize(&self, text: &str) -> Result<String, String> {
        // Short prompt for speed
        self.generate("Resume en 2-3 phrases concises en francais:", text, 256)
    }

    pub fn translate(&self, text: &str, target_language: &str) -> Result<String, String> {
        let instruction = format!("Translate to {}:", target_language);

        // Single pass — creating multiple contexts is more expensive than a larger context
        let input_tokens = text.len() / 4;
        let max_tokens = ((input_tokens as f32 * 1.3) as usize).clamp(32, 1024);
        self.generate(&instruction, text, max_tokens)
    }

    fn generate(&self, instruction: &str, text: &str, max_output_tokens: usize) -> Result<String, String> {
        let gen_start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(5);

        let backend = get_backend();
        let prompt = self.model_type.format_prompt(instruction, text);

        // Tokenize first to know exact prompt size
        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| format!("Failed to tokenize prompt: {}", e))?;

        if tokens.is_empty() {
            return Err("Prompt tokenized to zero tokens".to_string());
        }

        let n_prompt = tokens.len();
        // Context = prompt + output, no waste
        let n_ctx = (n_prompt + max_output_tokens + 16) as u32;
        let cores = physical_cores();

        log::info!(
            "Generating: {} prompt tokens, max {} output tokens, ctx={}, threads={}",
            n_prompt, max_output_tokens, n_ctx, cores
        );

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(n_ctx).unwrap()))
            .with_n_batch(n_prompt as u32)       // Process entire prompt in one batch
            .with_n_threads(cores)                // Optimal thread count for generation
            .with_n_threads_batch(cores * 2)      // More threads for prompt processing
            ;

        let mut ctx = self
            .model
            .new_context(backend, ctx_params)
            .map_err(|e| format!("Failed to create inference context: {}", e))?;

        // Feed entire prompt in one batch
        let mut batch = LlamaBatch::new(n_prompt + 1, 1);
        for (i, token) in tokens.iter().enumerate() {
            batch
                .add(*token, i as i32, &[0], i == n_prompt - 1)
                .map_err(|e| format!("Failed to add token to batch: {}", e))?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| format!("Failed to decode prompt: {}", e))?;

        // Greedy sampling with light repetition penalty — fastest possible
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::penalties(32, 1.2, 0.0, 0.0),
            LlamaSampler::greedy(),
        ]);

        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut output = String::new();
        let mut n_cur = n_prompt as i32;

        for _ in 0..max_output_tokens {
            // Timeout check — stop if we've spent too long
            if gen_start.elapsed() > timeout {
                log::warn!("Generation timeout reached ({:.1}s), returning partial result", gen_start.elapsed().as_secs_f32());
                break;
            }

            let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(new_token);

            if self.model.is_eog_token(new_token) {
                break;
            }

            match self.model.token_to_piece(new_token, &mut decoder, true, None) {
                Ok(piece) => output.push_str(&piece),
                Err(_) => {}
            }

            // Fast repetition loop detection (char-safe)
            if output.len() > 60 {
                // Work with chars to avoid slicing inside multi-byte characters
                let chars: Vec<char> = output.chars().collect();
                let n = chars.len();
                if n > 20 {
                    let check = 10.min(n / 2);
                    let a: String = chars[n - check * 2..n - check].iter().collect();
                    let b: String = chars[n - check..].iter().collect();
                    if a == b {
                        log::warn!("Repetition loop detected, stopping");
                        let keep: String = chars[..n - check * 2].iter().collect();
                        output = keep;
                        break;
                    }
                }
            }

            batch.clear();
            batch
                .add(new_token, n_cur, &[0], true)
                .map_err(|e| format!("Failed to add generated token: {}", e))?;
            n_cur += 1;

            ctx.decode(&mut batch)
                .map_err(|e| format!("Failed to decode generated token: {}", e))?;
        }

        let result = output.trim().to_string();
        if result.is_empty() {
            return Err("Model generated empty output".to_string());
        }

        log::info!("Generated {} chars ({} output tokens)", result.len(), n_cur - n_prompt as i32);
        Ok(result)
    }

    pub fn model_type(&self) -> LocalLlmModel {
        self.model_type
    }

    pub fn display_name(&self) -> String {
        format!("Local LLM ({})", self.model_type.display_name())
    }

    fn is_apple_silicon() -> bool {
        cfg!(target_arch = "aarch64") && cfg!(target_os = "macos")
    }
}

unsafe impl Send for LocalLlmEngine {}
unsafe impl Sync for LocalLlmEngine {}

