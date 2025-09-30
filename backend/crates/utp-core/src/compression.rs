use anyhow::Result;

pub struct Compressor {
    // TODO: What fields do we need?
    // Hint: maybe a compression level?
    _level: u8,
}

impl Compressor {
    pub fn new() -> Self {
        // TODO: Create and return a Compressor.
        Self {
            _level: 6,
        }
    }

    pub fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}