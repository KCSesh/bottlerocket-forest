//! File type classification for indexing
//!
//! The file scanner uses [`FileType::from_peek`] to classify files by content,
//! then filters to indexable types before dispatching to chunking strategies.

use crate::knowledge::chunking::LanguageSupport;
use crate::knowledge::domain::FilePeek;

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

    /// Classifies a file by content using registered language support.
    ///
    /// Returns `None` if no registered language supports this file type.
    pub fn from_peek(peek: &FilePeek) -> Option<Self> {
        for lang in inventory::iter::<&dyn LanguageSupport> {
            if lang.matches(peek) {
                return Some(Self(lang.context_type_name().to_string()));
            }
        }
        None
    }

    /// Classifies a file by extension only (for bare git repos where files don't exist on disk).
    ///
    /// Returns `None` if no registered language supports this extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
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
    use std::fs;
    use tempfile::TempDir;
    use test_case::test_case;

    fn create_file_and_peek(dir: &std::path::Path, name: &str, content: &str) -> FilePeek {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        FilePeek::from_path(&path).unwrap()
    }

    #[test_case("README.md", "# Test", Some("markdown") ; "markdown file")]
    #[test_case("src/main.rs", "fn main() {}", Some("rust_doc") ; "rust file")]
    #[test_case("main.go", "package main", Some("go_doc") ; "go file")]
    #[test_case("Main.java", "class Main {}", Some("java_doc") ; "java file")]
    #[test_case("Cargo.toml", "[package]", None ; "toml file")]
    fn test_file_classification(name: &str, content: &str, expected_type: Option<&str>) {
        // Given A file with content
        let temp = TempDir::new().unwrap();
        let peek = create_file_and_peek(temp.path(), name, content);

        // When Classifying the file
        let file_type = FileType::from_peek(&peek);

        // Then It should match expected classification
        assert_eq!(
            file_type.as_ref().map(|ft| ft.context_type_name()),
            expected_type
        );
    }

    #[test]
    fn test_shebang_shell_script_classification() {
        // Given A file with bash shebang but no extension
        let temp = TempDir::new().unwrap();
        let peek = create_file_and_peek(temp.path(), "script", "#!/bin/bash\necho hello");

        // When Classifying the file
        let file_type = FileType::from_peek(&peek);

        // Then It should be classified as shell
        assert_eq!(
            file_type.as_ref().map(|ft| ft.context_type_name()),
            Some("shell")
        );
    }
}
