//! Embedding model and provider trait
//!
//! Defines the interface for generating embeddings from text and provides
//! a builder for configuring and loading embedding models.

use std::path::PathBuf;

use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, Config, DTYPE};
use hf_hub::{Repo, RepoType, api::sync::ApiBuilder};
use snafu::IntoError;
use tokenizers::Tokenizer;

use crate::knowledge::domain::Embedding;

use super::error::{EmbeddingError, embedding_error};

/// Generates embeddings from text content
#[cfg_attr(test, mockall::automock)]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate an embedding vector for text
    fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError>;

    /// Generate embeddings for multiple texts
    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Embedding>, EmbeddingError>;

    /// Get the dimensionality of embeddings produced by this provider
    fn dimension(&self) -> usize;

    /// Get the model name
    fn model_name(&self) -> &str;
}

/// Embedding model configuration
///
/// Configure model parameters using the builder, then call `load()` to initialize
/// the model for generating embeddings. Models are downloaded and cached in the
/// configured cache directory (defaults to `$XDG_CACHE_HOME/crumbly/model` or
/// `~/.cache/crumbly/model` on Linux).
#[derive(bon::Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct EmbeddingModel {
    #[builder(default = crate::knowledge::constants::DEFAULT_TOKENIZER_MODEL.to_string())]
    model_name: String,
    #[builder(default = crate::knowledge::constants::EMBEDDING_DIM)]
    dimension: usize,
    #[builder(default = default_cache_dir())]
    cache_dir: PathBuf,
}

impl EmbeddingModel {
    /// Load the embedding model
    ///
    /// Downloads and caches the model if not already present in the cache directory.
    pub fn load(self) -> Result<LoadedEmbeddingModel, EmbeddingError> {
        let api = ApiBuilder::new()
            .with_cache_dir(self.cache_dir)
            .build()
            .map_err(|e| {
                embedding_error::ModelLoadFailedSnafu {
                    model_name: self.model_name.clone(),
                }
                .into_error(Box::new(std::io::Error::other(e.to_string())))
            })?;

        let repo = api.repo(Repo::with_revision(
            self.model_name.clone(),
            RepoType::Model,
            "main".to_string(),
        ));

        let config_path = repo.get("config.json").map_err(|e| {
            embedding_error::ModelLoadFailedSnafu {
                model_name: self.model_name.clone(),
            }
            .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let tokenizer_path = repo.get("tokenizer.json").map_err(|e| {
            embedding_error::ModelLoadFailedSnafu {
                model_name: self.model_name.clone(),
            }
            .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let weights_path = repo.get("model.safetensors").map_err(|e| {
            embedding_error::ModelLoadFailedSnafu {
                model_name: self.model_name.clone(),
            }
            .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let config: Config =
            serde_json::from_str(&std::fs::read_to_string(&config_path).map_err(|e| {
                embedding_error::ModelLoadFailedSnafu {
                    model_name: self.model_name.clone(),
                }
                .into_error(Box::new(e))
            })?)
            .map_err(|e| {
                embedding_error::ModelLoadFailedSnafu {
                    model_name: self.model_name.clone(),
                }
                .into_error(Box::new(std::io::Error::other(e.to_string())))
            })?;

        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| {
            embedding_error::ModelLoadFailedSnafu {
                model_name: self.model_name.clone(),
            }
            .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let device = Device::Cpu;
        // SAFETY: The safetensors model files are read-only and not modified during execution.
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], DTYPE, &device).map_err(|e| {
                embedding_error::ModelLoadFailedSnafu {
                    model_name: self.model_name.clone(),
                }
                .into_error(Box::new(std::io::Error::other(e.to_string())))
            })?
        };

        let model = BertModel::load(vb, &config).map_err(|e| {
            embedding_error::ModelLoadFailedSnafu {
                model_name: self.model_name.clone(),
            }
            .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        Ok(LoadedEmbeddingModel {
            model_name: self.model_name,
            dimension: self.dimension,
            model,
            tokenizer,
            device,
        })
    }
}

/// Loaded embedding model ready for generating embeddings
pub struct LoadedEmbeddingModel {
    model_name: String,
    dimension: usize,
    model: BertModel,
    tokenizer: Tokenizer,
    device: Device,
}

impl std::fmt::Debug for LoadedEmbeddingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedEmbeddingModel")
            .field("model_name", &self.model_name)
            .field("dimension", &self.dimension)
            .field("model", &"<BertModel>")
            .field("tokenizer", &"<Tokenizer>")
            .field("device", &self.device)
            .finish()
    }
}

impl EmbeddingProvider for LoadedEmbeddingModel {
    fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let encoding = self.tokenizer.encode(text, true).map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let input_ids = Tensor::new(encoding.get_ids(), &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(|e| {
                embedding_error::EmbeddingGenerationFailedSnafu
                    .into_error(Box::new(std::io::Error::other(e.to_string())))
            })?;

        let token_type_ids = input_ids.zeros_like().map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let attention_mask = Tensor::new(encoding.get_attention_mask(), &self.device)
            .and_then(|t| t.unsqueeze(0))
            .map_err(|e| {
                embedding_error::EmbeddingGenerationFailedSnafu
                    .into_error(Box::new(std::io::Error::other(e.to_string())))
            })?;

        let embeddings = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| {
                embedding_error::EmbeddingGenerationFailedSnafu
                    .into_error(Box::new(std::io::Error::other(e.to_string())))
            })?;

        let embedding_vec = mean_pool_and_normalize(&embeddings, &attention_mask)?;
        validate_dimension(&embedding_vec, self.dimension)?;

        Embedding::try_new(embedding_vec).map_err(|_| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other("invalid embedding vector")))
        })
    }

    fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Embedding>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let encodings: Vec<_> = texts
            .iter()
            .map(|t| {
                self.tokenizer.encode(t.as_str(), true).map_err(|e| {
                    embedding_error::EmbeddingGenerationFailedSnafu
                        .into_error(Box::new(std::io::Error::other(e.to_string())))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let max_len = encodings
            .iter()
            .map(|e| e.get_ids().len())
            .max()
            .unwrap_or(0);

        let mut all_input_ids = Vec::new();
        let mut all_attention_masks = Vec::new();

        for enc in &encodings {
            let ids = enc.get_ids();
            let mask = enc.get_attention_mask();
            let mut padded_ids = ids.to_vec();
            let mut padded_mask = mask.to_vec();
            padded_ids.resize(max_len, 0);
            padded_mask.resize(max_len, 0);
            all_input_ids.push(padded_ids);
            all_attention_masks.push(padded_mask);
        }

        let batch_size = texts.len();
        let input_ids = Tensor::new(
            all_input_ids
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .as_slice(),
            &self.device,
        )
        .and_then(|t| t.reshape((batch_size, max_len)))
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let token_type_ids = input_ids.zeros_like().map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let attention_mask = Tensor::new(
            all_attention_masks
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .as_slice(),
            &self.device,
        )
        .and_then(|t| t.reshape((batch_size, max_len)))
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

        let embeddings = self
            .model
            .forward(&input_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| {
                embedding_error::EmbeddingGenerationFailedSnafu
                    .into_error(Box::new(std::io::Error::other(e.to_string())))
            })?;

        let pooled = mean_pool_and_normalize_batch(&embeddings, &attention_mask)?;

        pooled
            .into_iter()
            .map(|vec| {
                validate_dimension(&vec, self.dimension)?;
                Embedding::try_new(vec).map_err(|_| {
                    embedding_error::EmbeddingGenerationFailedSnafu
                        .into_error(Box::new(std::io::Error::other("invalid embedding vector")))
                })
            })
            .collect()
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Returns the default cache directory for embedding models.
pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("crumbly")
        .join("model")
}

fn mean_pool_and_normalize(
    embeddings: &Tensor,
    attention_mask: &Tensor,
) -> Result<Vec<f32>, EmbeddingError> {
    let mask = attention_mask
        .unsqueeze(2)
        .and_then(|m| m.broadcast_as(embeddings.shape()))
        .and_then(|m| m.to_dtype(embeddings.dtype()))
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

    let masked = embeddings.mul(&mask).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    let sum = masked.sum(1).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    let count = mask
        .sum(1)
        .and_then(|c| c.to_dtype(embeddings.dtype()))
        .and_then(|c| c.clamp(1.0, f64::INFINITY))
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

    let pooled = sum.broadcast_div(&count).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    let norm = pooled
        .sqr()
        .and_then(|s| s.sum_keepdim(1))
        .and_then(|s| s.sqrt())
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

    let normalized = pooled.broadcast_div(&norm).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    normalized
        .squeeze(0)
        .and_then(|t| t.to_vec1())
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })
}

fn mean_pool_and_normalize_batch(
    embeddings: &Tensor,
    attention_mask: &Tensor,
) -> Result<Vec<Vec<f32>>, EmbeddingError> {
    let mask = attention_mask
        .unsqueeze(2)
        .and_then(|m| m.broadcast_as(embeddings.shape()))
        .and_then(|m| m.to_dtype(embeddings.dtype()))
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

    let masked = embeddings.mul(&mask).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    let sum = masked.sum(1).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    let count = mask
        .sum(1)
        .and_then(|c| c.to_dtype(embeddings.dtype()))
        .and_then(|c| c.clamp(1.0, f64::INFINITY))
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

    let pooled = sum.broadcast_div(&count).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    let norm = pooled
        .sqr()
        .and_then(|s| s.sum_keepdim(1))
        .and_then(|s| s.sqrt())
        .map_err(|e| {
            embedding_error::EmbeddingGenerationFailedSnafu
                .into_error(Box::new(std::io::Error::other(e.to_string())))
        })?;

    let normalized = pooled.broadcast_div(&norm).map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })?;

    normalized.to_vec2().map_err(|e| {
        embedding_error::EmbeddingGenerationFailedSnafu
            .into_error(Box::new(std::io::Error::other(e.to_string())))
    })
}

fn validate_dimension(embedding: &[f32], expected: usize) -> Result<(), EmbeddingError> {
    snafu::ensure!(
        embedding.len() == expected,
        embedding_error::DimensionMismatchSnafu {
            expected,
            actual: embedding.len()
        }
    );
    Ok(())
}
