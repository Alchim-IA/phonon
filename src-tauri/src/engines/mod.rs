pub mod error;
pub mod model_manager;
pub mod parakeet;
pub mod parakeet_coreml;
pub mod traits;
pub mod vosk;
pub mod whisper;

pub use error::EngineError;
pub use model_manager::ModelManager;
pub use parakeet::{ParakeetEngine, ParakeetModelSize};
pub use parakeet_coreml::ParakeetCoreMLEngine;
pub use traits::SpeechEngine;
pub use vosk::VoskEngine;
pub use whisper::WhisperEngine;
