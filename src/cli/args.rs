//! CLI argument definitions and `LaunchProfile` construction.

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};

use super::{build_launch_args, resolve_config_path, LaunchProfile};

/// Parsed command intent from CLI.
#[derive(Debug, Clone)]
pub enum ParsedCommand {
    RunServer(LaunchProfile),
    Cli(CliCommand),
}

/// Top-level optional CLI commands.
#[derive(Debug, Clone, Subcommand)]
pub enum CliCommand {
    /// Generate Seiro MCP configuration snippets and project config files.
    #[command(about = "Generate Seiro MCP configuration")]
    Config(ConfigArgs),
    /// Manage bundled Codex skills (install/remove).
    #[command(about = "Manage bundled Codex skills (install/remove)")]
    Skill(SkillArgs),
}

/// `config` command container.
#[derive(Debug, Clone, Args)]
#[command(
    about = "Generate Seiro MCP configuration",
    long_about = "Generate Seiro MCP configuration.\n\nSubcommands:\n  mcp      Print a Codex MCP registration snippet.\n  project  Create a project-local seiro-mcp.toml.",
    after_help = "Hint: use `seiro-mcp config mcp` for Codex config and `seiro-mcp config project` in a project root."
)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

/// Configuration helper subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum ConfigCommand {
    /// Print TOML to paste into Codex config.
    Mcp,
    /// Create a project-local seiro-mcp.toml.
    Project(ConfigProjectArgs),
}

/// Arguments for `config project`.
#[derive(Debug, Clone, Args)]
pub struct ConfigProjectArgs {
    /// Overwrite an existing seiro-mcp.toml.
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

/// `skill` command container.
#[derive(Debug, Clone, Args)]
#[command(
    about = "Manage bundled Codex skills",
    long_about = "Manage bundled Codex skills.\n\nSubcommands:\n  install  Place bundled skills into Codex skills directory.\n  remove   Delete bundled skills from Codex skills directory.",
    after_help = "Hint: use `seiro-mcp skill install --dry-run` to preview installing the default bundled skill without modifying files."
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
    /// Skill name (defaults to `seiro-mcp-visionos-build-operator`; must start with `seiro-mcp-` when provided).
    pub skill_name: Option<String>,
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
    /// Path to seiro-mcp.toml (overrides MCP_CONFIG_PATH).
    #[arg(long = "config")]
    pub config_override: Option<std::path::PathBuf>,
    /// Optional CLI command mode.
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

impl LaunchProfileArgs {
    /// Build a `LaunchProfile` from CLI args and environment variables.
    pub fn build(self) -> Result<LaunchProfile> {
        let config_path = resolve_config_path(self.config_override)?;

        let launch_args = build_launch_args(&config_path);

        Ok(LaunchProfile {
            config_path,
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

/// Validate command-level invariants before execution.
fn validate_command(command: &CliCommand) -> Result<()> {
    use crate::cli::validate_skill_name_prefix;

    match command {
        CliCommand::Config(_) => {}
        CliCommand::Skill(skill) => match &skill.command {
            SkillCommand::Install(args) => {
                if let Some(skill_name) = &args.skill_name {
                    if !validate_skill_name_prefix(skill_name) {
                        return Err(anyhow!("invalid skill name: must start with `seiro-mcp-`"));
                    }
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
