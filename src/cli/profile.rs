//! LaunchProfile and config resolution.
use std::{
    env,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

const DEFAULT_CONFIG: &str = "seiro-mcp.toml";
const MCP_CONFIG_ENV: &str = "MCP_CONFIG_PATH";

/// Resolved launch profile.
#[derive(Debug, Clone)]
pub struct LaunchProfile {
    pub config_path: PathBuf,
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

/// Build launch arguments suitable for reproduction/logging.
pub fn build_launch_args(config: &Path) -> Vec<String> {
    vec![format!("--config={}", config.display())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_default_config_resolves_to_seiro_mcp_toml_in_cwd() {
        let cwd = env::current_dir().expect("cwd should be available");
        let config = resolve_config_path(None).expect("default config path should resolve");
        assert_eq!(config, cwd.join("seiro-mcp.toml"));
    }

    #[test]
    fn explicit_config_path_still_wins() {
        let config = resolve_config_path(Some(PathBuf::from("/tmp/custom.toml")))
            .expect("absolute override should resolve");
        assert_eq!(config, PathBuf::from("/tmp/custom.toml"));
    }
}
