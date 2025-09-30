use crate::protocol::Frame;
use anyhow::Result;

/// Encode a frame to bytes for transmission
pub fn encode(frame: &Frame) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(frame)?)
}

/// Decode bytes to a frame
pub fn decode(data: &[u8]) -> Result<Frame> {
    Ok(serde_json::from_slice(data)?)
}