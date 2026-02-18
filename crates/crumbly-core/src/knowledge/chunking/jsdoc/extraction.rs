//! JavaScript/TypeScript documentation extraction utilities.

use tree_sitter::Node;

use super::context::{JsItemType, JsVisibility};
use crate::knowledge::domain::Visibility;

/// Constants for tree-sitter JS/TS node kinds.
pub mod node_kinds {
    pub const COMMENT: &str = "comment";
    pub const FUNCTION_DECLARATION: &str = "function_declaration";
    pub const CLASS_DECLARATION: &str = "class_declaration";
    pub const VARIABLE_DECLARATION: &str = "variable_declaration";
    pub const LEXICAL_DECLARATION: &str = "lexical_declaration";
    pub const EXPORT_STATEMENT: &str = "export_statement";
    pub const IMPORT_STATEMENT: &str = "import_statement";
    // TS-specific
    pub const INTERFACE_DECLARATION: &str = "interface_declaration";
    pub const TYPE_ALIAS_DECLARATION: &str = "type_alias_declaration";
    pub const ENUM_DECLARATION: &str = "enum_declaration";
    // Decorators to skip
    pub const DECORATOR: &str = "decorator";
}

/// Extracts doc comment from nodes preceding a declaration.
/// Priority: JSDoc (/**) > Line runs (//) > Block (/*)
pub fn extract_doc_comment(node: Node, source: &[u8]) -> Option<String> {
    // If the node is inside an export_statement, look at the export_statement's siblings
    let search_node = if let Some(parent) = node.parent() {
        if parent.kind() == node_kinds::EXPORT_STATEMENT {
            parent
        } else {
            node
        }
    } else {
        node
    };

    extract_comment_from_siblings(search_node, source)
}

fn extract_comment_from_siblings(node: Node, source: &[u8]) -> Option<String> {
    let mut cursor = node;

    // Skip decorators
    while let Some(prev) = cursor.prev_sibling() {
        if prev.kind() == node_kinds::DECORATOR {
            cursor = prev;
        } else {
            break;
        }
    }

    // Look for comments
    let mut line_comments = Vec::new();
    let mut found_jsdoc = None;
    let mut found_block = None;
    let decl_start_line = cursor.start_position().row;

    while let Some(prev) = cursor.prev_sibling() {
        if prev.kind() == node_kinds::COMMENT {
            let text = prev.utf8_text(source).ok()?;
            let comment_end_line = prev.end_position().row;

            // Check adjacency
            let is_adjacent = comment_end_line + 1 >= decl_start_line
                || (!line_comments.is_empty()
                    && comment_end_line + 1 >= cursor.start_position().row);

            if !is_adjacent {
                break;
            }

            if text.starts_with("/**") {
                found_jsdoc = Some(parse_jsdoc(text));
                break; // JSDoc has highest priority
            } else if text.starts_with("//") {
                line_comments.push(text.trim_start_matches("//").trim().to_string());
                cursor = prev;
            } else if text.starts_with("/*") {
                found_block = Some(parse_block_comment(text));
                cursor = prev;
            } else {
                break;
            }
        } else if prev.kind() == node_kinds::DECORATOR {
            cursor = prev;
        } else {
            break;
        }
    }

    // Return by priority
    if let Some(jsdoc) = found_jsdoc {
        return filter_empty(jsdoc);
    }

    if !line_comments.is_empty() {
        line_comments.reverse();
        return filter_empty(line_comments.join("\n"));
    }

    if let Some(block) = found_block {
        return filter_empty(block);
    }

    None
}

fn filter_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

fn parse_jsdoc(text: &str) -> String {
    let content = text.trim_start_matches("/**").trim_end_matches("*/");
    content
        .lines()
        .map(|line| line.trim().trim_start_matches('*').trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_block_comment(text: &str) -> String {
    let content = text.trim_start_matches("/*").trim_end_matches("*/");
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extracts the identifier name from a declaration node.
pub fn extract_identifier(node: Node, source: &[u8]) -> Option<String> {
    // Try field name first
    if let Some(name_node) = node.child_by_field_name("name") {
        return name_node.utf8_text(source).ok().map(|s| s.to_string());
    }

    // For variable/lexical declarations, look for variable_declarator
    for child in node.children(&mut node.walk()) {
        if child.kind() == "variable_declarator"
            && let Some(name_node) = child.child_by_field_name("name")
        {
            return name_node.utf8_text(source).ok().map(|s| s.to_string());
        }
    }

    None
}

/// Determines if a node is exported.
pub fn is_exported(node: Node) -> bool {
    if let Some(parent) = node.parent() {
        return parent.kind() == node_kinds::EXPORT_STATEMENT;
    }
    false
}

/// Returns visibility based on export status.
pub fn get_visibility(node: Node) -> (JsVisibility, Visibility) {
    if is_exported(node) {
        (JsVisibility::Exported, Visibility::Public)
    } else {
        (JsVisibility::Local, Visibility::Private)
    }
}

/// Converts a JsItemType to the corresponding filter type.
pub fn to_filter_type(item_type: JsItemType) -> JsItemType {
    item_type
}

/// Gets the item type from a node kind.
pub fn get_item_type(kind: &str) -> Option<JsItemType> {
    match kind {
        node_kinds::FUNCTION_DECLARATION => Some(JsItemType::Function),
        node_kinds::CLASS_DECLARATION => Some(JsItemType::Class),
        node_kinds::VARIABLE_DECLARATION | node_kinds::LEXICAL_DECLARATION => {
            Some(JsItemType::Variable)
        }
        node_kinds::INTERFACE_DECLARATION => Some(JsItemType::Interface),
        node_kinds::TYPE_ALIAS_DECLARATION => Some(JsItemType::TypeAlias),
        node_kinds::ENUM_DECLARATION => Some(JsItemType::Enum),
        _ => None,
    }
}

/// Extracts module-level documentation (first comment before any import/declaration).
pub fn extract_module_doc(root: Node, source: &[u8]) -> Option<String> {
    for child in root.children(&mut root.walk()) {
        match child.kind() {
            node_kinds::COMMENT => {
                let text = child.utf8_text(source).ok()?;
                if text.starts_with("/**") {
                    return filter_empty(parse_jsdoc(text));
                } else if text.starts_with("//") {
                    return extract_line_comment_run(child, source);
                } else if text.starts_with("/*") {
                    return filter_empty(parse_block_comment(text));
                }
            }
            node_kinds::IMPORT_STATEMENT
            | node_kinds::EXPORT_STATEMENT
            | node_kinds::FUNCTION_DECLARATION
            | node_kinds::CLASS_DECLARATION
            | node_kinds::VARIABLE_DECLARATION
            | node_kinds::LEXICAL_DECLARATION
            | node_kinds::INTERFACE_DECLARATION
            | node_kinds::TYPE_ALIAS_DECLARATION
            | node_kinds::ENUM_DECLARATION => {
                // Hit a declaration/import before finding a comment
                return None;
            }
            _ => {}
        }
    }
    None
}

fn extract_line_comment_run(start: Node, source: &[u8]) -> Option<String> {
    let text = start.utf8_text(source).ok()?;
    let mut comments = vec![text.trim_start_matches("//").trim().to_string()];
    let mut cursor = start;

    while let Some(next) = cursor.next_sibling() {
        if next.kind() != node_kinds::COMMENT {
            break;
        }
        let next_text = match next.utf8_text(source).ok() {
            Some(t) if t.starts_with("//") => t,
            _ => break,
        };
        comments.push(next_text.trim_start_matches("//").trim().to_string());
        cursor = next;
    }

    filter_empty(comments.join("\n"))
}
