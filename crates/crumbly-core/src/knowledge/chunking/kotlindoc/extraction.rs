//! Kotlin documentation extraction utilities.

use tree_sitter::Node;

use super::context::KotlinVisibility;
use crate::knowledge::domain::Visibility;

/// Constants for tree-sitter Kotlin node kinds.
pub mod node_kinds {
    pub const BLOCK_COMMENT: &str = "block_comment";
    pub const MODIFIERS: &str = "modifiers";
    pub const VISIBILITY_MODIFIER: &str = "visibility_modifier";
    pub const CLASS_DECLARATION: &str = "class_declaration";
    pub const OBJECT_DECLARATION: &str = "object_declaration";
    pub const FUNCTION_DECLARATION: &str = "function_declaration";
    pub const PROPERTY_DECLARATION: &str = "property_declaration";
}

/// Extracts KDoc comment (/** ... */) from nodes preceding a declaration.
pub fn extract_doc_comment(node: Node, source: &[u8]) -> Option<String> {
    let mut cursor = node;
    while let Some(prev) = cursor.prev_sibling() {
        if prev.kind() == node_kinds::BLOCK_COMMENT {
            let text = prev.utf8_text(source).ok()?;
            if text.starts_with("/**") {
                return Some(parse_kdoc(text));
            }
        } else if prev.kind() == node_kinds::MODIFIERS {
            cursor = prev;
            continue;
        }
        break;
    }
    None
}

fn parse_kdoc(text: &str) -> String {
    let content = text.trim_start_matches("/**").trim_end_matches("*/");
    content
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extracts the identifier name from a declaration node.
pub fn extract_identifier(node: Node, source: &[u8]) -> Option<String> {
    // Try "name" field first (used by most declarations)
    if let Some(name_node) = node.child_by_field_name("name") {
        return name_node.utf8_text(source).ok().map(|s| s.to_string());
    }
    // For some nodes, look for simple_identifier child
    for child in node.children(&mut node.walk()) {
        if child.kind() == "simple_identifier" {
            return child.utf8_text(source).ok().map(|s| s.to_string());
        }
    }
    None
}

/// Determines visibility from modifiers node.
pub fn get_visibility(node: Node, source: &[u8]) -> (KotlinVisibility, Visibility) {
    let visibility_text = find_visibility_modifier(node, source);
    match visibility_text {
        Some("public") => (KotlinVisibility::Public, Visibility::Public),
        Some("internal") => (KotlinVisibility::Internal, Visibility::Public),
        Some("protected") => (KotlinVisibility::Protected, Visibility::Public),
        Some("private") => (KotlinVisibility::Private, Visibility::Private),
        _ => (KotlinVisibility::Public, Visibility::Public),
    }
}

fn find_visibility_modifier<'a>(node: Node<'a>, source: &'a [u8]) -> Option<&'a str> {
    for child in node.children(&mut node.walk()) {
        if child.kind() != node_kinds::MODIFIERS {
            continue;
        }
        for modifier in child.children(&mut child.walk()) {
            if modifier.kind() == node_kinds::VISIBILITY_MODIFIER {
                return modifier.utf8_text(source).ok();
            }
        }
    }
    None
}
