//! Content-based file identification for indexing.
//!
//! [`FilePeek`] reads the first bytes of a file to detect content type
//! via shebang lines and magic bytes, enabling language plugins to identify
//! files by content rather than just extension.

use std::io::{self, Read};
use std::path::{Path, PathBuf};

/// Content-based file identification.
///
/// Constructed once per file during scanning, reads only the first ~256 bytes
/// to detect shebang lines and file format via magic bytes.
#[derive(Debug, Clone)]
pub struct FilePeek {
    path: PathBuf,
    extension: Option<String>,
    shebang: Option<String>,
    format: file_format::FileFormat,
}

impl FilePeek {
    /// Reads the first ~256 bytes of a file to detect content type.
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string());

        let mut file = std::fs::File::open(path)?;
        let mut buffer = [0u8; 256];
        let bytes_read = file.read(&mut buffer)?;
        let buffer = &buffer[..bytes_read];

        let shebang = Self::parse_shebang(buffer);
        let format = file_format::FileFormat::from_bytes(buffer);

        Ok(Self {
            path: path.to_path_buf(),
            extension,
            shebang,
            format,
        })
    }

    /// Returns the file path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the file extension if present.
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    /// Returns the shebang line if present.
    pub fn shebang(&self) -> Option<&str> {
        self.shebang.as_deref()
    }

    /// Returns the detected file format.
    pub fn format(&self) -> file_format::FileFormat {
        self.format
    }

    /// Creates a synthetic FilePeek from just a path string.
    ///
    /// Used for cases where the file doesn't exist on disk (bare git repos, cacher).
    /// Only extension-based detection will work; shebang detection requires actual file content.
    pub fn from_path_string(path: &str) -> Self {
        let path = std::path::Path::new(path);
        Self {
            path: path.to_path_buf(),
            extension: path
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_string()),
            shebang: None,
            format: file_format::FileFormat::default(),
        }
    }

    fn parse_shebang(buffer: &[u8]) -> Option<String> {
        if buffer.len() < 2 || buffer[0] != b'#' || buffer[1] != b'!' {
            return None;
        }
        // Find end of first line
        let end = buffer
            .iter()
            .position(|&b| b == b'\n')
            .unwrap_or(buffer.len());
        std::str::from_utf8(&buffer[..end])
            .ok()
            .map(|s| s.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_from_path_with_shebang() {
        // Given A file with a bash shebang
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("script");
        fs::write(&path, "#!/bin/bash\necho hello").unwrap();

        // When Creating a FilePeek
        let peek = FilePeek::from_path(&path).unwrap();

        // Then The shebang is detected
        assert_eq!(peek.shebang(), Some("#!/bin/bash"));
        assert!(peek.extension().is_none());
    }

    #[test]
    fn test_from_path_with_extension() {
        // Given A .rs file without shebang
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("main.rs");
        fs::write(&path, "fn main() {}").unwrap();

        // When Creating a FilePeek
        let peek = FilePeek::from_path(&path).unwrap();

        // Then The extension is detected and no shebang
        assert_eq!(peek.extension(), Some("rs"));
        assert!(peek.shebang().is_none());
    }

    #[test]
    fn test_from_path_with_env_shebang() {
        // Given A file with env bash shebang
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("script");
        fs::write(&path, "#!/usr/bin/env bash\necho hello").unwrap();

        // When Creating a FilePeek
        let peek = FilePeek::from_path(&path).unwrap();

        // Then The shebang is detected
        assert_eq!(peek.shebang(), Some("#!/usr/bin/env bash"));
    }
}
