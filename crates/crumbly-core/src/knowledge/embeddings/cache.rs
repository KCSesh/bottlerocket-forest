//! Tiered model cache for embedding models.
//!
//! Provides L1 (per-index) and L2 (user-level) caching to avoid redundant
//! model downloads. The resolver checks L1 first, falls back to L2, and
//! downloads to L2 if neither has the model.

use std::path::{Path, PathBuf};

use hf_hub::{Repo, RepoType, api::sync::ApiBuilder};
use snafu::{IntoError, ResultExt};
use tracing::{debug, info};

use super::error::{EmbeddingError, embedding_error};

/// Resolves model cache locations for embedding models.
#[cfg_attr(test, mockall::automock)]
pub trait ModelCacheResolver: Send + Sync {
    /// Resolve the cache directory for a model.
    ///
    /// Returns a path where hf_hub can find the model files locally.
    fn resolve(&self, model_name: &str) -> Result<PathBuf, EmbeddingError>;
}

/// Tiered model cache with L1 (per-index) and L2 (user-level) layers.
#[derive(Debug, Clone)]
pub struct TieredModelCache {
    l1_dir: PathBuf,
    l2_dir: PathBuf,
}

impl TieredModelCache {
    /// Create a new tiered cache.
    pub fn new(l1_dir: impl Into<PathBuf>, l2_dir: impl Into<PathBuf>) -> Self {
        Self {
            l1_dir: l1_dir.into(),
            l2_dir: l2_dir.into(),
        }
    }
}

impl ModelCacheResolver for TieredModelCache {
    fn resolve(&self, model_name: &str) -> Result<PathBuf, EmbeddingError> {
        debug!(model_name, l1 = %self.l1_dir.display(), l2 = %self.l2_dir.display(), "resolving model cache");

        // Check L1
        if is_model_cached(&self.l1_dir, model_name)? {
            info!(model_name, "model found in L1 cache");
            return Ok(self.l1_dir.clone());
        }

        // Check L2
        if is_model_cached(&self.l2_dir, model_name)? {
            info!(model_name, "model found in L2 cache, copying to L1");
            copy_model_cache(&self.l2_dir, &self.l1_dir, model_name)?;
            return Ok(self.l1_dir.clone());
        }

        // Download to L2, then copy to L1
        info!(model_name, "model not cached, downloading to L2");
        download_model(&self.l2_dir, model_name)?;
        copy_model_cache(&self.l2_dir, &self.l1_dir, model_name)?;
        Ok(self.l1_dir.clone())
    }
}

/// Returns the default L2 cache directory (~/.cache/crumbly).
pub fn default_l2_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("crumbly")
}

/// Check if a model is cached in the given directory.
fn is_model_cached(cache_dir: &Path, model_name: &str) -> Result<bool, EmbeddingError> {
    let api = ApiBuilder::new()
        .with_cache_dir(cache_dir.to_path_buf())
        .build()
        .map_err(|e| {
            embedding_error::CacheAccessFailedSnafu {
                path: cache_dir.display().to_string(),
            }
            .into_error(std::io::Error::other(e.to_string()))
        })?;

    let repo = api.repo(Repo::with_revision(
        model_name.to_string(),
        RepoType::Model,
        "main".to_string(),
    ));

    // Try to get config.json - if it succeeds, model is cached
    match repo.get("config.json") {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Download a model to the cache directory.
fn download_model(cache_dir: &Path, model_name: &str) -> Result<(), EmbeddingError> {
    use embedding_error::*;

    std::fs::create_dir_all(cache_dir).context(CacheAccessFailedSnafu {
        path: cache_dir.display().to_string(),
    })?;

    let api = ApiBuilder::new()
        .with_cache_dir(cache_dir.to_path_buf())
        .build()
        .map_err(|e| {
            CacheAccessFailedSnafu {
                path: cache_dir.display().to_string(),
            }
            .into_error(std::io::Error::other(e.to_string()))
        })?;

    let repo = api.repo(Repo::with_revision(
        model_name.to_string(),
        RepoType::Model,
        "main".to_string(),
    ));

    for file in ["config.json", "tokenizer.json", "model.safetensors"] {
        repo.get(file).map_err(|e| {
            ModelDownloadFailedSnafu {
                url: format!("{}/{}", model_name, file),
            }
            .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;
    }

    Ok(())
}

/// Copy model cache from source to destination.
///
/// Copies the entire model subdirectory tree (models--org--name/) preserving
/// the hf_hub layout structure.
fn copy_model_cache(
    src_dir: &Path,
    dst_dir: &Path,
    model_name: &str,
) -> Result<(), EmbeddingError> {
    use embedding_error::*;

    // hf_hub stores models in models--<org>--<model>/ format
    let model_dir_name = format!("models--{}", model_name.replace('/', "--"));
    let src_model_dir = src_dir.join(&model_dir_name);
    let dst_model_dir = dst_dir.join(&model_dir_name);

    if !src_model_dir.exists() {
        return Err(CacheAccessFailedSnafu {
            path: src_model_dir.display().to_string(),
        }
        .into_error(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "model directory not found in source cache",
        )));
    }

    debug!(
        src = %src_model_dir.display(),
        dst = %dst_model_dir.display(),
        "copying model cache"
    );

    copy_dir_recursive(&src_model_dir, &dst_model_dir).context(CacheAccessFailedSnafu {
        path: dst_model_dir.display().to_string(),
    })?;

    Ok(())
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tempfile::TempDir;

    const TEST_MODEL: &str = "test-org/test-model";

    fn create_fake_hf_cache(base_dir: &Path, model_name: &str) {
        // hf_hub layout: models--<org>--<model>/snapshots/<hash>/<files>
        let model_dir_name = format!("models--{}", model_name.replace('/', "--"));
        let snapshot_dir = base_dir
            .join(&model_dir_name)
            .join("snapshots")
            .join("abc123");
        std::fs::create_dir_all(&snapshot_dir).unwrap();

        // Create refs/main pointing to the snapshot
        let refs_dir = base_dir.join(&model_dir_name).join("refs");
        std::fs::create_dir_all(&refs_dir).unwrap();
        std::fs::write(refs_dir.join("main"), "abc123").unwrap();

        // Create model files in snapshot
        std::fs::write(snapshot_dir.join("config.json"), r#"{"test": true}"#).unwrap();
        std::fs::write(snapshot_dir.join("tokenizer.json"), r#"{"test": true}"#).unwrap();
        std::fs::write(snapshot_dir.join("model.safetensors"), b"fake model data").unwrap();
    }

    #[test]
    fn l1_hit_returns_l1_path() {
        // Given L1 has the model cached
        let l1 = TempDir::new().unwrap();
        let l2 = TempDir::new().unwrap();
        create_fake_hf_cache(l1.path(), TEST_MODEL);

        let cache = TieredModelCache::new(l1.path(), l2.path());

        // When resolving the model
        let result = cache.resolve(TEST_MODEL).unwrap();

        // Then it returns L1 path
        assert_eq!(result, l1.path());
    }

    #[test]
    fn l2_hit_copies_to_l1() {
        // Given L2 has the model but L1 does not
        let l1 = TempDir::new().unwrap();
        let l2 = TempDir::new().unwrap();
        create_fake_hf_cache(l2.path(), TEST_MODEL);

        let cache = TieredModelCache::new(l1.path(), l2.path());

        // When resolving the model
        let result = cache.resolve(TEST_MODEL).unwrap();

        // Then it returns L1 path and copies files
        assert_eq!(result, l1.path());

        let model_dir_name = format!("models--{}", TEST_MODEL.replace('/', "--"));
        let l1_snapshot = l1
            .path()
            .join(&model_dir_name)
            .join("snapshots")
            .join("abc123");
        assert!(l1_snapshot.join("config.json").exists());
        assert!(l1_snapshot.join("tokenizer.json").exists());
        assert!(l1_snapshot.join("model.safetensors").exists());
    }

    #[test]
    fn copy_preserves_directory_structure() {
        // Given a source directory with nested structure
        let src = TempDir::new().unwrap();
        let dst = TempDir::new().unwrap();

        let nested = src.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(nested.join("file.txt"), "content").unwrap();

        // When copying recursively
        copy_dir_recursive(src.path(), dst.path()).unwrap();

        // Then structure is preserved
        assert!(
            dst.path()
                .join("a")
                .join("b")
                .join("c")
                .join("file.txt")
                .exists()
        );
    }

    #[test]
    fn default_l2_dir_returns_cache_path() {
        // When getting default L2 dir
        let result = default_l2_dir();

        // Then it ends with crumbly
        assert!(result.ends_with("crumbly"));
    }
}
