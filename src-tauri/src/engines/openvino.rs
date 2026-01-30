use std::path::Path;

use openvino::{Core, CompiledModel, DeviceType};

use crate::engines::{EngineError, SpeechEngine, Vocabulary};
use crate::types::TranscriptionResult;

pub struct OpenVINOEngine {
    #[allow(dead_code)]
    core: Core,
    #[allow(dead_code)]
    mel_model: CompiledModel,
    #[allow(dead_code)]
    encoder_model: CompiledModel,
    #[allow(dead_code)]
    decoder_model: CompiledModel,
    #[allow(dead_code)]
    joint_model: CompiledModel,
    vocabulary: Vocabulary,
    language: String,
}

// Implement Send + Sync manually since CompiledModel might not be Sync
unsafe impl Send for OpenVINOEngine {}
unsafe impl Sync for OpenVINOEngine {}

impl OpenVINOEngine {
    pub fn new(resources_path: &Path, language: &str) -> Result<Self, EngineError> {
        log::info!("Initializing OpenVINO engine from {:?}", resources_path);

        // Configurer le chemin des plugins OpenVINO
        let openvino_path = resources_path.join("openvino");
        if openvino_path.exists() {
            std::env::set_var("OPENVINO_LIB_PATHS", &openvino_path);
            log::info!("Set OPENVINO_LIB_PATHS to {:?}", openvino_path);
        }

        // Initialiser OpenVINO Core
        let mut core = Core::new()
            .map_err(|e| EngineError::OpenVINOInitFailed(format!("{:?}", e)))?;

        log::info!("OpenVINO Core initialized");

        let models_path = resources_path.join("models");

        // Charger les 4 modèles
        let mel_model = Self::load_model(&mut core, &models_path, "parakeet_melspectogram")?;
        let encoder_model = Self::load_model(&mut core, &models_path, "parakeet_encoder")?;
        let decoder_model = Self::load_model(&mut core, &models_path, "parakeet_decoder")?;
        let joint_model = Self::load_model(&mut core, &models_path, "parakeet_joint")?;

        // Charger le vocabulaire
        let vocabulary = Vocabulary::load(&models_path.join("parakeet_v3_vocab.json"))?;

        log::info!("OpenVINO engine initialized successfully with {} vocab tokens", vocabulary.vocab_size());

        Ok(Self {
            core,
            mel_model,
            encoder_model,
            decoder_model,
            joint_model,
            vocabulary,
            language: language.to_string(),
        })
    }

    fn load_model(core: &mut Core, models_path: &Path, model_name: &str) -> Result<CompiledModel, EngineError> {
        let xml_path = models_path.join(format!("{}.xml", model_name));
        let bin_path = models_path.join(format!("{}.bin", model_name));

        log::info!("Loading model: {}", model_name);

        if !xml_path.exists() {
            return Err(EngineError::ModelLoadFailed(format!("XML file not found: {:?}", xml_path)));
        }
        if !bin_path.exists() {
            return Err(EngineError::ModelLoadFailed(format!("BIN file not found: {:?}", bin_path)));
        }

        let model = core.read_model_from_file(
            xml_path.to_str().unwrap(),
            bin_path.to_str().unwrap(),
        ).map_err(|e| EngineError::ModelLoadFailed(format!("{}: {:?}", model_name, e)))?;

        let compiled = core.compile_model(&model, DeviceType::CPU)
            .map_err(|e| EngineError::ModelLoadFailed(format!("compile {}: {:?}", model_name, e)))?;

        log::info!("Model {} loaded and compiled", model_name);

        Ok(compiled)
    }

    /// Placeholder transcription - full TDT decoding requires understanding model I/O shapes
    fn run_inference(&self, audio: &[f32], sample_rate: u32) -> Result<String, EngineError> {
        // Pour l'instant, on retourne un placeholder car l'implémentation complète
        // du pipeline TDT nécessite de comprendre les shapes exactes des modèles

        let duration = audio.len() as f32 / sample_rate as f32;

        log::warn!("OpenVINO inference placeholder - full TDT implementation needed");
        log::info!("Audio duration: {:.2}s, sample_rate: {}Hz", duration, sample_rate);

        // TODO: Implémenter le vrai pipeline:
        // 1. mel_model: audio -> mel spectrogram
        // 2. encoder_model: mel -> encoder output
        // 3. decoder_model + joint_model: greedy TDT decoding
        // 4. vocabulary.decode(tokens)

        Ok(format!("[OpenVINO] Modeles charges - audio {:.1}s - implementation TDT en cours", duration))
    }
}

impl SpeechEngine for OpenVINOEngine {
    fn transcribe(&self, audio: &[f32], sample_rate: u32) -> Result<TranscriptionResult, String> {
        let start = std::time::Instant::now();

        // Vérifier le sample rate (Parakeet attend 16kHz)
        if sample_rate != 16000 {
            return Err(EngineError::InvalidSampleRate(sample_rate).into());
        }

        let duration_seconds = audio.len() as f32 / sample_rate as f32;

        if duration_seconds < 0.5 {
            return Err(EngineError::AudioTooShort.into());
        }

        // Exécuter l'inférence
        let text = self.run_inference(audio, sample_rate)
            .map_err(|e| e.to_string())?;

        let processing_time_ms = start.elapsed().as_millis() as u64;

        log::info!("Transcription completed: {} chars in {}ms", text.len(), processing_time_ms);

        Ok(TranscriptionResult {
            text,
            confidence: 0.95,
            duration_seconds,
            processing_time_ms,
            detected_language: Some(self.language.clone()),
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    fn name(&self) -> &str {
        "OpenVINO-Parakeet"
    }
}
