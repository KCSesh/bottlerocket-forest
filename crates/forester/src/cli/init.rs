//! Init command - initialize a new forest.

use clap::Args;
use owo_colors::OwoColorize;
use std::fs;
use std::path::Path;
use tracing::instrument;

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Forest name
    #[arg(short, long)]
    name: Option<String>,
}

const GITIGNORE: &str = r#"# Forester
.forest/
worktrees/
"#;

const FORESTER_TOML_TEMPLATE: &str = r#"[forest]
name = "{name}"

# Add members like this:
# [[member]]
# name = "my-repo"
# remote = "git@github.com:org/my-repo.git"
# path = "my-repo"
# default_branch = "main"
"#;

#[instrument(err)]
pub fn run(args: InitArgs) -> miette::Result<()> {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let name = args.name.unwrap_or_else(|| {
        cwd.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("my-forest")
            .to_string()
    });

    // Check if already initialized
    if Path::new("forester.toml").exists() {
        println!("{} Forest already initialized", "!".yellow());
        return Ok(());
    }

    // Create forester.toml
    let forester_toml = FORESTER_TOML_TEMPLATE.replace("{name}", &name);
    fs::write("forester.toml", forester_toml).expect("Failed to write forester.toml");
    println!("{} Created forester.toml", "✓".green());

    // Create/update .gitignore
    let gitignore_path = Path::new(".gitignore");
    if gitignore_path.exists() {
        let existing = fs::read_to_string(gitignore_path).unwrap_or_default();
        if !existing.contains(".forest/") {
            let updated = format!("{}\n{}", existing.trim_end(), GITIGNORE);
            fs::write(gitignore_path, updated).expect("Failed to update .gitignore");
            println!("{} Updated .gitignore", "✓".green());
        }
    } else {
        fs::write(gitignore_path, GITIGNORE).expect("Failed to write .gitignore");
        println!("{} Created .gitignore", "✓".green());
    }

    println!("\nForest '{}' initialized. Next steps:", name.cyan());
    println!(
        "  1. Edit {} to add member repositories",
        "forester.toml".cyan()
    );
    println!("  2. Run {} to clone and set up", "forester seed".cyan());

    Ok(())
}
