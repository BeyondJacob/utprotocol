use super::protocol::{CompressedEmbedding, Precision, QuantizedData};
use half::f16;

/// Embedding compressor with support for multiple quantization levels
pub struct Compressor;

impl Compressor {
    pub fn new() -> Self {
        Self
    }

    /// Compress an embedding to the specified precision
    pub fn compress(&self, embedding: &[f32], precision: Precision) -> CompressedEmbedding {
        let dimension = embedding.len() as u16;

        match precision {
            Precision::F32 => {
                // No compression - baseline
                CompressedEmbedding {
                    dimension,
                    precision: Precision::F32,
                    scale: 1.0,
                    data: QuantizedData::F32(embedding.to_vec()),
                }
            }

            Precision::F16 => {
                // 2x compression using half-precision floats
                let f16_data: Vec<f16> = embedding.iter().map(|&x| f16::from_f32(x)).collect();

                CompressedEmbedding {
                    dimension,
                    precision: Precision::F16,
                    scale: 1.0,
                    data: QuantizedData::F16(f16_data),
                }
            }

            Precision::Int8 => {
                // 4x compression with dynamic scaling
                // Find max absolute value for scaling
                let max_abs = embedding
                    .iter()
                    .map(|&x| x.abs())
                    .fold(0.0f32, f32::max);

                let scale = if max_abs > 0.0 {
                    max_abs / 127.0
                } else {
                    1.0
                };

                let quantized: Vec<i8> = embedding
                    .iter()
                    .map(|&x| {
                        let scaled = x / scale;
                        scaled.round().clamp(-128.0, 127.0) as i8
                    })
                    .collect();

                CompressedEmbedding {
                    dimension,
                    precision: Precision::Int8,
                    scale,
                    data: QuantizedData::Int8 {
                        scale,
                        data: quantized,
                    },
                }
            }

            Precision::Int4 => {
                // 8x compression with nibble packing
                // Find max absolute value for scaling
                let max_abs = embedding
                    .iter()
                    .map(|&x| x.abs())
                    .fold(0.0f32, f32::max);

                let scale = if max_abs > 0.0 {
                    max_abs / 7.0 // 4-bit signed: -8 to 7
                } else {
                    1.0
                };

                // Quantize to 4-bit values
                let quantized_4bit: Vec<i8> = embedding
                    .iter()
                    .map(|&x| {
                        let scaled = x / scale;
                        scaled.round().clamp(-8.0, 7.0) as i8
                    })
                    .collect();

                // Pack two 4-bit values into one byte (nibbles)
                let mut packed = Vec::with_capacity((quantized_4bit.len() + 1) / 2);

                for chunk in quantized_4bit.chunks(2) {
                    let high = (chunk[0] & 0x0F) as u8;
                    let low = if chunk.len() > 1 {
                        (chunk[1] & 0x0F) as u8
                    } else {
                        0
                    };

                    // Pack high nibble in upper 4 bits, low nibble in lower 4 bits
                    packed.push((high << 4) | low);
                }

                CompressedEmbedding {
                    dimension,
                    precision: Precision::Int4,
                    scale,
                    data: QuantizedData::Int4 {
                        scale,
                        data: packed,
                    },
                }
            }
        }
    }

    /// Decompress a compressed embedding back to f32
    #[cfg(test)]
    pub fn decompress(&self, compressed: &CompressedEmbedding) -> Vec<f32> {
        match &compressed.data {
            QuantizedData::F32(data) => data.clone(),

            QuantizedData::F16(data) => data.iter().map(|&x| x.to_f32()).collect(),

            QuantizedData::Int8 { scale, data } => {
                data.iter().map(|&x| (x as f32) * scale).collect()
            }

            QuantizedData::Int4 { scale, data } => {
                let mut result = Vec::with_capacity(compressed.dimension as usize);

                for &byte in data {
                    // Extract high nibble (upper 4 bits)
                    let high = ((byte >> 4) & 0x0F) as i8;
                    // Sign extend if necessary
                    let high = if high >= 8 { high - 16 } else { high };
                    result.push((high as f32) * scale);

                    // Extract low nibble (lower 4 bits)
                    if result.len() < compressed.dimension as usize {
                        let low = (byte & 0x0F) as i8;
                        // Sign extend if necessary
                        let low = if low >= 8 { low - 16 } else { low };
                        result.push((low as f32) * scale);
                    }
                }

                // Truncate to exact dimension if needed
                result.truncate(compressed.dimension as usize);
                result
            }
        }
    }

    /// Calculate compression ratio
    pub fn calculate_compression_ratio(&self, original_size: usize, compressed_size: usize) -> f32 {
        if compressed_size == 0 {
            return 1.0;
        }
        original_size as f32 / compressed_size as f32
    }

    /// Get compressed size in bytes for a given precision and dimension
    pub fn get_compressed_size(&self, dimension: u16, precision: Precision) -> usize {
        match precision {
            Precision::F32 => (dimension as usize) * 4, // 4 bytes per f32
            Precision::F16 => (dimension as usize) * 2, // 2 bytes per f16
            Precision::Int8 => {
                (dimension as usize) + std::mem::size_of::<f32>() // data + scale
            }
            Precision::Int4 => {
                ((dimension as usize) + 1) / 2 + std::mem::size_of::<f32>() // packed data + scale
            }
        }
    }
}

impl Default for Compressor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f32_compression_round_trip() {
        let compressor = Compressor::default();
        let original = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let compressed = compressor.compress(&original, Precision::F32);
        let decompressed = compressor.decompress(&compressed);

        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_f16_compression_round_trip() {
        let compressor = Compressor::default();
        let original = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let compressed = compressor.compress(&original, Precision::F16);
        let decompressed = compressor.decompress(&compressed);

        // Allow small epsilon for f16 precision loss
        for (a, b) in original.iter().zip(decompressed.iter()) {
            assert!((a - b).abs() < 0.01, "Values differ: {} vs {}", a, b);
        }
    }

    #[test]
    fn test_int8_compression_round_trip() {
        let compressor = Compressor::default();
        let original = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let compressed = compressor.compress(&original, Precision::Int8);
        let decompressed = compressor.decompress(&compressed);

        // Allow epsilon for quantization error
        for (a, b) in original.iter().zip(decompressed.iter()) {
            assert!((a - b).abs() < 0.1, "Values differ: {} vs {}", a, b);
        }
    }

    #[test]
    fn test_int4_compression_round_trip() {
        let compressor = Compressor::default();
        let original = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let compressed = compressor.compress(&original, Precision::Int4);
        let decompressed = compressor.decompress(&compressed);

        // Allow larger epsilon for 4-bit quantization
        for (a, b) in original.iter().zip(decompressed.iter()) {
            assert!((a - b).abs() < 0.5, "Values differ: {} vs {}", a, b);
        }
    }

    #[test]
    fn test_compression_sizes() {
        let compressor = Compressor::default();
        let dimension = 768;

        let f32_size = compressor.get_compressed_size(dimension, Precision::F32);
        let f16_size = compressor.get_compressed_size(dimension, Precision::F16);
        let int8_size = compressor.get_compressed_size(dimension, Precision::Int8);
        let int4_size = compressor.get_compressed_size(dimension, Precision::Int4);

        assert_eq!(f32_size, 768 * 4); // 3072 bytes
        assert_eq!(f16_size, 768 * 2); // 1536 bytes
        assert!(int8_size < f16_size);
        assert!(int4_size < int8_size);
    }
}
