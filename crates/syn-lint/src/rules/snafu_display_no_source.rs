//! Snafu display messages should not contain {source}.

use crate::rule::{Rule, Violation};
use std::path::Path;
use syn::File;

struct SnafuDisplayNoSource;

inventory::submit!(&SnafuDisplayNoSource as &dyn Rule);

impl Rule for SnafuDisplayNoSource {
    fn name(&self) -> &'static str {
        "snafu-display-no-source"
    }

    fn check(&self, path: &Path, file: &File) -> Vec<Violation> {
        let mut violations = Vec::new();
        for item in &file.items {
            if let syn::Item::Enum(e) = item {
                for variant in &e.variants {
                    if let Some(line) = has_source_in_display(&variant.attrs) {
                        violations.push(Violation {
                            file: path.display().to_string(),
                            line,
                            message: format!(
                                "variant `{}::{}` has {{source}} in snafu display message",
                                e.ident, variant.ident
                            ),
                            doc_url: Some("docs/style/rust-design.md#key-snafu-rules"),
                        });
                    }
                }
            }
        }
        violations
    }
}

fn has_source_in_display(attrs: &[syn::Attribute]) -> Option<usize> {
    for attr in attrs {
        if !attr.path().is_ident("snafu") {
            continue;
        }
        let mut found_line = None;
        let _ = attr.parse_nested_meta(|m| {
            if m.path.is_ident("display") {
                let content;
                syn::parenthesized!(content in m.input);
                if let Ok(lit) = content.parse::<syn::LitStr>() {
                    if lit.value().contains("{source}") {
                        found_line = Some(lit.span().start().line);
                    }
                }
            }
            Ok(())
        });
        if found_line.is_some() {
            return found_line;
        }
    }
    None
}
