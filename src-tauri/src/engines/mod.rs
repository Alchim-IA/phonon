pub mod error;
pub mod openvino;
pub mod traits;
pub mod vocabulary;

pub use error::EngineError;
pub use openvino::OpenVINOEngine;
pub use traits::SpeechEngine;
pub use vocabulary::Vocabulary;
