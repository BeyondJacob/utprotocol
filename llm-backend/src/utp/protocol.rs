use serde::{Deserialize, Serialize};

/// Precision levels for embedding compression
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precision {
    F32,  // Baseline, no compression
    F16,  // 2x compression
    Int8, // 4x compression
    Int4, // 8x compression
}

impl Precision {
    pub fn as_str(&self) -> &str {
        match self {
            Precision::F32 => "F32",
            Precision::F16 => "F16",
            Precision::Int8 => "Int8",
            Precision::Int4 => "Int4",
        }
    }
}

/// UTP Protocol Header
#[derive(Debug, Clone)]
#[allow(dead_code)] // Reserved for future protocol implementation
pub struct UtpHeader {
    pub version: u8,
    pub precision: Precision,
    pub dimension: u16,
    pub payload_size: u32,
    pub timestamp: u64,
    pub checksum: u32,
}

/// Quantized data variants for different precision levels
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields accessed via pattern matching in compression module
pub enum QuantizedData {
    F32(Vec<f32>),                    // Baseline, no compression
    F16(Vec<half::f16>),              // 2x compression
    Int8 { scale: f32, data: Vec<i8> }, // 4x compression
    Int4 { scale: f32, data: Vec<u8> }, // 8x compression (nibble-packed)
}

/// Compressed embedding representation
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields accessed in compression and middleware modules
pub struct CompressedEmbedding {
    pub dimension: u16,
    pub precision: Precision,
    pub scale: f32,
    pub data: QuantizedData,
}

/// Trace step for visualizing data flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceStep {
    pub stage: String,
    pub size_bytes: usize,
    pub duration_us: u64,
    pub description: String,
}

/// Metadata for tracking UTP performance (stored as JSON in database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtpMetadata {
    pub used_utp: bool,
    pub cache_hit: bool,
    pub compression_ratio: f32,
    pub latency_us: u64,
    pub original_size: usize,
    pub compressed_size: usize,
    pub precision_used: String,
    pub trace: Vec<TraceStep>,
}

impl UtpMetadata {
    pub fn traditional(latency_us: u64, original_size: usize) -> Self {
        let trace = vec![
            TraceStep {
                stage: "Query".to_string(),
                size_bytes: original_size,
                duration_us: 0,
                description: format!("Input query ({}B)", original_size),
            },
            TraceStep {
                stage: "LLM".to_string(),
                size_bytes: original_size,
                duration_us: latency_us,
                description: format!("LLM API call ({:.2}ms)", latency_us as f64 / 1000.0),
            },
            TraceStep {
                stage: "Response".to_string(),
                size_bytes: 0,
                duration_us: 0,
                description: "Generated response".to_string(),
            },
        ];

        Self {
            used_utp: false,
            cache_hit: false,
            compression_ratio: 1.0,
            latency_us,
            original_size,
            compressed_size: 0,
            precision_used: "None".to_string(),
            trace,
        }
    }

    pub fn utp(
        cache_hit: bool,
        compression_ratio: f32,
        latency_us: u64,
        original_size: usize,
        compressed_size: usize,
        precision: Precision,
        compression_time_us: u64,
        cache_lookup_time_us: u64,
        llm_time_us: u64,
    ) -> Self {
        let mut trace = vec![
            TraceStep {
                stage: "Query".to_string(),
                size_bytes: original_size,
                duration_us: 0,
                description: format!("Input query ({}B)", original_size),
            },
            TraceStep {
                stage: "Compress".to_string(),
                size_bytes: compressed_size,
                duration_us: compression_time_us,
                description: format!("{}→{}B ({:.1}x, {:.2}ms)",
                    original_size, compressed_size, compression_ratio,
                    compression_time_us as f64 / 1000.0),
            },
        ];

        if cache_hit {
            trace.push(TraceStep {
                stage: "Cache".to_string(),
                size_bytes: compressed_size,
                duration_us: cache_lookup_time_us,
                description: format!("⚡ Cache Hit ({:.3}ms)", cache_lookup_time_us as f64 / 1000.0),
            });
        } else {
            trace.push(TraceStep {
                stage: "Cache".to_string(),
                size_bytes: compressed_size,
                duration_us: cache_lookup_time_us,
                description: format!("Cache Miss ({:.3}ms)", cache_lookup_time_us as f64 / 1000.0),
            });
            trace.push(TraceStep {
                stage: "LLM".to_string(),
                size_bytes: compressed_size,
                duration_us: llm_time_us,
                description: format!("LLM API call ({:.2}ms)", llm_time_us as f64 / 1000.0),
            });
        }

        trace.push(TraceStep {
            stage: "Response".to_string(),
            size_bytes: 0,
            duration_us: 0,
            description: "Generated response".to_string(),
        });

        Self {
            used_utp: true,
            cache_hit,
            compression_ratio,
            latency_us,
            original_size,
            compressed_size,
            precision_used: precision.as_str().to_string(),
            trace,
        }
    }
}
