use crate::{cli::LaunchProfile, server::config::ServerConfig};

/// Build the `ServerInfo.instructions` string shown to MCP clients.
pub fn build_instructions(_profile: &LaunchProfile, config: &ServerConfig) -> String {
    format!(
        "Loaded config {path}; waiting in stdio mode from a local MCP client.",
        path = config.source_path.display(),
    )
}
