//! Script execution plugin.

use crate::domain::HookConfig;
use crate::events::ForesterEvent;
use crate::hooks::{Hook, HookContext, HookError, Plugin, PluginError, Trigger};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn parse_line(line: &str, default_stderr: bool) -> ForesterEvent {
    if let Some(msg) = line.strip_prefix("::info::") {
        ForesterEvent::Info(msg.to_string())
    } else if let Some(msg) = line.strip_prefix("::warning::") {
        ForesterEvent::Warning(msg.to_string())
    } else if let Some(msg) = line.strip_prefix("::error::") {
        ForesterEvent::Error(msg.to_string())
    } else if default_stderr {
        ForesterEvent::Warning(line.to_string())
    } else {
        ForesterEvent::Info(line.to_string())
    }
}

/// Plugin for executing arbitrary scripts.
pub struct ExecPlugin;

impl Plugin for ExecPlugin {
    fn name(&self) -> &str {
        "exec"
    }

    fn create_hook(&self, config: &HookConfig) -> Result<Box<dyn Hook>, PluginError> {
        let path = config
            .path
            .clone()
            .ok_or_else(|| PluginError::InvalidConfig {
                message: "exec hook requires 'path' field".to_string(),
            })?;
        let args = config.args.clone().unwrap_or_default();
        let triggers: Vec<Trigger> = config
            .triggers
            .iter()
            .filter_map(|s| Trigger::parse(s))
            .collect();
        Ok(Box::new(ExecHook {
            path,
            args,
            triggers,
        }))
    }
}

struct ExecHook {
    path: PathBuf,
    args: Vec<String>,
    triggers: Vec<Trigger>,
}

impl Hook for ExecHook {
    fn triggers(&self) -> &[Trigger] {
        &self.triggers
    }

    fn execute(&self, ctx: &HookContext) -> Result<(), HookError> {
        let script_path = if self.path.is_relative() {
            ctx.forest_root.path().join(&self.path)
        } else {
            self.path.clone()
        };

        ctx.emit(&ForesterEvent::Info(format!(
            "Running script: {}",
            script_path.display()
        )));

        let mut cmd = Command::new(&script_path);
        cmd.args(&self.args)
            .current_dir(ctx.forest_root.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (k, v) in ctx.env_vars() {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn().map_err(|e| HookError::Execution {
            message: format!("Failed to run {}: {}", script_path.display(), e),
        })?;

        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                ctx.emit(&parse_line(&line, false));
            }
        }

        if let Some(stderr) = child.stderr.take() {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                ctx.emit(&parse_line(&line, true));
            }
        }

        let status = child.wait().map_err(|e| HookError::Execution {
            message: format!("Failed to wait for {}: {}", script_path.display(), e),
        })?;

        if !status.success() {
            return Err(HookError::Execution {
                message: format!("Script {} failed", script_path.display()),
            });
        }

        Ok(())
    }
}
