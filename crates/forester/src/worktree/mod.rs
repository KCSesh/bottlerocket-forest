//! Forest worktree management.

use crate::error::Error;
use crate::forest::{ForestConfig, Member};
use std::path::PathBuf;
use std::process::Command;

/// Manages forest operations including seeding and worktrees.
pub struct ForestManager {
    root: PathBuf,
    config: ForestConfig,
}

impl ForestManager {
    pub fn new(root: PathBuf, config: ForestConfig) -> Self {
        Self { root, config }
    }

    /// Path to bare repos storage.
    fn bare_dir(&self) -> PathBuf {
        self.root.join(".forest").join("bare")
    }

    /// Path to worktrees directory.
    fn worktrees_dir(&self) -> PathBuf {
        self.root.join("worktrees")
    }

    /// Seed the forest - clone bare repos and create develop worktree.
    pub fn seed(&self, verbose: bool) -> Result<(), Error> {
        let bare_dir = self.bare_dir();
        std::fs::create_dir_all(&bare_dir).map_err(|e| Error::CreateDir {
            path: bare_dir.clone(),
            source: e,
        })?;

        // Clone all bare repos first
        for member in &self.config.member {
            self.clone_bare(member, verbose)?;
        }

        // Create the default "develop" worktree
        self.create_worktree("develop", None, verbose)?;

        Ok(())
    }

    /// Clone a member repo as bare.
    fn clone_bare(&self, member: &Member, verbose: bool) -> Result<(), Error> {
        let bare_path = self.bare_dir().join(format!("{}.git", member.name));

        if !bare_path.exists() {
            if verbose {
                println!("Cloning {} (bare)...", member.name);
            }
            let status = Command::new("git")
                .args(["clone", "--bare", &member.remote])
                .arg(&bare_path)
                .status()
                .map_err(|_| Error::Git {
                    message: format!("Failed to clone {}", member.remote),
                })?;
            if !status.success() {
                return Err(Error::Git {
                    message: format!("git clone failed for {}", member.name),
                });
            }
        } else if verbose {
            println!("✓ {} bare repo exists", member.name);
        }

        Ok(())
    }

    /// Create a new forest worktree.
    pub fn create_worktree(&self, name: &str, branch: Option<&str>, verbose: bool) -> Result<(), Error> {
        let wt_dir = self.worktrees_dir().join(name);
        if wt_dir.exists() {
            if verbose {
                println!("✓ worktree '{}' already exists", name);
            }
            return Ok(());
        }

        std::fs::create_dir_all(&wt_dir).map_err(|e| Error::CreateDir {
            path: wt_dir.clone(),
            source: e,
        })?;

        for member in &self.config.member {
            let bare_path = self.bare_dir().join(format!("{}.git", member.name));
            let member_wt_path = wt_dir.join(&member.path);

            // Ensure parent exists
            if let Some(parent) = member_wt_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::CreateDir {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }

            if verbose {
                println!("Creating worktree for {} in {}...", member.name, name);
            }

            // Determine branch: explicit > member default
            let target_branch = branch.unwrap_or_else(|| member.branch());

            // For non-develop worktrees, create a unique branch to avoid conflicts
            let use_new_branch = name != "develop" && branch.is_none();
            let new_branch_name = if use_new_branch {
                format!("{}/{}", name, member.name)
            } else {
                target_branch.to_string()
            };

            // Create worktree with appropriate branch strategy
            let status = if use_new_branch {
                // Create new branch from default branch
                Command::new("git")
                    .args(["worktree", "add", "-b", &new_branch_name])
                    .arg(&member_wt_path)
                    .arg(member.branch())
                    .current_dir(&bare_path)
                    .status()
                    .map_err(|_| Error::Git {
                        message: format!("Failed to create worktree for {}", member.name),
                    })?
            } else {
                // Try to checkout existing branch
                let status = Command::new("git")
                    .args(["worktree", "add"])
                    .arg(&member_wt_path)
                    .arg(target_branch)
                    .current_dir(&bare_path)
                    .status()
                    .map_err(|_| Error::Git {
                        message: format!("Failed to create worktree for {}", member.name),
                    })?;

                if !status.success() {
                    // If branch doesn't exist, create it from default
                    Command::new("git")
                        .args(["worktree", "add", "-b", target_branch])
                        .arg(&member_wt_path)
                        .arg(member.branch())
                        .current_dir(&bare_path)
                        .status()
                        .map_err(|_| Error::Git {
                            message: format!("Failed to create worktree for {}", member.name),
                        })?
                } else {
                    status
                }
            };

            if !status.success() {
                return Err(Error::Git {
                    message: format!("git worktree add failed for {}", member.name),
                });
            }
        }

        // Copy sembly.toml to worktree if it exists
        let sembly_src = self.root.join("sembly.toml");
        if sembly_src.exists() {
            let _ = std::fs::copy(&sembly_src, wt_dir.join("sembly.toml"));
        }

        // Build sembly index for this worktree
        if verbose {
            println!("Building sembly index for {}...", name);
        }
        let _ = Command::new("sembly")
            .arg("build")
            .current_dir(&wt_dir)
            .status();

        if verbose {
            println!("✓ Created worktree '{}'", name);
        }

        Ok(())
    }

    /// List existing forest worktrees.
    pub fn list_worktrees(&self) -> Result<Vec<String>, Error> {
        let wt_dir = self.worktrees_dir();
        if !wt_dir.exists() {
            return Ok(vec![]);
        }

        let mut worktrees = Vec::new();
        let entries = std::fs::read_dir(&wt_dir).map_err(|e| Error::CreateDir {
            path: wt_dir.clone(),
            source: e,
        })?;

        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    worktrees.push(name.to_string());
                }
            }
        }

        Ok(worktrees)
    }

    /// Remove a forest worktree.
    pub fn remove_worktree(&self, name: &str, force: bool) -> Result<(), Error> {
        let wt_dir = self.worktrees_dir().join(name);
        if !wt_dir.exists() {
            return Err(Error::WorktreeNotFound {
                name: name.to_string(),
            });
        }

        // Remove git worktrees for each member
        for member in &self.config.member {
            let bare_path = self.bare_dir().join(format!("{}.git", member.name));
            let member_wt_path = wt_dir.join(&member.path);

            let mut cmd = Command::new("git");
            cmd.args(["worktree", "remove"]);
            if force {
                cmd.arg("--force");
            }
            cmd.arg(&member_wt_path);
            cmd.current_dir(&bare_path);

            let _ = cmd.status(); // Ignore errors, directory might already be gone
        }

        // Remove the worktree directory
        let _ = std::fs::remove_dir_all(&wt_dir);

        Ok(())
    }
}
