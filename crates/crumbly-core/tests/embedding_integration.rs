//! Integration tests for embedding model loading and generation
//!
//! These tests verify the concrete candle-based embedding implementation works correctly.
//! They are marked with `#[ignore]` because they:
//! - Download models from the internet (slow, network-dependent)
//! - Require significant disk space for model caching
//! - Take several seconds to run
//!
//! Run with: `cargo test --test embedding_integration -- --ignored`

use std::sync::Arc;

use crumbly_core::knowledge::search::embeddings::{
    EmbeddingModel, EmbeddingProvider, TieredModelCache, VectorOps, default_l2_dir,
};
use tempfile::TempDir;

fn create_resolver(l1_dir: &std::path::Path) -> Arc<TieredModelCache> {
    Arc::new(TieredModelCache::new(l1_dir, default_l2_dir()))
}

#[test]
#[ignore]
fn test_load_model_downloads_and_caches() {
    // Given A fresh cache directory
    let cache_dir = TempDir::new().unwrap();

    // When Loading the default model
    let model = EmbeddingModel::builder()
        .cache_resolver(create_resolver(cache_dir.path()))
        .build()
        .load();

    // Then Model loads successfully
    assert!(model.is_ok());

    // Then Cache directory contains model files
    let entries: Vec<_> = std::fs::read_dir(cache_dir.path())
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(!entries.is_empty());
}

#[test]
#[ignore]
fn test_embed_generates_correct_dimension() {
    // Given A loaded embedding model
    let cache_dir = TempDir::new().unwrap();
    let model = EmbeddingModel::builder()
        .cache_resolver(create_resolver(cache_dir.path()))
        .build()
        .load()
        .unwrap();

    // When Embedding a single text string
    let text = "Bottlerocket is a Linux-based operating system";
    let embedding = model.embed(text);

    // Then Returns embedding with correct dimension
    assert!(embedding.is_ok());
    let embedding = embedding.unwrap();
    assert_eq!(embedding.as_ref().len(), 384);
}

#[test]
#[ignore]
fn test_embed_batch_generates_multiple_embeddings() {
    // Given A loaded embedding model
    let cache_dir = TempDir::new().unwrap();
    let model = EmbeddingModel::builder()
        .cache_resolver(create_resolver(cache_dir.path()))
        .build()
        .load()
        .unwrap();

    // When Embedding multiple texts in a batch
    let texts = vec![
        "Bottlerocket is a Linux-based operating system".to_string(),
        "Kubernetes orchestrates containers".to_string(),
        "AWS provides cloud computing services".to_string(),
    ];
    let embeddings = model.embed_batch(texts.clone());

    // Then Returns correct number of embeddings
    assert!(embeddings.is_ok());
    let embeddings = embeddings.unwrap();
    assert_eq!(embeddings.len(), texts.len());

    // Then Each embedding has correct dimension
    for embedding in embeddings {
        assert_eq!(embedding.as_ref().len(), 384);
    }
}

#[test]
#[ignore]
fn test_embed_similar_texts_have_high_similarity() {
    // Given A loaded embedding model
    let cache_dir = TempDir::new().unwrap();
    let model = EmbeddingModel::builder()
        .cache_resolver(create_resolver(cache_dir.path()))
        .build()
        .load()
        .unwrap();

    // When Embedding two semantically similar texts
    let text1 = "Bottlerocket is a Linux-based operating system for containers";
    let text2 = "Bottlerocket is a container-focused Linux distribution";
    let embedding1 = model.embed(text1).unwrap();
    let embedding2 = model.embed(text2).unwrap();

    // Then Cosine similarity is high
    let similarity = embedding1.cosine_similarity(&embedding2);
    assert!(
        similarity > 0.7,
        "Expected similarity > 0.7, got {}",
        similarity
    );
}

#[test]
#[ignore]
fn test_embed_dissimilar_texts_have_low_similarity() {
    // Given A loaded embedding model
    let cache_dir = TempDir::new().unwrap();
    let model = EmbeddingModel::builder()
        .cache_resolver(create_resolver(cache_dir.path()))
        .build()
        .load()
        .unwrap();

    // When Embedding two semantically different texts
    let text1 = "Bottlerocket is a Linux-based operating system";
    let text2 = "The quick brown fox jumps over the lazy dog";
    let embedding1 = model.embed(text1).unwrap();
    let embedding2 = model.embed(text2).unwrap();

    // Then Cosine similarity is low
    let similarity = embedding1.cosine_similarity(&embedding2);
    assert!(
        similarity < 0.3,
        "Expected similarity < 0.3, got {}",
        similarity
    );
}

#[test]
#[ignore]
fn test_unsupported_model_name_fails() {
    // Given An unsupported model name
    let cache_dir = TempDir::new().unwrap();

    // When Attempting to load the model
    let result = EmbeddingModel::builder()
        .model_name("unsupported/model-name")
        .cache_resolver(create_resolver(cache_dir.path()))
        .build()
        .load();

    // Then Returns ModelDownloadFailed error
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Failed to download model from"));
}
