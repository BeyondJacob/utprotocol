/// PDF processing and document chunking for RAG

use anyhow::{Context, Result};
use std::path::Path;

/// Configuration for text chunking
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Target chunk size in characters
    pub chunk_size: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1000,  // ~200-250 tokens
            overlap: 100,      // ~20-25 tokens overlap
        }
    }
}

/// Extract text from a PDF file
#[allow(dead_code)]
pub fn extract_pdf_text<P: AsRef<Path>>(pdf_path: P) -> Result<String> {
    let bytes = std::fs::read(pdf_path.as_ref())
        .context("Failed to read PDF file")?;

    extract_pdf_text_from_bytes(&bytes)
}

/// Extract text from PDF bytes
pub fn extract_pdf_text_from_bytes(pdf_bytes: &[u8]) -> Result<String> {
    let out = pdf_extract::extract_text_from_mem(pdf_bytes)
        .context("Failed to extract text from PDF")?;

    Ok(out)
}

/// Split text into overlapping chunks
pub fn chunk_text(text: &str, config: &ChunkConfig) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let total_chars = chars.len();

    let mut start = 0;

    while start < total_chars {
        // Calculate end position
        let end = (start + config.chunk_size).min(total_chars);

        // Extract chunk
        let chunk: String = chars[start..end].iter().collect();

        // Only add non-empty chunks
        let trimmed = chunk.trim();
        if !trimmed.is_empty() {
            chunks.push(trimmed.to_string());
        }

        // Move start position with overlap
        if end >= total_chars {
            break;
        }

        start += config.chunk_size - config.overlap;
    }

    chunks
}

/// Split text into semantic chunks (paragraph-based)
#[allow(dead_code)]
pub fn chunk_text_semantic(text: &str, max_chunk_size: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    // Split by paragraphs (double newlines)
    for paragraph in text.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        // If adding this paragraph would exceed max size, start new chunk
        if !current_chunk.is_empty() && current_chunk.len() + trimmed.len() > max_chunk_size {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = String::new();
        }

        // Add paragraph to current chunk
        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
        }
        current_chunk.push_str(trimmed);

        // If single paragraph exceeds max size, split by sentences
        if current_chunk.len() > max_chunk_size {
            for sentence in current_chunk.split(". ") {
                if sentence.trim().is_empty() {
                    continue;
                }

                chunks.push(sentence.trim().to_string());
            }
            current_chunk = String::new();
        }
    }

    // Add remaining chunk
    if !current_chunk.trim().is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}

/// Estimate token count from character count (rough approximation)
#[allow(dead_code)]
pub fn estimate_tokens(char_count: usize) -> usize {
    // Rough estimate: 1 token ≈ 4 characters for English text
    (char_count / 4).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_text_basic() {
        let text = "This is a test. " .repeat(100); // 1600 characters
        let config = ChunkConfig {
            chunk_size: 500,
            overlap: 50,
        };

        let chunks = chunk_text(&text, &config);

        // Should create multiple chunks
        assert!(chunks.len() > 1);

        // Each chunk should be approximately chunk_size
        for chunk in &chunks[..chunks.len() - 1] {
            assert!(chunk.len() <= config.chunk_size + 10); // Allow small variance
        }
    }

    #[test]
    fn test_chunk_text_overlap() {
        let text = "ABCDEFGHIJ".repeat(100); // 1000 characters
        let config = ChunkConfig {
            chunk_size: 300,
            overlap: 100,
        };

        let chunks = chunk_text(&text, &config);

        // Should have overlap between chunks
        assert!(chunks.len() > 1);

        // Check overlap exists (approximate check)
        if chunks.len() >= 2 {
            let chunk1_end = &chunks[0][chunks[0].len().saturating_sub(50)..];
            let chunk2_start = &chunks[1][..50.min(chunks[1].len())];

            // Some characters should appear in both chunks
            let has_overlap = chunk1_end.chars().any(|c| chunk2_start.contains(c));
            assert!(has_overlap, "Chunks should have overlap");
        }
    }

    #[test]
    fn test_chunk_text_empty() {
        let config = ChunkConfig::default();
        let chunks = chunk_text("", &config);
        assert_eq!(chunks.len(), 0);
    }

    #[test]
    fn test_chunk_text_small() {
        let text = "Small text.";
        let config = ChunkConfig::default();
        let chunks = chunk_text(text, &config);

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Small text.");
    }

    #[test]
    fn test_chunk_semantic() {
        let text = "Paragraph 1.\n\nParagraph 2.\n\nParagraph 3.";
        let chunks = chunk_text_semantic(text, 50);

        // Should split by paragraphs
        assert!(chunks.len() >= 3);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(400), 100);
        assert_eq!(estimate_tokens(1000), 250);
        assert_eq!(estimate_tokens(4000), 1000);
    }
}
