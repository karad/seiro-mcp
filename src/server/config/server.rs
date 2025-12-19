use std::path::Path;

use serde::Deserialize;

use crate::lib::errors::ConfigError;

pub const DEFAULT_HOST: &str = "127.0.0.1";
pub const DEFAULT_PORT: u16 = 8787;

/// Server socket settings.
#[derive(Debug, Clone)]
pub struct ServerSection {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Default)]
pub struct RawServerSection {
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub struct RawToolsSection {}

pub fn parse_server_section(
    raw: Option<RawServerSection>,
    path: &Path,
) -> Result<ServerSection, ConfigError> {
    let server_raw = raw.unwrap_or_default();
    let host = server_raw.host.unwrap_or_else(|| DEFAULT_HOST.to_string());
    let port = server_raw.port.unwrap_or(DEFAULT_PORT);
    validate_port(port, path)?;
    Ok(ServerSection { host, port })
}

pub fn parse_tools_section(_raw: Option<RawToolsSection>, _path: &Path) -> Result<(), ConfigError> {
    Ok(())
}

fn validate_port(port: u16, path: &Path) -> Result<(), ConfigError> {
    if (1024..=65535).contains(&port) {
        return Ok(());
    }

    Err(ConfigError::InvalidField {
        path: path.to_path_buf(),
        field: "server.port",
        message: "Use a port in the range 1024-65535".into(),
    })
}
