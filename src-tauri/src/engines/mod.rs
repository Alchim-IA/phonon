pub mod error;
pub mod model_manager;
pub mod openvino;
pub mod traits;
pub mod vocabulary;

pub use error::EngineError;
pub use model_manager::ModelManager;
pub use openvino::OpenVINOEngine;
pub use traits::SpeechEngine;
pub use vocabulary::Vocabulary;
