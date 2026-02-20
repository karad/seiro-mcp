//! CLI argument definitions and `LaunchProfile` construction.
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};

use super::{build_launch_args, resolve_config_path, resolve_token, LaunchProfile, TransportMode};

/// Parsed command intent from CLI.
#[derive(Debug, Clone)]
pub enum ParsedCommand {
    RunServer(LaunchProfile),
    Cli(CliCommand),
}

/// Top-level optional CLI commands.
#[derive(Debug, Clone, Subcommand)]
pub enum CliCommand {
    /// Manage bundled Codex skills (install/remove).
    #[command(about = "Manage bundled Codex skills (install/remove)")]
    Skill(SkillArgs),
}

/// `skill` command container.
#[derive(Debug, Clone, Args)]
#[command(
    about = "Manage bundled Codex skills",
    long_about = "Manage bundled Codex skills.\n\nSubcommands:\n  install  Place bundled skills into Codex skills directory.\n  remove   Delete bundled skills from Codex skills directory.",
    after_help = "Hint: use `seiro-mcp skill install --dry-run <SKILL_NAME>` to preview planned file changes without modifying files."
)]
pub struct SkillArgs {
    #[command(subcommand)]
    pub command: SkillCommand,
}

/// Skill management subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum SkillCommand {
    /// Install a bundled skill into Codex skills directory.
    Install(SkillInstallArgs),
    /// Remove a bundled skill from Codex skills directory.
    Remove(SkillRemoveArgs),
}

/// Arguments for `skill install`.
#[derive(Debug, Clone, Args)]
pub struct SkillInstallArgs {
    /// Skill name (must start with `seiro-mcp-`).
    pub skill_name: String,
    /// Overwrite existing files.
    #[arg(long, default_value_t = false)]
    pub force: bool,
    /// Show planned changes without touching files.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

/// Arguments for `skill remove`.
#[derive(Debug, Clone, Args)]
pub struct SkillRemoveArgs {
    /// Skill name (must start with `seiro-mcp-`).
    pub skill_name: String,
}

/// Command-line arguments.
#[derive(Debug, Clone, Parser)]
#[command(
    author,
    version,
    about = "Seiro MCP (for Codex / Inspector)",
    long_about = None
)]
pub struct LaunchProfileArgs {
    /// Select stdio (default) or tcp.
    #[arg(long, value_enum, default_value_t = TransportMode::Stdio)]
    pub transport: TransportMode,
    /// Path to config.toml (overrides MCP_CONFIG_PATH).
    #[arg(long = "config")]
    pub config_override: Option<PathBuf>,
    /// Explicit token override via CLI.
    #[arg(long = "token")]
    pub token_override: Option<String>,
    /// Optional CLI command mode.
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

impl LaunchProfileArgs {
    /// Build a `LaunchProfile` from CLI args and environment variables.
    pub fn build(self) -> Result<LaunchProfile> {
        let config_path = resolve_config_path(self.config_override)?;
        let (shared_token, token_source) = resolve_token(self.token_override);

        let launch_args = build_launch_args(self.transport, &config_path);

        Ok(LaunchProfile {
            config_path,
            transport: self.transport,
            shared_token,
            token_source,
            launch_args,
        })
    }

    /// Parse CLI args into either server launch mode or utility command mode.
    pub fn into_command(self) -> Result<ParsedCommand> {
        match self.command {
            Some(command) => {
                validate_command(&command)?;
                Ok(ParsedCommand::Cli(command))
            }
            None => Ok(ParsedCommand::RunServer(self.build()?)),
        }
    }
}

fn validate_command(command: &CliCommand) -> Result<()> {
    use crate::cli::validate_skill_name_prefix;

    match command {
        CliCommand::Skill(skill) => match &skill.command {
            SkillCommand::Install(args) => {
                if !validate_skill_name_prefix(&args.skill_name) {
                    return Err(anyhow!("invalid skill name: must start with `seiro-mcp-`"));
                }
            }
            SkillCommand::Remove(args) => {
                if !validate_skill_name_prefix(&args.skill_name) {
                    return Err(anyhow!("invalid skill name: must start with `seiro-mcp-`"));
                }
            }
        },
    }

    Ok(())
}
