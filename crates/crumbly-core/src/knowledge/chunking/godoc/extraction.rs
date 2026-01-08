//! Go documentation extraction utilities.

use tree_sitter::Node;

use crate::knowledge::domain::{GoItemType, GoVisibility, Visibility};
use crate::knowledge::indexing::GoItemType as FilterGoItemType;

/// Constants for tree-sitter Go node kinds.
pub mod node_kinds {
    pub const COMMENT: &str = "comment";
    pub const IDENTIFIER: &str = "identifier";
    pub const TYPE_IDENTIFIER: &str = "type_identifier";
    pub const PACKAGE_CLAUSE: &str = "package_clause";
    pub const FUNCTION_DECLARATION: &str = "function_declaration";
    pub const METHOD_DECLARATION: &str = "method_declaration";
    pub const TYPE_DECLARATION: &str = "type_declaration";
    pub const CONST_DECLARATION: &str = "const_declaration";
    pub const VAR_DECLARATION: &str = "var_declaration";
    pub const STRUCT_TYPE: &str = "struct_type";
    pub const INTERFACE_TYPE: &str = "interface_type";
}

/// Extracts doc comments from nodes preceding a declaration.
pub fn extract_doc_comment(node: Node, source: &[u8]) -> Option<String> {
    let decl_start_line = node.start_position().row;
    let mut comments = Vec::new();
    let mut cursor = node;
    let mut total_len = 0;

    while let Some(prev) = cursor.prev_sibling() {
        if prev.kind() == node_kinds::COMMENT {
            let comment_end_line = prev.end_position().row;
            if comment_end_line + 1 >= decl_start_line
                || (!comments.is_empty() && comment_end_line + 1 >= cursor.start_position().row)
            {
                let text = prev.utf8_text(source).ok()?;
                let trimmed = text.trim_start_matches("//").trim();
                total_len += trimmed.len() + 1;
                comments.push(trimmed);
                cursor = prev;
            } else {
                break;
            }
        } else if prev.kind().contains(node_kinds::COMMENT) {
            cursor = prev;
        } else {
            break;
        }
    }

    if comments.is_empty() {
        return None;
    }

    comments.reverse();
    let mut result = String::with_capacity(total_len);
    for (i, comment) in comments.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }
        result.push_str(comment);
    }

    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Extracts the identifier name from a declaration node.
pub fn extract_identifier(node: Node, source: &[u8]) -> Option<String> {
    for child in node.children(&mut node.walk()) {
        if child.kind() == node_kinds::IDENTIFIER || child.kind() == node_kinds::TYPE_IDENTIFIER {
            return child.utf8_text(source).ok().map(|s| s.to_string());
        }
        if child.kind() == "type_spec"
            && let Some(name) = extract_identifier(child, source)
        {
            return Some(name);
        }
    }
    None
}

/// Returns true if the name is exported (starts with uppercase).
pub fn is_exported(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Determines the specific type kind (struct, interface, or generic type).
pub fn determine_type_kind(node: Node) -> GoItemType {
    for child in node.children(&mut node.walk()) {
        match child.kind() {
            node_kinds::STRUCT_TYPE => return GoItemType::Struct,
            node_kinds::INTERFACE_TYPE => return GoItemType::Interface,
            "type_spec" => {
                let result = determine_type_kind(child);
                if result != GoItemType::Type {
                    return result;
                }
            }
            _ => {}
        }
    }
    GoItemType::Type
}

/// Returns visibility based on whether the name is exported.
pub fn get_visibility(name: &str) -> (GoVisibility, Visibility) {
    if is_exported(name) {
        (GoVisibility::Exported, Visibility::Public)
    } else {
        (GoVisibility::Unexported, Visibility::Private)
    }
}

/// Converts a GoItemType to the corresponding filter type.
pub fn to_filter_type(item_type: GoItemType) -> FilterGoItemType {
    match item_type {
        GoItemType::Function => FilterGoItemType::Function,
        GoItemType::Method => FilterGoItemType::Method,
        GoItemType::Struct => FilterGoItemType::Struct,
        GoItemType::Interface => FilterGoItemType::Interface,
        GoItemType::Type => FilterGoItemType::Type,
        GoItemType::Const => FilterGoItemType::Const,
        GoItemType::Var => FilterGoItemType::Var,
        GoItemType::Package => FilterGoItemType::Function, // Package docs always included
    }
}
