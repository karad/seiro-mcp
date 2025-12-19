//! LaunchProfile and token/config resolution.
use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use clap::ValueEnum;

const DEFAULT_CONFIG: &str = "config.toml";
const MIN_TOKEN_LENGTH: usize = 16;
const MAX_TOKEN_LENGTH: usize = 128;
const MCP_CONFIG_ENV: &str = "MCP_CONFIG_PATH";
const MCP_SHARED_TOKEN_ENV: &str = "MCP_SHARED_TOKEN";

/// MCP transport mode.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum TransportMode {
    Stdio,
    Tcp,
}

impl TransportMode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            TransportMode::Stdio => "stdio",
            TransportMode::Tcp => "tcp",
        }
    }
}

/// Source for the shared token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    Env,
    Cli,
    Missing,
}

/// Resolved launch profile.
#[derive(Debug, Clone)]
pub struct LaunchProfile {
    pub config_path: PathBuf,
    pub transport: TransportMode,
    pub shared_token: Option<String>,
    pub token_source: TokenSource,
    pub launch_args: Vec<String>,
}

/// Resolve config path in the order: CLI override → env var → default.
pub fn resolve_config_path(override_path: Option<PathBuf>) -> Result<PathBuf> {
    let path = override_path
        .or_else(|| env::var_os(MCP_CONFIG_ENV).map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG));

    if path.is_absolute() {
        return Ok(path);
    }

    let cwd = env::current_dir().context("failed to obtain current directory")?;
    Ok(cwd.join(path))
}

/// Resolve token in the order: CLI override → env var.
pub fn resolve_token(token_override: Option<String>) -> (Option<String>, TokenSource) {
    if let Some(token) = token_override.and_then(|v| normalize_token(&v)) {
        return (Some(token), TokenSource::Cli);
    }

    if let Some(env_token) = env::var(MCP_SHARED_TOKEN_ENV)
        .ok()
        .and_then(|v| normalize_token(&v))
    {
        return (Some(env_token), TokenSource::Env);
    }

    (None, TokenSource::Missing)
}

/// Build launch arguments suitable for reproduction/logging.
pub fn build_launch_args(transport: TransportMode, config: &Path) -> Vec<String> {
    vec![
        format!("--transport={}", transport.as_str()),
        format!("--config={}", config.display()),
    ]
}

fn normalize_token(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.len() < MIN_TOKEN_LENGTH || trimmed.len() > MAX_TOKEN_LENGTH {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_shorter_than_minimum_is_rejected() {
        assert!(normalize_token("short").is_none());
        assert_eq!(
            normalize_token("valid-token-123456"),
            Some("valid-token-123456".to_string())
        );
    }
}
