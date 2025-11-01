/// Semantic similarity utilities for UTP
///
/// This module provides functions for computing similarity between embeddings,
/// enabling semantic cache lookups beyond exact string matches.

/// Compute cosine similarity between two embeddings
///
/// Cosine similarity = (A · B) / (||A|| * ||B||)
/// Range: [-1, 1] where 1 = identical, 0 = orthogonal, -1 = opposite
///
/// # Arguments
/// * `a` - First embedding vector
/// * `b` - Second embedding vector
///
/// # Returns
/// Cosine similarity score in range [-1.0, 1.0]
///
/// # Panics
/// Panics if embeddings have different lengths
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Embeddings must have same dimension");

    if a.is_empty() {
        return 0.0;
    }

    // Compute dot product
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    // Compute magnitudes
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    // Handle zero vectors
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    // Compute cosine similarity
    dot_product / (magnitude_a * magnitude_b)
}

/// Compute L2 (Euclidean) distance between two embeddings
///
/// L2 distance = sqrt(sum((a_i - b_i)^2))
/// Range: [0, ∞) where 0 = identical, larger = more different
///
/// # Arguments
/// * `a` - First embedding vector
/// * `b` - Second embedding vector
///
/// # Returns
/// L2 distance score (lower is more similar)
///
/// # Panics
/// Panics if embeddings have different lengths
#[allow(dead_code)]
pub fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Embeddings must have same dimension");

    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum::<f32>()
        .sqrt()
}

/// Normalize an embedding vector to unit length
///
/// This is useful for cosine similarity computation, as normalized vectors
/// allow dot product to equal cosine similarity directly.
///
/// # Arguments
/// * `embedding` - Embedding vector to normalize
///
/// # Returns
/// Normalized embedding (unit vector)
#[allow(dead_code)]
pub fn normalize(embedding: &[f32]) -> Vec<f32> {
    let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude == 0.0 {
        return embedding.to_vec();
    }

    embedding.iter().map(|x| x / magnitude).collect()
}

/// Find the k most similar embeddings to a query
///
/// Uses cosine similarity for ranking. Returns indices and scores
/// of the top-k most similar embeddings.
///
/// # Arguments
/// * `query` - Query embedding vector
/// * `candidates` - Slice of candidate embedding vectors
/// * `k` - Number of top results to return
///
/// # Returns
/// Vector of (index, similarity_score) tuples, sorted by score (descending)
#[allow(dead_code)]
pub fn top_k_similar(query: &[f32], candidates: &[&[f32]], k: usize) -> Vec<(usize, f32)> {
    let mut similarities: Vec<(usize, f32)> = candidates
        .iter()
        .enumerate()
        .map(|(idx, candidate)| (idx, cosine_similarity(query, candidate)))
        .collect();

    // Sort by similarity score (descending)
    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    // Take top k
    similarities.truncate(k);

    similarities
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        let similarity = cosine_similarity(&a, &b);
        assert!((similarity - 1.0).abs() < 0.0001, "Identical vectors should have similarity 1.0");
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];

        let similarity = cosine_similarity(&a, &b);
        assert!((similarity - 0.0).abs() < 0.0001, "Orthogonal vectors should have similarity 0.0");
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];

        let similarity = cosine_similarity(&a, &b);
        assert!((similarity + 1.0).abs() < 0.0001, "Opposite vectors should have similarity -1.0");
    }

    #[test]
    fn test_cosine_similarity_similar() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.1, 2.1, 2.9];

        let similarity = cosine_similarity(&a, &b);
        assert!(similarity > 0.99, "Very similar vectors should have high similarity");
    }

    #[test]
    fn test_l2_distance_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];

        let distance = l2_distance(&a, &b);
        assert!(distance < 0.0001, "Identical vectors should have distance 0.0");
    }

    #[test]
    fn test_l2_distance_different() {
        let a = vec![0.0, 0.0, 0.0];
        let b = vec![3.0, 4.0, 0.0];

        let distance = l2_distance(&a, &b);
        assert!((distance - 5.0).abs() < 0.0001, "L2 distance should be 5.0 (3-4-5 triangle)");
    }

    #[test]
    fn test_normalize() {
        let embedding = vec![3.0, 4.0, 0.0];
        let normalized = normalize(&embedding);

        // Magnitude should be 1.0
        let magnitude: f32 = normalized.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.0001, "Normalized vector should have magnitude 1.0");

        // Values should be [0.6, 0.8, 0.0]
        assert!((normalized[0] - 0.6).abs() < 0.0001);
        assert!((normalized[1] - 0.8).abs() < 0.0001);
        assert!((normalized[2] - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_top_k_similar() {
        let query = vec![1.0, 0.0, 0.0];

        let candidates: Vec<Vec<f32>> = vec![
            vec![1.0, 0.0, 0.0],   // Perfect match
            vec![0.9, 0.1, 0.0],   // Very similar
            vec![0.0, 1.0, 0.0],   // Orthogonal
            vec![-1.0, 0.0, 0.0],  // Opposite
            vec![0.7, 0.7, 0.0],   // Somewhat similar
        ];

        let candidate_refs: Vec<&[f32]> = candidates.iter().map(|v| v.as_slice()).collect();

        let top_k = top_k_similar(&query, &candidate_refs, 3);

        // Should return indices 0, 1, 4 (in that order)
        assert_eq!(top_k.len(), 3);
        assert_eq!(top_k[0].0, 0); // Perfect match first
        assert!(top_k[0].1 > 0.99);

        assert_eq!(top_k[1].0, 1); // Very similar second
        assert!(top_k[1].1 > 0.99);

        // Third could be index 4 (0.707) or index 2 (0.0)
        assert!(top_k[2].1 >= 0.0);
    }

    #[test]
    fn test_zero_vector_handling() {
        let zero = vec![0.0, 0.0, 0.0];
        let normal = vec![1.0, 2.0, 3.0];

        // Should not panic or produce NaN
        let similarity = cosine_similarity(&zero, &normal);
        assert_eq!(similarity, 0.0);

        let distance = l2_distance(&zero, &normal);
        assert!(distance.is_finite());
    }

    #[test]
    #[should_panic(expected = "Embeddings must have same dimension")]
    fn test_dimension_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];

        cosine_similarity(&a, &b);
    }
}
