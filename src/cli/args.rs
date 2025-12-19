//! CLI argument definitions and `LaunchProfile` construction.
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use super::{build_launch_args, resolve_config_path, resolve_token, LaunchProfile, TransportMode};

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
}
