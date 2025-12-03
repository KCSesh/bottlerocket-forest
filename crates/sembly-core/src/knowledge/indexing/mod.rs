//! Indexing module
pub mod filter;

pub use filter::{IndexingFilter, RustFilter, RustItemType};

// Stub types for modules not yet added
pub struct FileScanner;
pub struct IndexResult;
pub struct IndexStrategy;
pub struct IndexableFile;
pub struct Indexer;
pub struct IndexingError;
pub struct ScanError;
pub struct SemblyConfig;
pub struct SemblyConfigError;
pub fn load_sembly_config() {}
