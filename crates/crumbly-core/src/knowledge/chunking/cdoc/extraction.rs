//! C documentation extraction utilities.

use tree_sitter::Node;

use super::context::CItemType;

/// Constants for tree-sitter C node kinds.
pub mod node_kinds {
    pub const COMMENT: &str = "comment";
    pub const FUNCTION_DEFINITION: &str = "function_definition";
    pub const STRUCT_SPECIFIER: &str = "struct_specifier";
    pub const ENUM_SPECIFIER: &str = "enum_specifier";
    pub const TYPE_DEFINITION: &str = "type_definition";
    pub const PREPROC_DEF: &str = "preproc_def";
    pub const PREPROC_FUNCTION_DEF: &str = "preproc_function_def";
    pub const DECLARATION: &str = "declaration";
}

/// Represents an extracted comment with its position.
#[derive(Debug)]
pub struct ExtractedComment {
    pub text: String,
}

/// Extracts doc comment from nodes preceding a declaration.
pub fn extract_doc_comment(node: Node, source: &[u8]) -> Option<String> {
    let decl_start_line = node.start_position().row;
    let mut comments = Vec::new();
    let mut cursor = node;

    while let Some(prev) = cursor.prev_sibling() {
        if prev.kind() != node_kinds::COMMENT {
            break;
        }
        let comment_end_line = prev.end_position().row;
        let is_adjacent = comment_end_line + 1 >= decl_start_line
            || (!comments.is_empty() && comment_end_line + 1 >= cursor.start_position().row);
        if !is_adjacent {
            break;
        }
        let text = prev.utf8_text(source).ok()?;
        let parsed = parse_comment(text);
        if !parsed.is_empty() {
            comments.push(parsed);
        }
        cursor = prev;
    }

    if comments.is_empty() {
        return None;
    }

    comments.reverse();
    Some(comments.join("\n"))
}

/// Parses a single comment node, handling //, /* */, and /** */ styles.
fn parse_comment(text: &str) -> String {
    let text = text.trim();
    if text.starts_with("//") {
        text.trim_start_matches("//").trim().to_string()
    } else if text.starts_with("/**") {
        parse_block_comment(text, true)
    } else if text.starts_with("/*") {
        parse_block_comment(text, false)
    } else {
        text.to_string()
    }
}

/// Parses block comments (/* */ or /** */).
fn parse_block_comment(text: &str, is_kernel_style: bool) -> String {
    let content = if is_kernel_style {
        text.trim_start_matches("/**").trim_end_matches("*/")
    } else {
        text.trim_start_matches("/*").trim_end_matches("*/")
    };
    content
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('*') {
                trimmed.trim_start_matches('*').trim()
            } else {
                trimmed
            }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extracts the identifier name from a declaration node.
pub fn extract_identifier(node: Node, source: &[u8]) -> Option<String> {
    // Try declarator field first (for function definitions)
    if let Some(declarator) = node.child_by_field_name("declarator") {
        return extract_name_from_declarator(declarator, source);
    }
    // Try name field (for macros)
    if let Some(name) = node.child_by_field_name("name") {
        return name.utf8_text(source).ok().map(|s| s.to_string());
    }
    // For struct/enum specifiers, look for name child
    for child in node.children(&mut node.walk()) {
        if child.kind() == "type_identifier" {
            return child.utf8_text(source).ok().map(|s| s.to_string());
        }
    }
    None
}

fn extract_name_from_declarator(node: Node, source: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => node.utf8_text(source).ok().map(|s| s.to_string()),
        "function_declarator" | "pointer_declarator" | "array_declarator" => {
            if let Some(declarator) = node.child_by_field_name("declarator") {
                return extract_name_from_declarator(declarator, source);
            }
            node.children(&mut node.walk())
                .find_map(|child| extract_name_from_declarator(child, source))
        }
        _ => None,
    }
}

/// Determines the item type from a node kind.
pub fn get_item_type(kind: &str) -> Option<CItemType> {
    match kind {
        node_kinds::FUNCTION_DEFINITION => Some(CItemType::Function),
        node_kinds::STRUCT_SPECIFIER => Some(CItemType::Struct),
        node_kinds::ENUM_SPECIFIER => Some(CItemType::Enum),
        node_kinds::TYPE_DEFINITION => Some(CItemType::Typedef),
        node_kinds::PREPROC_DEF | node_kinds::PREPROC_FUNCTION_DEF => Some(CItemType::Macro),
        node_kinds::DECLARATION => Some(CItemType::Declaration),
        _ => None,
    }
}

/// Finds standalone comment blocks not attached to declarations.
pub fn find_standalone_comments(root: Node, source: &[u8]) -> Vec<ExtractedComment> {
    let mut comments = Vec::new();
    let mut cursor = root.walk();

    for node in root.children(&mut cursor) {
        if node.kind() != node_kinds::COMMENT {
            continue;
        }
        // Check if next sibling is a declaration
        let has_following_decl = node.next_sibling().is_some_and(|next| {
            let next_start = next.start_position().row;
            let comment_end = node.end_position().row;
            comment_end + 1 >= next_start && get_item_type(next.kind()).is_some()
        });
        if has_following_decl {
            continue;
        }
        let Ok(text) = node.utf8_text(source) else {
            continue;
        };
        let parsed = parse_comment(text);
        if parsed.is_empty() {
            continue;
        }
        comments.push(ExtractedComment { text: parsed });
    }

    comments
}
