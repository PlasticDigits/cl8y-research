//! Hashing embedder for fixture tests and credential-free local search.
//!
//! Live Replicate embeddings are optional and must use create-once / poll-by-id
//! with the same dimension as [`crate::invariants::EMBEDDING_DIM`].

use crate::invariants::EMBEDDING_DIM;
use sha2::{Digest, Sha256};

pub trait Embedder: Send + Sync {
    fn embed(&self, text: &str) -> Vec<f32>;
    fn dim(&self) -> usize {
        EMBEDDING_DIM
    }
}

/// Deterministic bag-of-hashed-tokens embedder. No network, no secrets.
#[derive(Debug, Default, Clone)]
pub struct HashingEmbedder;

impl Embedder for HashingEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut v = vec![0.0f32; EMBEDDING_DIM];
        for token in tokenize(text) {
            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            let hash = hasher.finalize();
            let idx = u16::from_be_bytes([hash[0], hash[1]]) as usize % EMBEDDING_DIM;
            v[idx] += 1.0;
        }
        l2_normalize(&mut v);
        v
    }
}

pub fn tokenize(text: &str) -> Vec<String> {
    text.to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}

pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

fn l2_normalize(v: &mut [f32]) {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if n > 0.0 {
        for x in v.iter_mut() {
            *x /= n;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_text_ranks_higher() {
        let e = HashingEmbedder;
        let a = e.embed("CL8Y DEX overview volume as of 2026-08-31");
        let b = e.embed("DEX volume overview for CL8Y");
        let c = e.embed("unrelated cooking recipes and pasta");
        assert!(cosine(&a, &b) > cosine(&a, &c));
    }
}
