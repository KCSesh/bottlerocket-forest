#!/usr/bin/env python3
"""Lint Rust source files for line count limits."""
import sys
from pathlib import Path

MAX_LOC = 550
OVERRIDES = {
    "crates/crumbly-core/src/knowledge/storage/sqlite/mod.rs": 630,
    "crates/crumbly-core/src/knowledge/indexing/scanner/internals.rs": 565,
    "crates/crumbly-core/src/knowledge/chunking/rustdoc/extraction.rs": 565,
    "crates/crumbly-core/src/knowledge/indexing/config.rs": 570,
    "crates/crumbly-core/src/knowledge/search/semantic.rs": 575,
}
CRATES = ["crates/brdev", "crates/crumbly-cli", "crates/crumbly-core", "crates/forester"]

def is_test_file(path):
    s = str(path)
    return "/tests/" in s or s.endswith("tests.rs") or s.endswith("_test.rs")

def main():
    violations = []
    for crate in CRATES:
        for rs in Path(crate).rglob("*.rs"):
            if is_test_file(rs):
                continue
            limit = OVERRIDES.get(str(rs), MAX_LOC)
            lines = len(rs.read_text().splitlines())
            if lines > limit:
                violations.append((str(rs), lines, limit))
    if violations:
        print("LOC limit exceeded:\n")
        for path, lines, limit in violations:
            print(f"  {path}: {lines} lines (limit: {limit})")
        print("""
=== Refactoring Guidance ===

1. SPLIT IMPLEMENTATION (preferred for large files)
   If a module has multiple logical operations, split them:
   
   BEFORE: facade/build.rs (877 lines)
     - build(), rebuild(), update(), clear() + all tests
   
   AFTER:
     - facade/build.rs (550 lines): build(), rebuild() + their tests
     - facade/update.rs (189 lines): update(), clear() + their tests
   
   Tests stay co-located with their implementation.

2. DRY TEST HELPERS (module-local)
   Add helpers to #[cfg(test)] module to reduce setup boilerplate:
   
   fn setup_test_repo() -> (TempDir, PathBuf) { ... }
   fn create_test_file(dir: &Path, name: &str, content: &str) { ... }

3. USE test_case FOR PARAMETERIZED TESTS
   Consolidate similar tests that differ only in inputs:
   
   #[test_case("" ; "empty string")]
   #[test_case("   " ; "whitespace only")]
   fn rejects_blank_input(input: &str) { ... }

* Keep tests co-located with implementation
* Preserve Given/When/Then comments (required by style guide)
* DO NOT delete docstrings or comments to reduce line count
""")
        sys.exit(1)
    print("All modules are within LOC limits")

if __name__ == "__main__":
    main()
