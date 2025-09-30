use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// This is the protocol wrapper with compression metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct Frame {
    pub payload: Vec<u8>,
    pub source: String,
    pub timestamp: u64,
    pub compression_ratio: f64,
}

impl Frame {
    pub fn new(payload: Vec<u8>, source: String) -> Self {
        Self {
            payload,
            source,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            compression_ratio: 1.0,
        }
    }
}
