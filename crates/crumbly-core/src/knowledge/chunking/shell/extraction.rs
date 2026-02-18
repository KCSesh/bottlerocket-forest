//! Shell comment extraction utilities.

use tree_sitter::Node;

use super::context::ShellItemType;

/// Constants for tree-sitter bash node kinds.
pub mod node_kinds {
    pub const COMMENT: &str = "comment";
    pub const FUNCTION_DEFINITION: &str = "function_definition";
}

/// Extracted comment with its relationship to code.
pub struct ExtractedComment {
    /// The comment text with # prefix stripped.
    pub text: String,
    /// Whether this comment is attached to a function.
    pub item_type: ShellItemType,
    /// Function name if attached to a function.
    pub function_name: Option<String>,
    /// Function signature if attached to a function.
    pub signature: Option<String>,
}

/// Extracts comment blocks from shell source.
pub fn extract_comments(root: Node, source: &[u8]) -> Vec<ExtractedComment> {
    let mut results = Vec::new();
    let mut cursor = root.walk();
    let children: Vec<_> = root.children(&mut cursor).collect();
    let mut i = 0;

    while i < children.len() {
        let node = children[i];
        if node.kind() == node_kinds::COMMENT {
            // Skip shebang lines
            if let Ok(text) = node.utf8_text(source)
                && text.starts_with("#!")
            {
                i += 1;
                continue;
            }

            // Collect consecutive comments
            let (comment_text, end_idx) = collect_comment_block(&children, i, source);

            // Check if next non-whitespace sibling is a function
            let next_idx = end_idx + 1;
            if next_idx < children.len()
                && children[next_idx].kind() == node_kinds::FUNCTION_DEFINITION
            {
                let func_node = children[next_idx];
                let (name, sig) = extract_function_info(func_node, source);
                results.push(ExtractedComment {
                    text: comment_text,
                    item_type: ShellItemType::Function,
                    function_name: name,
                    signature: sig,
                });
                i = next_idx + 1;
            } else {
                results.push(ExtractedComment {
                    text: comment_text,
                    item_type: ShellItemType::StandaloneComment,
                    function_name: None,
                    signature: None,
                });
                i = end_idx + 1;
            }
        } else {
            i += 1;
        }
    }

    results
}

fn collect_comment_block(children: &[Node], start: usize, source: &[u8]) -> (String, usize) {
    let mut lines = Vec::new();
    let mut end = start;
    let mut prev_end_row = children[start].end_position().row;

    for (idx, node) in children.iter().enumerate().skip(start) {
        if node.kind() != node_kinds::COMMENT {
            break;
        }
        // Skip shebang in block collection too
        if let Ok(text) = node.utf8_text(source)
            && text.starts_with("#!")
        {
            break;
        }
        let start_row = node.start_position().row;
        // Check adjacency (allow for the first comment or consecutive lines)
        if idx > start && start_row > prev_end_row + 1 {
            break;
        }
        if let Ok(text) = node.utf8_text(source) {
            let stripped = text.trim_start_matches('#').trim_start_matches(' ');
            lines.push(stripped.to_string());
        }
        prev_end_row = node.end_position().row;
        end = idx;
    }

    (lines.join("\n"), end)
}

fn extract_function_info(node: Node, source: &[u8]) -> (Option<String>, Option<String>) {
    // tree-sitter-bash function_definition structure:
    // - May have "function" keyword as first child
    // - Has function name as "word" node
    // - Has body as compound_statement
    let mut name = None;
    for child in node.children(&mut node.walk()) {
        // The function name is typically a "word" node
        if child.kind() == "word" {
            name = child.utf8_text(source).ok().map(|s| s.to_string());
            break;
        }
    }

    let signature = node
        .utf8_text(source)
        .ok()
        .and_then(|s| s.lines().next())
        .map(|s| s.trim().to_string());

    (name, signature)
}
