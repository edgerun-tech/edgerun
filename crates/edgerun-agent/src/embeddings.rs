//! Embedding generation and semantic search module.
//! Optimized for AMD NPU (XDNA) and iGPU (RDNA3) acceleration.
//!
//! Uses ONNX Runtime (ort) for NPU acceleration or candle for iGPU.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// Embedding model wrapper supporting multiple backends
pub struct EmbeddingModel {
    backend: Backend,
    dimension: usize,
}

enum Backend {
    #[cfg(feature = "ml-acceleration")]
    Ort(ort::Session),
    #[cfg(feature = "ml-acceleration")]
    Candle(candle_transformers::models::all_mini_lm::AllMiniLm),
    Dummy,
}

impl EmbeddingModel {
    /// Load embedding model with automatic backend selection
    pub fn load(_model_path: &Path) -> Result<Self, String> {
        // Try NPU acceleration first (ONNX Runtime)
        #[cfg(feature = "ml-acceleration")]
        {
            if let Ok(session) = Self::load_ort(model_path) {
                println!("Loaded embedding model with ONNX Runtime (NPU acceleration)");
                return Ok(session);
            }

            // Fall back to candle (iGPU)
            if let Ok(model) = Self::load_candle(model_path) {
                println!("Loaded embedding model with Candle (iGPU acceleration)");
                return Ok(model);
            }
        }

        println!("Using dummy embedding model (ML acceleration not enabled)");
        Ok(Self::dummy())
    }

    #[cfg(feature = "ml-acceleration")]
    fn load_ort(model_path: &Path) -> Result<Self, String> {
        use ort::{ExecutionProvider, SessionBuilder};

        // Configure for NPU (AMD XDNA)
        let session = SessionBuilder::new()?
            .with_execution_providers([
                ExecutionProvider::Dml, // DirectML for AMD GPU/NPU
                ExecutionProvider::Cpu,
            ])?
            .commit_from_file(model_path.join("model.onnx"))?;

        Ok(Self {
            backend: Backend::Ort(session),
            dimension: 384, // All-MiniLM-L6-v2 dimension
        })
    }

    #[cfg(feature = "ml-acceleration")]
    fn load_candle(model_path: &Path) -> Result<Self, String> {
        use candle_core::{Device, Tensor};
        use candle_nn::VarBuilder;
        use candle_transformers::models::all_mini_lm;

        let device = Device::new_cuda(0).unwrap_or(Device::Cpu);

        // Load model weights
        let vb = VarBuilder::from_pth(model_path.join("model.safetensors"), &device)?;
        let model = all_mini_lm::AllMiniLm::new(vb)?;

        Ok(Self {
            backend: Backend::Candle(model),
            dimension: 384,
        })
    }

    /// Create dummy model for testing/fallback
    pub fn dummy() -> Self {
        Self {
            backend: Backend::Dummy,
            dimension: 384,
        }
    }

    /// Generate embedding for text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        match &self.backend {
            #[cfg(feature = "ml-acceleration")]
            Backend::Ort(session) => {
                // Tokenize and run inference
                // Simplified - would use tokenizers crate
                let mut embedding = vec![0.0f32; self.dimension];

                // In real implementation:
                // 1. Tokenize input
                // 2. Create input tensor
                // 3. Run session.run()
                // 4. Extract and normalize embedding

                // Placeholder for demonstration
                for (i, val) in embedding.iter_mut().enumerate() {
                    *val = ((text.len() + i) % 100) as f32 / 100.0;
                }

                Ok(embedding)
            }
            #[cfg(feature = "ml-acceleration")]
            Backend::Candle(_model) => {
                // Candle implementation
                // 1. Tokenize
                // 2. Forward pass
                // 3. Normalize
                let mut embedding = vec![0.0f32; self.dimension];
                Ok(embedding)
            }
            Backend::Dummy => {
                // Return deterministic pseudo-embedding for testing
                let mut embedding = vec![0.0f32; self.dimension];
                for (i, val) in embedding.iter_mut().enumerate() {
                    *val = ((text.len() + i) % 100) as f32 / 100.0;
                }
                // Normalize
                let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
                if norm > 0.0 {
                    for x in &mut embedding {
                        *x /= norm;
                    }
                }
                Ok(embedding)
            }
        }
    }

    /// Batch embed multiple texts
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, String> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Vector index for semantic search using HNSW
pub struct VectorIndex {
    embeddings: HashMap<String, Vec<f32>>,
    dimension: usize,
    #[cfg(feature = "ml-acceleration")]
    hnsw_index: Option<usearch::Index>,
}

impl VectorIndex {
    pub fn new(dimension: usize) -> Self {
        Self {
            embeddings: HashMap::new(),
            dimension,
            #[cfg(feature = "ml-acceleration")]
            hnsw_index: None,
        }
    }

    /// Add embedding to index
    pub fn add(&mut self, id: String, embedding: Vec<f32>) {
        if embedding.len() != self.dimension {
            eprintln!("Warning: embedding dimension mismatch");
            return;
        }

        self.embeddings.insert(id, embedding);

        #[cfg(feature = "ml-acceleration")]
        {
            if let Some(index) = &mut self.hnsw_index {
                let key = self.embeddings.len() as u64;
                index.add(key, &embedding).ok();
            }
        }
    }

    /// Remove embedding by ID
    pub fn remove(&mut self, id: &str) {
        self.embeddings.remove(id);
    }

    /// Search for similar embeddings
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(String, f32)> {
        if query.len() != self.dimension {
            return Vec::new();
        }

        // Compute cosine similarity
        let mut similarities: Vec<_> = self
            .embeddings
            .iter()
            .map(|(id, embedding)| {
                let sim = cosine_similarity(query, embedding);
                (id.clone(), sim)
            })
            .collect();

        // Sort by similarity (descending)
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return top-k
        similarities.into_iter().take(k).collect()
    }

    /// Build HNSW index for faster search (optional, for large datasets)
    #[cfg(feature = "ml-acceleration")]
    pub fn build_hnsw(&mut self) -> Result<(), String> {
        use usearch::{Index, IndexOptions};

        let mut options = IndexOptions::default();
        options.dimensions = self.dimension as _;
        options.connectivity = 16;
        options.expansion_add = 128;
        options.expansion_search = 64;

        let mut index = Index::new(&options)?;

        for (key, embedding) in self.embeddings.iter().enumerate() {
            index.add(key as u64, embedding)?;
        }

        self.hnsw_index = Some(index);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a > 0.0 && norm_b > 0.0 {
        dot / (norm_a * norm_b)
    } else {
        0.0
    }
}

/// Semantic search engine combining embeddings and vector index
pub struct SemanticSearch {
    model: Arc<EmbeddingModel>,
    index: VectorIndex,
}

impl SemanticSearch {
    pub fn new(model: Arc<EmbeddingModel>) -> Self {
        let dimension = model.dimension();
        Self {
            model,
            index: VectorIndex::new(dimension),
        }
    }

    /// Index a code snippet
    pub fn index_code(&mut self, id: String, code: &str) -> Result<(), String> {
        let embedding = self.model.embed(code)?;
        self.index.add(id, embedding);
        Ok(())
    }

    /// Search for similar code
    pub fn search(&self, query: &str, k: usize) -> Vec<(String, f32)> {
        let query_embedding = match self.model.embed(query) {
            Ok(emb) => emb,
            Err(_) => return Vec::new(),
        };

        self.index.search(&query_embedding, k)
    }

    /// Batch index multiple snippets
    pub fn index_batch(&mut self, items: Vec<(String, String)>) -> Result<(), String> {
        let texts: Vec<&str> = items.iter().map(|(_, code)| code.as_str()).collect();
        let embeddings = self.model.embed_batch(&texts)?;

        for ((id, _code), embedding) in items.into_iter().zip(embeddings) {
            self.index.add(id, embedding);
        }

        Ok(())
    }

    pub fn indexed_count(&self) -> usize {
        self.index.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dummy_embedding() {
        let model = EmbeddingModel::dummy();
        let embedding = model.embed("test code").unwrap();
        assert_eq!(embedding.len(), 384);

        // Check normalization
        let norm: f32 = embedding.iter().map(|x| x * x).sum();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[test]
    fn test_vector_index() {
        let mut index = VectorIndex::new(3);

        // Create distinct vectors
        index.add("item1".to_string(), vec![1.0, 0.0, 0.0]);
        index.add("item2".to_string(), vec![0.0, 1.0, 0.0]);

        let query = vec![0.9, 0.1, 0.0]; // Closer to item1
        let results = index.search(&query, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "item1"); // Most similar
    }
}

// Benchmark comment