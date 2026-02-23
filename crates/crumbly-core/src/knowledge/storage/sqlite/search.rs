//! Semantic search implementation using vector embeddings
//!
//! Provides k-nearest-neighbor search using sqlite-vec with cosine distance.
//!
//! # Implementation Notes
//!
//! Similarity scores are computed as `1.0 - distance` where distance is cosine distance.
//! This assumes normalized embeddings (as produced by candle). Non-normalized embeddings
//! may produce scores outside [0, 1].
//!
//! # Context Filtering
//!
//! sqlite-vec's `k` parameter limits results BEFORE any JOIN/WHERE filters are applied.
//! However, sqlite-vec supports `chunk_hash IN (subquery)` to pre-filter at the vector
//! search level. We use this to constrain KNN search to only chunks in the target context.
//!
//! This ensures context isolation is maintained (MCI-15).

use rusqlite::Connection;
use snafu::ResultExt;

use super::serialization::{indexed_chunk_from_row, serialize_embedding};
use crate::knowledge::domain::{
    ContextId, FileSearchResult, IndexRelativePath, IndexedChunk, RelevanceScore, RepoName,
    ResultLimit, SearchResult,
};
use crate::knowledge::storage::repository::StorageError;

/// Performs k-nearest-neighbor search using cosine distance
///
/// Returns chunks ranked by similarity score in descending order. Similarity scores
/// are computed as `1.0 - distance` where distance is the cosine distance between
/// normalized embeddings.
///
/// When context_id is Some, results are filtered to files in that context.
/// When context_id is None, all chunks are searched.
pub fn search_semantic(
    conn: &Connection,
    query_embedding: &[f32],
    limit: ResultLimit,
    context_id: ContextId,
) -> Result<Vec<(IndexedChunk, RelevanceScore)>, StorageError> {
    use crate::knowledge::storage::repository::storage_error::*;

    let embedding_bytes = serialize_embedding(query_embedding);

    // Two-phase approach
    // Phase 1: Get valid file_hashes and file_paths for this context
    let file_paths = get_context_file_paths(conn, &context_id)?;

    if file_paths.is_empty() {
        return Ok(Vec::new());
    }

    // Use IN subquery to pre-filter at sqlite-vec level
    // Both chunks.chunk_hash and vec_chunks.chunk_hash are TEXT now - simple equality JOIN
    let query = r#"
                SELECT c.chunk_hash, c.chunk_hash, c.file_hash, '', c.repo_name,
                    c.context_type, c.context_data, c.content, c.token_count, c.last_modified,
                    v.distance
                FROM vec_chunks v
                JOIN chunks c ON c.chunk_hash = v.chunk_hash
                WHERE v.embedding MATCH ?1 AND k = ?2
                  AND v.chunk_hash IN (
                    SELECT c2.chunk_hash
                    FROM chunks c2
                    JOIN indexed_files f ON c2.file_hash = f.file_hash
                    WHERE f.context_id = ?3
                  )
                ORDER BY v.distance
            "#;

    let mut stmt = conn.prepare(query).context(DatabaseSnafu)?;
    let rows = stmt
        .query_map(
            [
                &embedding_bytes as &dyn rusqlite::ToSql,
                &(limit.into_inner() as i64),
                &context_id.as_str(),
            ],
            |row| {
                let distance: f32 = row.get(10)?;
                let chunk =
                    indexed_chunk_from_row(row).map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok((chunk, distance))
            },
        )
        .context(DatabaseSnafu)?;

    // Filter results to only include chunks from valid files
    // and update file_path from indexed_files
    let mut results = Vec::new();
    for row_result in rows {
        let (mut chunk, distance) = row_result.context(DatabaseSnafu)?;

        // Check if this chunk's file_hash is in the context and get file_path
        if let Some(file_path) = file_paths.get(&chunk.chunk.file_hash) {
            // Update the chunk's file_path from indexed_files
            chunk.chunk.source.file_path = file_path.clone();
            let similarity = (1.0 - distance).clamp(0.0, 1.0);
            let score =
                RelevanceScore::try_new(similarity).unwrap_or_else(|_| RelevanceScore::zero());
            results.push((chunk, score));

            if results.len() >= limit.into_inner() {
                break;
            }
        }
    }

    Ok(results)
}

/// Gets the file_hash to file_path mapping for a specific context
fn get_context_file_paths(
    conn: &Connection,
    context_id: &ContextId,
) -> Result<
    std::collections::HashMap<
        crate::knowledge::domain::FileHash,
        crate::knowledge::domain::IndexRelativePath,
    >,
    StorageError,
> {
    use crate::knowledge::storage::repository::storage_error::*;

    let query = "SELECT file_hash, file_path FROM indexed_files WHERE context_id = ?";
    let mut stmt = conn.prepare(query).context(DatabaseSnafu)?;

    let rows = stmt
        .query_map([context_id.to_string()], |row| {
            let hash_bytes: Vec<u8> = row.get(0)?;
            let file_path: String = row.get(1)?;
            Ok((hash_bytes, file_path))
        })
        .context(DatabaseSnafu)?;

    let mut mapping = std::collections::HashMap::new();
    for row_result in rows {
        let (hash_bytes, file_path) = row_result.context(DatabaseSnafu)?;
        if hash_bytes.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&hash_bytes);
            let file_hash = crate::knowledge::domain::FileHash::new(arr);
            if let Ok(path) = crate::knowledge::domain::IndexRelativePath::try_new(&file_path) {
                mapping.insert(file_hash, path);
            }
        }
    }

    Ok(mapping)
}

/// Searches for files containing semantically similar chunks.
///
/// Over-fetches chunks using `k = file_limit * chunk_multiplier`, then groups
/// results by file path in Rust. Returns files ranked by their best chunk match.
pub fn search_files(
    conn: &Connection,
    query_embedding: &[f32],
    file_limit: ResultLimit,
    chunk_multiplier: usize,
    context_id: ContextId,
) -> Result<Vec<FileSearchResult>, StorageError> {
    use crate::knowledge::storage::repository::storage_error::*;

    let k = file_limit
        .into_inner()
        .saturating_mul(chunk_multiplier)
        .min(100 * chunk_multiplier);
    let embedding_bytes = serialize_embedding(query_embedding);

    let file_paths = get_context_file_paths(conn, &context_id)?;
    if file_paths.is_empty() {
        return Ok(Vec::new());
    }

    // Both chunks.chunk_hash and vec_chunks.chunk_hash are TEXT now - simple equality JOIN
    let query = r#"
        SELECT c.chunk_hash, c.chunk_hash, c.file_hash, '', c.repo_name,
            c.context_type, c.context_data, c.content, c.token_count, c.last_modified,
            v.distance
        FROM vec_chunks v
        JOIN chunks c ON c.chunk_hash = v.chunk_hash
        WHERE v.embedding MATCH ?1 AND k = ?2
          AND v.chunk_hash IN (
            SELECT c2.chunk_hash
            FROM chunks c2
            JOIN indexed_files f ON c2.file_hash = f.file_hash
            WHERE f.context_id = ?3
          )
        ORDER BY v.distance
    "#;

    let mut stmt = conn.prepare(query).context(DatabaseSnafu)?;
    let rows = stmt
        .query_map(
            [
                &embedding_bytes as &dyn rusqlite::ToSql,
                &(k as i64),
                &context_id.as_str(),
            ],
            |row| {
                let distance: f32 = row.get(10)?;
                let chunk =
                    indexed_chunk_from_row(row).map_err(|_| rusqlite::Error::InvalidQuery)?;
                Ok((chunk, distance))
            },
        )
        .context(DatabaseSnafu)?;

    // Collect chunks and group by file
    #[allow(clippy::type_complexity)]
    let mut file_chunks: std::collections::HashMap<
        crate::knowledge::domain::FileHash,
        (
            IndexRelativePath,
            RepoName,
            Vec<(IndexedChunk, RelevanceScore)>,
        ),
    > = std::collections::HashMap::new();

    for row_result in rows {
        let (mut chunk, distance) = row_result.context(DatabaseSnafu)?;
        if let Some(file_path) = file_paths.get(&chunk.chunk.file_hash) {
            chunk.chunk.source.file_path = file_path.clone();
            let similarity = (1.0 - distance).clamp(0.0, 1.0);
            let score =
                RelevanceScore::try_new(similarity).unwrap_or_else(|_| RelevanceScore::zero());

            let entry = file_chunks.entry(chunk.chunk.file_hash).or_insert_with(|| {
                (
                    file_path.clone(),
                    chunk.chunk.source.repo_name.clone(),
                    Vec::new(),
                )
            });
            entry.2.push((chunk, score));
        }
    }

    // Build FileSearchResults sorted by best score
    let mut results: Vec<FileSearchResult> = file_chunks
        .into_iter()
        .map(|(_, (file_path, repo_name, chunks))| {
            let best_score = chunks
                .iter()
                .map(|(_, s)| *s)
                .max()
                .unwrap_or_else(RelevanceScore::zero);
            let match_count = chunks.len();
            let search_results: Vec<SearchResult> = chunks
                .into_iter()
                .map(|(c, s)| SearchResult::builder().chunk(c.chunk).score(s).build())
                .collect();
            FileSearchResult::builder()
                .file_path(file_path)
                .repo_name(repo_name)
                .match_count(match_count)
                .best_score(best_score)
                .chunks(search_results)
                .build()
        })
        .collect();

    results.sort_by(|a, b| {
        b.best_score
            .partial_cmp(&a.best_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(file_limit.into_inner());

    Ok(results)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::chunking::markdown::MarkdownContext;
    use crate::knowledge::constants::EMBEDDING_DIM;
    use crate::knowledge::domain::{
        Chunk, ChunkContent, ChunkContext, ChunkHash, ChunkId, ChunkSource, Embedding,
        EmbeddingModelConfig, FileHash, IndexRelativePath, IndexedFile, RepoName, ResultLimit,
        Timestamp, TokenCount,
    };
    use crate::knowledge::storage::schema;
    use crate::knowledge::storage::sqlite::files::insert_indexed_file;
    use rusqlite::Connection;
    use test_case::test_case;

    fn setup_connection() -> Connection {
        #[allow(clippy::missing_transmute_annotations)]
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn, &EmbeddingModelConfig::default()).unwrap();
        conn
    }

    fn create_chunk(content: &str, file_hash: FileHash, embedding: Vec<f32>) -> IndexedChunk {
        let chunk = Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(ChunkHash::from_text(content))
            .file_hash(file_hash)
            .source(
                ChunkSource::builder()
                    .file_path(IndexRelativePath::try_new("test.md").unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(content)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(
                ChunkContext::new(
                    "markdown",
                    &MarkdownContext::builder().heading_hierarchy(vec![]).build(),
                )
                .unwrap(),
            )
            .build();
        IndexedChunk::builder()
            .chunk(chunk)
            .embedding(Embedding::try_new(embedding).unwrap())
            .indexed_at(Timestamp::now())
            .build()
    }

    fn insert_chunk(conn: &Connection, chunk: &IndexedChunk) {
        use crate::knowledge::storage::sqlite::chunks::save_chunk;
        save_chunk(conn, chunk).unwrap();
        let bytes: Vec<u8> = chunk
            .embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect();
        conn.execute(
            "INSERT INTO vec_chunks (chunk_hash, embedding) VALUES (?, ?)",
            rusqlite::params![chunk.chunk.chunk_hash.to_string(), bytes],
        )
        .unwrap();
    }

    fn index_file(conn: &Connection, ctx: &ContextId, path: &str, hash: FileHash) {
        insert_indexed_file(
            conn,
            &IndexedFile::builder()
                .context_id(ctx.clone())
                .file_path(IndexRelativePath::try_new(path).unwrap())
                .file_hash(hash)
                .mtime_ns(1000)
                .build(),
        )
        .unwrap();
    }

    #[test_case("empty" ; "returns empty when context has no files")]
    #[test_case("other" ; "does not leak results across contexts")]
    fn search_semantic_context_isolation(scenario: &str) {
        // Given a chunk indexed only in context-a
        let conn = setup_connection();
        let hash = FileHash::new([1u8; 32]);
        insert_chunk(
            &conn,
            &create_chunk("content", hash, vec![0.8; EMBEDDING_DIM]),
        );
        let ctx_a = ContextId::from_path("context-a").unwrap();
        index_file(&conn, &ctx_a, "file.md", hash);

        // When searching a different context
        let target = ContextId::from_path(if scenario == "empty" {
            "empty"
        } else {
            "context-b"
        })
        .unwrap();
        let results = search_semantic(
            &conn,
            &vec![0.8; EMBEDDING_DIM],
            ResultLimit::try_new(10).unwrap(),
            target,
        )
        .unwrap();

        // Then no results should leak
        assert!(results.is_empty());
    }

    #[test]
    fn search_semantic_filters_by_context() {
        // Given chunks in two different contexts
        let conn = setup_connection();
        let hash_a = FileHash::new([1u8; 32]);
        let hash_b = FileHash::new([2u8; 32]);
        insert_chunk(
            &conn,
            &create_chunk("content A", hash_a, vec![0.8; EMBEDDING_DIM]),
        );
        insert_chunk(
            &conn,
            &create_chunk("content B", hash_b, vec![0.7; EMBEDDING_DIM]),
        );

        let ctx_a = ContextId::from_path("context-a").unwrap();
        let ctx_b = ContextId::from_path("context-b").unwrap();
        index_file(&conn, &ctx_a, "file-a.md", hash_a);
        index_file(&conn, &ctx_b, "file-b.md", hash_b);

        // When searching context_a
        let results = search_semantic(
            &conn,
            &vec![0.75; EMBEDDING_DIM],
            ResultLimit::try_new(10).unwrap(),
            ctx_a,
        )
        .unwrap();

        // Then only context_a chunks returned
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.chunk.file_hash, hash_a);
    }

    /// Verifies context filtering works when other contexts have closer matches.
    /// sqlite-vec's k parameter limits results BEFORE WHERE filtering, so naive
    /// queries could return 0 results. Our IN subquery approach avoids this.
    #[test_case(5, 1, 2, 1 ; "finds target when other context has closer matches")]
    #[test_case(100, 5, 5, 5 ; "works without multiplier hack")]
    fn search_semantic_context_filtering_with_many_chunks(
        other_count: usize,
        target_count: usize,
        limit: usize,
        expected: usize,
    ) {
        let conn = setup_connection();
        let hash_other = FileHash::new([2u8; 32]);
        for i in 0..other_count {
            let mut emb = vec![0.99; EMBEDDING_DIM];
            emb[0] = 0.99 - (i as f32 * 0.0001);
            insert_chunk(&conn, &create_chunk(&format!("O{}", i), hash_other, emb));
        }
        let hash_target = FileHash::new([1u8; 32]);
        for i in 0..target_count {
            let mut emb = vec![0.5; EMBEDDING_DIM];
            emb[0] = 0.5 + (i as f32 * 0.01);
            insert_chunk(&conn, &create_chunk(&format!("T{}", i), hash_target, emb));
        }

        let ctx_other = ContextId::from_path("ctx-other").unwrap();
        let ctx_target = ContextId::from_path("ctx-target").unwrap();
        index_file(&conn, &ctx_other, "other.md", hash_other);
        index_file(&conn, &ctx_target, "target.md", hash_target);

        let results = search_semantic(
            &conn,
            &vec![0.99; EMBEDDING_DIM],
            ResultLimit::try_new(limit).unwrap(),
            ctx_target,
        )
        .unwrap();

        assert_eq!(results.len(), expected);
        assert!(
            results
                .iter()
                .all(|(c, _)| c.chunk.file_hash == hash_target)
        );
    }

    #[test]
    fn search_files_groups_chunks_by_file() {
        // Given chunks from two files in the same context
        let conn = setup_connection();
        let hash_a = FileHash::new([1u8; 32]);
        let hash_b = FileHash::new([2u8; 32]);

        for i in 0..3 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[0] = 1.0;
            emb[1] = 0.1 * (i as f32);
            insert_chunk(&conn, &create_chunk(&format!("A{}", i), hash_a, emb));
        }
        for i in 0..2 {
            let mut emb = vec![0.0; EMBEDDING_DIM];
            emb[1] = 1.0;
            emb[2] = 0.1 * (i as f32);
            insert_chunk(&conn, &create_chunk(&format!("B{}", i), hash_b, emb));
        }

        let ctx = ContextId::from_path("test-context").unwrap();
        index_file(&conn, &ctx, "file-a.md", hash_a);
        index_file(&conn, &ctx, "file-b.md", hash_b);

        // When searching with query pointing toward file A
        let mut query = vec![0.0; EMBEDDING_DIM];
        query[0] = 1.0;
        let results =
            search_files(&conn, &query, ResultLimit::try_new(10).unwrap(), 3, ctx).unwrap();

        // Then results grouped by file, file A first
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].file_path.to_string(), "file-a.md");
        assert_eq!(results[0].match_count, 3);
        assert_eq!(results[1].file_path.to_string(), "file-b.md");
        assert_eq!(results[1].match_count, 2);
    }

    #[test]
    fn search_files_respects_file_limit() {
        // Given 5 files with one chunk each
        let conn = setup_connection();
        let ctx = ContextId::from_path("test-context").unwrap();
        for i in 0..5u8 {
            let hash = FileHash::new([i + 1; 32]);
            let emb = vec![0.9 - (i as f32 * 0.05); EMBEDDING_DIM];
            insert_chunk(&conn, &create_chunk(&format!("file{}", i), hash, emb));
            index_file(&conn, &ctx, &format!("file-{}.md", i), hash);
        }

        // When searching with file_limit=2
        let results = search_files(
            &conn,
            &vec![0.9; EMBEDDING_DIM],
            ResultLimit::try_new(2).unwrap(),
            3,
            ctx,
        )
        .unwrap();

        // Then only 2 files returned
        assert_eq!(results.len(), 2);
    }
}
