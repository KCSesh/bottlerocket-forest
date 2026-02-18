//! Integration tests for JavaScript code indexing.
//!
//! Tests JavaScript-specific features: visibility filtering, item type filtering.
//!
//! Run with: `cargo test --test js_indexing -- --ignored`

mod common;

use common::{crumbly_build, crumbly_search_with_chunks, setup_fixture};

// =============================================================================
// JavaScript Basic Indexing Tests
// =============================================================================

#[test]
#[ignore]
fn js_basic_indexes_and_searches() {
    // Given: A workspace with JavaScript files
    let workspace = setup_fixture("js_basic");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for function content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "quantum entanglement protocols");

    // Then: Function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("initializeQuantumApp") || stdout.contains("quantum"),
        "Should find quantum app function: {}",
        stdout
    );
}

// =============================================================================
// JavaScript Visibility Filtering Tests
// =============================================================================

#[test]
#[ignore]
fn js_visibility_indexes_exported_items() {
    // Given: A workspace with visibility = ["public"]
    let workspace = setup_fixture("js_visibility");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for exported function
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "biometric verification retinal");

    // Then: Exported function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("authenticateBiometric") || stdout.contains("biometric"),
        "Should find exported authenticateBiometric: {}",
        stdout
    );
}

#[test]
#[ignore]
fn js_visibility_excludes_local_items() {
    // Given: A workspace with visibility = ["public"]
    let workspace = setup_fixture("js_visibility");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for local function content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "synaptic resonance detectors calibrates");

    // Then: Local function should not appear in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("setupNeuralMatcher"),
        "Should NOT find local setupNeuralMatcher: {}",
        stdout
    );
}

// =============================================================================
// JavaScript Item Type Filtering Tests
// =============================================================================

#[test]
#[ignore]
fn js_item_types_indexes_functions_and_classes() {
    // Given: A workspace with items = ["functions", "classes"]
    let workspace = setup_fixture("js_item_types");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for function content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "antimatter containment field");

    // Then: Function is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("processAntimatterRequest") || stdout.contains("antimatter"),
        "Should find function: {}",
        stdout
    );

    // When: Searching for class content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "plasma conduit routing");

    // Then: Class is found
    assert_eq!(code, 0, "Search failed");
    assert!(
        stdout.contains("PlasmaConduitManager") || stdout.contains("plasma"),
        "Should find class: {}",
        stdout
    );
}

#[test]
#[ignore]
fn js_item_types_excludes_variables() {
    // Given: A workspace with items = ["functions", "classes"]
    let workspace = setup_fixture("js_item_types");
    let (code, _, stderr) = crumbly_build(workspace.path());
    assert_eq!(code, 0, "Build failed: {}", stderr);

    // When: Searching for variable content
    let (code, stdout, _) =
        crumbly_search_with_chunks(workspace.path(), "warp field subspace distortion coefficient");

    // Then: Variable should not be in results
    assert_eq!(code, 0, "Search failed");
    assert!(
        !stdout.contains("WARP_FIELD_COEFFICIENT"),
        "Should NOT find variable WARP_FIELD_COEFFICIENT: {}",
        stdout
    );
}
