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

    /// Seed the forest - clone bare repos and create main worktree.
    pub fn seed(&self, verbose: bool) -> Result<(), Error> {
        let bare_dir = self.bare_dir();
        std::fs::create_dir_all(&bare_dir).map_err(|e| Error::CreateDir {
            path: bare_dir.clone(),
            source: e,
        })?;

        for member in &self.config.member {
            self.seed_member(member, verbose)?;
        }

        // Build sembly index if sembly.toml exists
        if self.root.join("sembly.toml").exists() {
            if verbose {
                println!("Building sembly index...");
            }
            let _ = Command::new("sembly")
                .arg("build")
                .current_dir(&self.root)
                .status();
        }

        Ok(())
    }

    fn seed_member(&self, member: &Member, verbose: bool) -> Result<(), Error> {
        let bare_path = self.bare_dir().join(format!("{}.git", member.name));
        let worktree_path = self.root.join(&member.path);

        // Clone bare if not exists
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

        // Create main worktree if not exists
        if !worktree_path.exists() {
            if verbose {
                println!("Creating worktree for {}...", member.name);
            }
            // Ensure parent directory exists
            if let Some(parent) = worktree_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::CreateDir {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }
            let status = Command::new("git")
                .args(["worktree", "add"])
                .arg(&worktree_path)
                .arg(member.branch())
                .current_dir(&bare_path)
                .status()
                .map_err(|_| Error::Git {
                    message: format!("Failed to create worktree for {}", member.name),
                })?;
            if !status.success() {
                return Err(Error::Git {
                    message: format!("git worktree add failed for {}", member.name),
                });
            }
        } else if verbose {
            println!("✓ {} worktree exists", member.name);
        }

        Ok(())
    }

    /// Create a new forest worktree.
    pub fn create_worktree(&self, name: &str, branch: &str) -> Result<(), Error> {
        let wt_dir = self.worktrees_dir().join(name);
        if wt_dir.exists() {
            return Err(Error::WorktreeExists {
                name: name.to_string(),
            });
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

            let status = Command::new("git")
                .args(["worktree", "add", "-b", branch])
                .arg(&member_wt_path)
                .arg(member.branch())
                .current_dir(&bare_path)
                .status()
                .map_err(|_| Error::Git {
                    message: format!("Failed to create worktree for {}", member.name),
                })?;

            // If branch already exists, try without -b
            if status.success() {
                continue;
            }
            
            let status = Command::new("git")
                .args(["worktree", "add"])
                .arg(&member_wt_path)
                .arg(branch)
                .current_dir(&bare_path)
                .status()
                .map_err(|_| Error::Git {
                    message: format!("Failed to create worktree for {}", member.name),
                })?;
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

        Ok(())
    }

    /// List existing forest worktrees.
    pub fn list_worktrees(&self) -> Result<Vec<String>, Error> {
        let wt_dir = self.worktrees_dir();
        if !wt_dir.exists() {
            return Ok(vec![]);
        }

        let mut worktrees = Vec::new();
        for entry in std::fs::read_dir(&wt_dir).map_err(|e| Error::CreateDir {
            path: wt_dir.clone(),
            source: e,
        })?.flatten() {
            if entry.path().is_dir() && let Some(name) = entry.file_name().to_str() {
                worktrees.push(name.to_string());
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
