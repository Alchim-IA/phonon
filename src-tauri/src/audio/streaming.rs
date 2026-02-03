use std::sync::{Arc, Mutex};

/// Configuration for streaming transcription
pub struct StreamingConfig {
    /// Chunk duration in seconds (default: 2.5s)
    pub chunk_duration_secs: f32,
    /// Overlap between chunks in seconds (helps with word boundaries)
    pub overlap_secs: f32,
    /// Sample rate of the audio
    pub sample_rate: u32,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_duration_secs: 2.5,
            overlap_secs: 0.5,
            sample_rate: 16000,
        }
    }
}

/// A streaming buffer that accumulates audio and provides chunks for transcription
pub struct StreamingBuffer {
    /// The accumulated audio samples
    buffer: Arc<Mutex<Vec<f32>>>,
    /// Configuration
    config: StreamingConfig,
    /// Number of samples already processed (for overlap handling)
    processed_samples: usize,
    /// Total accumulated text from all chunks
    accumulated_text: Arc<Mutex<String>>,
}

impl StreamingBuffer {
    pub fn new(config: StreamingConfig) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            config,
            processed_samples: 0,
            accumulated_text: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Returns a clone of the buffer Arc for use in audio capture callbacks
    pub fn buffer_handle(&self) -> Arc<Mutex<Vec<f32>>> {
        self.buffer.clone()
    }

    /// Returns the number of samples needed for one chunk
    fn chunk_samples(&self) -> usize {
        (self.config.chunk_duration_secs * self.config.sample_rate as f32) as usize
    }

    /// Returns the number of samples for overlap
    fn overlap_samples(&self) -> usize {
        (self.config.overlap_secs * self.config.sample_rate as f32) as usize
    }

    /// Check if there's enough audio for a new chunk
    pub fn has_chunk_available(&self) -> bool {
        let buffer = self.buffer.lock().unwrap();
        let available = buffer.len().saturating_sub(self.processed_samples);
        available >= self.chunk_samples()
    }

    /// Extract the next chunk of audio for transcription
    /// Returns None if not enough audio is available
    pub fn extract_chunk(&mut self) -> Option<Vec<f32>> {
        let buffer = self.buffer.lock().unwrap();
        let chunk_size = self.chunk_samples();
        let overlap = self.overlap_samples();

        // Start position includes overlap from previous chunk (except for first chunk)
        let start = if self.processed_samples > overlap {
            self.processed_samples - overlap
        } else {
            0
        };

        let end = start + chunk_size;

        if buffer.len() < end {
            return None;
        }

        let chunk = buffer[start..end].to_vec();

        // Update processed position (move forward by chunk minus overlap)
        self.processed_samples = end - overlap;

        Some(chunk)
    }

    /// Get all remaining audio (used when stopping recording)
    pub fn get_remaining(&self) -> Vec<f32> {
        let buffer = self.buffer.lock().unwrap();
        let overlap = self.overlap_samples();

        let start = if self.processed_samples > overlap {
            self.processed_samples - overlap
        } else {
            0
        };

        if start < buffer.len() {
            buffer[start..].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Get all audio accumulated so far
    pub fn get_all_audio(&self) -> Vec<f32> {
        self.buffer.lock().unwrap().clone()
    }

    /// Clear the buffer and reset state
    pub fn clear(&mut self) {
        self.buffer.lock().unwrap().clear();
        self.processed_samples = 0;
        self.accumulated_text.lock().unwrap().clear();
    }

    /// Append text from a chunk transcription
    pub fn append_text(&self, text: &str) {
        let mut accumulated = self.accumulated_text.lock().unwrap();
        if !accumulated.is_empty() && !text.is_empty() {
            accumulated.push(' ');
        }
        accumulated.push_str(text.trim());
    }

    /// Get the accumulated text so far
    pub fn get_accumulated_text(&self) -> String {
        self.accumulated_text.lock().unwrap().clone()
    }

    /// Get the current buffer length in samples
    pub fn buffer_len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }

    /// Get the current duration in seconds
    pub fn duration_secs(&self) -> f32 {
        self.buffer_len() as f32 / self.config.sample_rate as f32
    }
}

/// Result of a streaming chunk transcription
#[derive(Debug, Clone)]
pub struct StreamingChunk {
    /// The transcribed text for this chunk
    pub text: String,
    /// Whether this is the final chunk (recording stopped)
    pub is_final: bool,
    /// Duration of audio processed so far
    pub duration_seconds: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_streaming_buffer_chunk_extraction() {
        let config = StreamingConfig {
            chunk_duration_secs: 1.0,
            overlap_secs: 0.2,
            sample_rate: 16000,
        };

        let mut buffer = StreamingBuffer::new(config);
        let handle = buffer.buffer_handle();

        // Add 2 seconds of audio (32000 samples)
        {
            let mut buf = handle.lock().unwrap();
            buf.extend(vec![0.0f32; 32000]);
        }

        // Should be able to extract first chunk
        assert!(buffer.has_chunk_available());
        let chunk1 = buffer.extract_chunk();
        assert!(chunk1.is_some());
        assert_eq!(chunk1.unwrap().len(), 16000); // 1 second

        // Should be able to extract second chunk
        assert!(buffer.has_chunk_available());
        let chunk2 = buffer.extract_chunk();
        assert!(chunk2.is_some());
    }

    #[test]
    fn test_accumulated_text() {
        let config = StreamingConfig::default();
        let buffer = StreamingBuffer::new(config);

        buffer.append_text("Hello");
        buffer.append_text("world");
        buffer.append_text("test");

        assert_eq!(buffer.get_accumulated_text(), "Hello world test");
    }
}
