use crate::{cli::LaunchProfile, server::config::ServerConfig};

/// Build the `ServerInfo.instructions` string shown to MCP clients.
pub fn build_instructions(profile: &LaunchProfile, config: &ServerConfig) -> String {
    format!(
        "Loaded config {path}; waiting in {transport} mode (host={host}, port={port}). Set MCP_SHARED_TOKEN when connecting from Codex CLI / Inspector.",
        path = config.source_path.display(),
        transport = profile.transport.as_str(),
        host = config.server.host,
        port = config.server.port
    )
}
