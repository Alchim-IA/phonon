use std::num::NonZeroU32;
use std::path::Path;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;

use crate::types::LocalLlmModel;

/// Moteur LLM local via llama.cpp (GGUF)
pub struct LocalLlmEngine {
    backend: LlamaBackend,
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

        let backend = LlamaBackend::init()
            .map_err(|e| format!("Failed to initialize llama backend: {}", e))?;

        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(1000); // Offload all layers to GPU (Metal on macOS)

        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| format!("Failed to load model from {:?}: {}", model_path, e))?;

        log::info!(
            "Local LLM loaded: {} params, vocab size {}",
            model.n_params(),
            model.n_vocab()
        );

        Ok(Self {
            backend,
            model,
            model_type,
        })
    }

    /// Generates a summary of the given text
    pub fn summarize(&self, text: &str) -> Result<String, String> {
        let prompt = self.model_type.format_prompt("", text);

        // Create a fresh context for this inference
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(2048).unwrap()));

        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| format!("Failed to create inference context: {}", e))?;

        // Tokenize the prompt
        let tokens = self
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| format!("Failed to tokenize prompt: {}", e))?;

        if tokens.is_empty() {
            return Err("Prompt tokenized to zero tokens".to_string());
        }

        log::info!("Prompt tokenized to {} tokens", tokens.len());

        // Create batch large enough for the prompt
        let batch_capacity = (tokens.len() + 1).max(512);
        let mut batch = LlamaBatch::new(batch_capacity, 1);

        for (i, token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            batch
                .add(*token, i as i32, &[0], is_last)
                .map_err(|e| format!("Failed to add token to batch: {}", e))?;
        }

        // Evaluate the prompt
        ctx.decode(&mut batch)
            .map_err(|e| format!("Failed to decode prompt: {}", e))?;

        // Set up sampling: temperature 0.3 for focused output
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(0.3),
            LlamaSampler::dist(42),
        ]);

        // UTF-8 decoder for token-to-text conversion
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        // Generate up to 512 output tokens
        let max_output_tokens = 512;
        let mut output = String::new();
        let mut n_cur = tokens.len() as i32;

        for _ in 0..max_output_tokens {
            let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(new_token);

            // Stop at end-of-generation
            if self.model.is_eog_token(new_token) {
                break;
            }

            // Decode token to text
            match self.model.token_to_piece(new_token, &mut decoder, true, None) {
                Ok(piece) => output.push_str(&piece),
                Err(_) => {} // Skip invalid tokens
            }

            // Prepare next iteration
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

        log::info!("Local LLM generated {} chars", result.len());
        Ok(result)
    }

    pub fn model_type(&self) -> LocalLlmModel {
        self.model_type
    }

    pub fn display_name(&self) -> String {
        format!("Local LLM ({})", self.model_type.display_name())
    }
}

unsafe impl Send for LocalLlmEngine {}
unsafe impl Sync for LocalLlmEngine {}
