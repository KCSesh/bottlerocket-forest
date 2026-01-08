//! Test helpers for facade tests

#[cfg(test)]
pub mod test_helpers {
    use crate::knowledge::KnowledgeIndex;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    pub fn test_index_with_content(temp_dir: &TempDir) -> KnowledgeIndex {
        let repo_dir = temp_dir.path().join("test-repo");
        fs::create_dir(&repo_dir).unwrap();
        fs::write(repo_dir.join("test.md"), "# Test\n\nContent").unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();
        index
    }

    pub fn create_test_file(base: &Path, subdir: &str, filename: &str, content: &str) {
        let dir = base.join(subdir);
        if !dir.exists() {
            fs::create_dir_all(&dir).unwrap();
        }
        fs::write(dir.join(filename), content).unwrap();
    }
}
