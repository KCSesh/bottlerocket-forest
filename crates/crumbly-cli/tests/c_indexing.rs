//! Integration tests for C code indexing.
//!
//! Tests C-specific features: item type filtering for functions, structs, enums.
//!
//! Run with: `cargo test --test c_indexing -- --ignored`

mod common;

use common::{crumbly_build, crumbly_search_with_chunks, setup_fixture};

// =============================================================================
// C Basic Indexing Tests
// =============================================================================

#[test]
#[ignore]
fn c_basic_indexes_and_searches() {
    // Given: A workspace with C source files
    let workspace = setup_fixture("c_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for function content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "combustion engine thermodynamic");

    // Then: Function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("combustion") || stdout.contains("engine"),
        "Should find combustion engine function: {}",
        stdout
    );
}

#[test]
#[ignore]
fn c_basic_finds_structs() {
    // Given: An indexed C workspace
    let workspace = setup_fixture("c_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for struct content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "cylinder displacement compression");

    // Then: Struct is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("EngineConfig") || stdout.contains("cylinder"),
        "Should find EngineConfig struct: {}",
        stdout
    );
}

#[test]
#[ignore]
fn c_basic_indexes_header_files() {
    // Given: An indexed C workspace with header files
    let workspace = setup_fixture("c_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for header file content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "telemetry rpm coolant oil pressure");

    // Then: Header content is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("Telemetry") || stdout.contains("telemetry") || stdout.contains("rpm"),
        "Should find EngineTelemetry from header: {}",
        stdout
    );
}

// =============================================================================
// C Item Type Filtering Tests
// =============================================================================

#[test]
#[ignore]
fn c_item_types_indexes_functions_and_structs() {
    // Given: A workspace with items = ["function", "struct"]
    let workspace = setup_fixture("c_item_types");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for function content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "syntax tokens abstract tree");

    // Then: Function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("parse") || stdout.contains("syntax"),
        "Should find parse_syntax_tokens function: {}",
        stdout
    );

    // When: Searching for struct content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "token lexeme source location");

    // Then: Struct is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("SyntaxToken") || stdout.contains("lexeme"),
        "Should find SyntaxToken struct: {}",
        stdout
    );
}

#[test]
#[ignore]
fn c_item_types_excludes_enums() {
    // Given: A workspace with items = ["function", "struct"] (no enum)
    let workspace = setup_fixture("c_item_types");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for enum content
    let (code, stdout, _) = crumbly_search_with_chunks(
        workspace.path(),
        "parser state machine transitions recursive descent",
    );

    // Then: Enum should not be in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("ParserState"),
        "Should NOT find enum ParserState: {}",
        stdout
    );
}
