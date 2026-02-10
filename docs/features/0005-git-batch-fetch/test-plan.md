# Test Plan: Git Batch Fetch

## Requirements Coverage

| Req ID | Test Type | Test Name | Description |
|--------|-----------|-----------|-------------|
| REQ-1 | unit | test_batch_fetch_default_impl | Default batch_fetch calls fetch per entry |
| REQ-2 | integration | test_batch_fetch_git_single_file | Retrieves single file via git cat-file |
| REQ-3 | integration | test_batch_fetch_git_multiple_files | Retrieves multiple files in one batch |
| REQ-4 | unit | test_batch_fetch_git_missing_file | Handles missing blob gracefully |
| REQ-5 | unit | test_batch_fetch_git_empty_file | Handles empty file content |
| REQ-6 | integration | test_indexer_with_filesystem_source | Indexer works with FilesystemSource |
| REQ-7 | integration | test_indexer_with_git_source | Indexer works with BareGitSource |
| REQ-8 | integration | test_incremental_indexing_with_content_source | Incremental updates via last_modified |

## Critical Constraints Verification

| CC ID | Verification Approach | Test Name(s) |
|-------|----------------------|---------------|
| CC-1 | FilesystemSource continues to work with indexer | test_indexer_with_filesystem_source |
| CC-2 | Incremental indexing respects last_modified | test_incremental_indexing_with_content_source |
| CC-3 | Batch returns per-file results, doesn't abort on error | test_batch_fetch_git_missing_file |
| CC-4 | Missing/empty/invalid files return FetchError, not panic | test_batch_fetch_git_missing_file, test_batch_fetch_git_empty_file |
