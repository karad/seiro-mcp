use tracing::{debug, info};

use super::{ServerConfig, CONFIG_ENV_KEY, DEFAULT_CONFIG_PATH};

pub fn log_env_source(path: &std::path::Path, from_env: bool) {
    if from_env {
        info!(
            target: "rmcp_sample::config",
            path = %path.display(),
            "Loading configuration using MCP_CONFIG_PATH environment variable"
        );
    } else {
        debug!(
            target: "rmcp_sample::config",
            path = %path.display(),
            env = CONFIG_ENV_KEY,
            default = DEFAULT_CONFIG_PATH,
            "MCP_CONFIG_PATH not set; using default config.toml"
        );
    }
}

pub fn log_loaded(config: &ServerConfig) {
    info!(
        target: "rmcp_sample::config",
        path = %config.source_path.display(),
        host = %config.server.host,
        port = config.server.port,
        visionos_allowed_paths = %config.visionos.allowed_paths.len(),
        visionos_allowed_schemes = %config.visionos.allowed_schemes.len(),
        max_build_minutes = config.visionos.max_build_minutes,
        artifact_ttl_secs = config.visionos.artifact_ttl_secs,
        "Configuration file loaded successfully"
    );
}
