//! Style linter for Rust code using syn.

mod rule;
mod rules;

use clap::{Parser, ValueEnum};
use owo_colors::OwoColorize;
use rule::{Rule, Violation};
use std::{fs, io::IsTerminal, path::Path, process::ExitCode};
use walkdir::WalkDir;

/// Style linter for Rust code.
#[derive(Parser)]
#[command(version, about, styles = clap_cargo::style::CLAP_STYLING)]
struct Args {
    /// When to use colors (auto, always, never)
    #[arg(long, default_value = "auto")]
    color: ColorChoice,
}

/// When to use colored output.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum ColorChoice {
    /// Use colors only when output is a terminal
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

fn main() -> ExitCode {
    let args = Args::parse();

    match args.color {
        ColorChoice::Always => owo_colors::set_override(true),
        ColorChoice::Never => owo_colors::set_override(false),
        ColorChoice::Auto => {
            if !std::io::stderr().is_terminal() {
                owo_colors::set_override(false);
            }
        }
    }

    let violations = lint_crates(Path::new("crates"));
    for v in &violations {
        eprint!(
            "{}:{}: {}",
            v.file.cyan(),
            v.line.bold(),
            v.message.yellow()
        );
        if let Some(url) = v.doc_url {
            eprint!(" {}", format!("See: {url}").dimmed());
        }
        eprintln!();
    }
    if violations.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn lint_crates(root: &Path) -> Vec<Violation> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            let p = e.path();
            p.extension().is_some_and(|e| e == "rs")
                && p.components().any(|c| c.as_os_str() == "src")
        })
        .filter_map(|e| lint_file(e.path()))
        .flatten()
        .collect()
}

fn lint_file(path: &Path) -> Option<Vec<Violation>> {
    let content = fs::read_to_string(path).ok()?;
    let file = syn::parse_file(&content).ok()?;
    let v: Vec<_> = inventory::iter::<&dyn Rule>()
        .flat_map(|rule| rule.check(path, &file))
        .collect();
    Some(v)
}
