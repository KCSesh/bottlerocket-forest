//! File type classification for indexing
//!
//! The file scanner uses [`FileType::from_path`] to classify files by extension,
//! then filters to indexable types before dispatching to chunking strategies.

use std::path::Path;

use crate::knowledge::chunking::LanguageSupport;

/// File classification determining indexing strategy.
///
/// Built dynamically from registered language support modules.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileType(String);

impl FileType {
    /// Creates a FileType from a context type name.
    pub fn new(context_type_name: impl Into<String>) -> Self {
        Self(context_type_name.into())
    }

    /// Classifies a file by its extension using registered language support.
    ///
    /// Returns `None` if no registered language supports this file type.
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        for lang in inventory::iter::<&dyn LanguageSupport> {
            if lang.extensions().contains(&ext) {
                return Some(Self(lang.context_type_name().to_string()));
            }
        }
        None
    }

    /// Returns the context type name for this file type.
    pub fn context_type_name(&self) -> &str {
        &self.0
    }
}

impl serde::Serialize for FileType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> serde::Deserialize<'de> for FileType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use test_case::test_case;

    #[test_case("README.md", Some("markdown") ; "markdown file")]
    #[test_case("src/main.rs", Some("rust_doc") ; "rust file")]
    #[test_case("main.go", Some("go_doc") ; "go file")]
    #[test_case("Main.java", Some("java_doc") ; "java file")]
    #[test_case("Cargo.toml", None ; "toml file")]
    #[test_case("LICENSE", None ; "no extension")]
    fn test_file_classification(path: &str, expected_type: Option<&str>) {
        // Given A file path
        let path = Path::new(path);

        // When Classifying the file
        let file_type = FileType::from_path(path);

        // Then It should match expected classification
        assert_eq!(
            file_type.as_ref().map(|ft| ft.context_type_name()),
            expected_type
        );
    }
}
