//! Univresal Thought Protocol Core Library

pub mod compression;
pub mod protocol;
pub mod codec;

pub use compression::Compressor;
pub use protocol::{Frame, Message};
pub use codec::{encode, decode};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_flow() {
        // Create a message
        let message = Message {
            role: "user".to_string(),
            content: "Hello from agent".to_string(),
        };
        
        // Serialize it
        let json = serde_json::to_vec(&message).unwrap();
        
        // Compress it
        let compressor = Compressor::new();
        let compressed = compressor.compress(&json).unwrap();
        
        // Wrap in frame
        let frame = Frame::new(compressed, "agent-1".to_string());
        
        // Encode frame`
        let encoded = encode(&frame).unwrap();
        
        // Decode frame
        let decoded_frame = decode(&encoded).unwrap();
        
        // Verify
        assert_eq!(decoded_frame.source, "agent-1");
        assert!(decoded_frame.timestamp > 0);
    }
}