//! Load and validate server configuration.
use std::{env, path::PathBuf};

use serde::Deserialize;
use tracing::{error, info};

use crate::lib::errors::ConfigError;

pub mod auth;
pub mod server;
pub mod telemetry;
pub mod visionos;

pub use auth::{parse_auth_section, AuthSection, RawAuthSection};
pub use server::{
    parse_server_section, parse_tools_section, RawServerSection, RawToolsSection, ServerSection,
    DEFAULT_HOST, DEFAULT_PORT,
};
pub use visionos::{
    parse_visionos_section, RawVisionOsConfig, VisionOsConfig, DEFAULT_ARTIFACT_TTL_SECS,
    DEFAULT_CLEANUP_SCHEDULE_SECS, DEFAULT_MAX_BUILD_MINUTES, DEFAULT_VISIONOS_DESTINATION,
    DEFAULT_XCODEBUILD_PATH,
};

const CONFIG_ENV_KEY: &str = "MCP_CONFIG_PATH";
const DEFAULT_CONFIG_PATH: &str = "config.toml";

/// Top-level configuration container.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub server: ServerSection,
    pub auth: AuthSection,
    pub visionos: VisionOsConfig,
    pub source_path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RawServerConfig {
    server: Option<RawServerSection>,
    auth: Option<RawAuthSection>,
    tools: Option<RawToolsSection>,
    visionos: Option<RawVisionOsConfig>,
}

impl ServerConfig {
    /// Prefer `MCP_CONFIG_PATH` if set; otherwise read `config.toml`.
    pub fn load_from_env_or_default() -> Result<Self, ConfigError> {
        let (path, from_env) = match env::var(CONFIG_ENV_KEY) {
            Ok(value) if !value.trim().is_empty() => (PathBuf::from(value), true),
            _ => (PathBuf::from(DEFAULT_CONFIG_PATH), false),
        };

        telemetry::log_env_source(&path, from_env);
        Self::load_from_path(path)
    }

    /// Load configuration from a specific path.
    pub fn load_from_path(path: PathBuf) -> Result<Self, ConfigError> {
        info!(
            target: "rmcp_sample::config",
            path = %path.display(),
            "Starting configuration load"
        );

        let builder = config::Config::builder().add_source(config::File::from(path.clone()));
        let document = builder.build().map_err(|err| {
            let error = ConfigError::from_read_error(path.clone(), err);
            error!(
                target: "rmcp_sample::config",
                path = %path.display(),
                reason = %error,
                "Failed to read configuration file"
            );
            error
        })?;

        let raw: RawServerConfig = document.try_deserialize().map_err(|err| {
            let error = ConfigError::from_parse_error(path.clone(), err);
            error!(
                target: "rmcp_sample::config",
                path = %path.display(),
                reason = %error,
                "Failed to parse configuration file"
            );
            error
        })?;

        let config = Self::from_raw(raw, path.clone()).map_err(|err| {
            error!(
                target: "rmcp_sample::config",
                path = %path.display(),
                reason = %err,
                "Failed to validate configuration file"
            );
            err
        })?;

        telemetry::log_loaded(&config);
        Ok(config)
    }

    fn from_raw(raw: RawServerConfig, path: PathBuf) -> Result<Self, ConfigError> {
        let server = parse_server_section(raw.server, &path)?;
        let auth = parse_auth_section(raw.auth, &path)?;
        parse_tools_section(raw.tools, &path)?;
        let visionos = parse_visionos_section(path.clone(), raw.visionos)?;

        Ok(Self {
            server,
            auth,
            visionos,
            source_path: path,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        path::{Path, PathBuf},
    };

    use crate::lib::errors::ConfigError;

    use super::ServerConfig;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn with_config_env<T>(path: &Path, test: impl FnOnce() -> T) -> T {
        let original = env::var(super::CONFIG_ENV_KEY).ok();
        env::set_var(super::CONFIG_ENV_KEY, path);
        let result = test();
        match original {
            Some(value) => env::set_var(super::CONFIG_ENV_KEY, value),
            None => env::remove_var(super::CONFIG_ENV_KEY),
        }
        result
    }

    #[test]
    fn load_valid_config() {
        let config = ServerConfig::load_from_path(fixture_path("config_valid.toml"))
            .expect("config_valid.toml should load");

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8787);
        assert_eq!(config.auth.token, "valid-token-123456");
        assert_eq!(
            config.visionos.allowed_paths,
            vec![PathBuf::from("/Users/example/codex/workspaces")]
        );
        assert_eq!(
            config.visionos.allowed_schemes,
            vec![String::from("VisionApp"), String::from("VisionToolbox")]
        );
        assert_eq!(
            config.visionos.default_destination,
            "platform=visionOS Simulator,name=Apple Vision Pro"
        );
        assert_eq!(config.visionos.required_sdks.len(), 2);
        assert_eq!(
            config.visionos.xcode_path,
            PathBuf::from("/Applications/Xcode.app/Contents/Developer")
        );
        assert_eq!(
            config.visionos.xcodebuild_path,
            PathBuf::from("/usr/bin/xcodebuild")
        );
        assert_eq!(config.visionos.max_build_minutes, 20);
        assert_eq!(config.visionos.artifact_ttl_secs, 600);
        assert_eq!(config.visionos.cleanup_schedule_secs, 60);
    }

    #[test]
    fn missing_token_returns_error() {
        let error = ServerConfig::load_from_path(fixture_path("config_missing_token.toml"))
            .expect_err("should error when token is missing");

        match error {
            ConfigError::MissingField { field, .. } => assert_eq!(field, "auth.token"),
            other => panic!("Unexpected error: {other:?}", other = other),
        }
    }

    #[test]
    fn invalid_port_returns_error() {
        let error = ServerConfig::load_from_path(fixture_path("config_invalid_port.toml"))
            .expect_err("should error for an invalid port");

        match error {
            ConfigError::InvalidField { field, .. } => assert_eq!(field, "server.port"),
            other => panic!("Unexpected error: {other:?}", other = other),
        }
    }

    #[test]
    fn load_config_from_env_override() {
        let path = fixture_path("config_valid.toml");
        let config = with_config_env(&path, || {
            ServerConfig::load_from_env_or_default().expect("should load via environment variable")
        });

        assert_eq!(config.source_path, path);
        assert_eq!(config.auth.token, "valid-token-123456");
        assert_eq!(
            config.visionos.xcodebuild_path,
            PathBuf::from("/usr/bin/xcodebuild")
        );
        assert!(config
            .visionos
            .allowed_schemes
            .iter()
            .any(|s| s == "VisionApp"));
    }

    #[test]
    fn missing_visionos_section_returns_error() {
        let error = ServerConfig::load_from_path(fixture_path("config_missing_visionos.toml"))
            .expect_err("should error when visionos section is missing");

        match error {
            ConfigError::MissingField { field, .. } => assert_eq!(field, "visionos"),
            other => panic!("Unexpected error: {other:?}", other = other),
        }
    }

    #[test]
    fn visionos_allowed_paths_must_be_absolute() {
        let error = ServerConfig::load_from_path(fixture_path("config_relative_allowed_path.toml"))
            .expect_err("should error on relative path");

        match error {
            ConfigError::InvalidField { field, .. } => assert_eq!(field, "visionos.allowed_paths"),
            other => panic!("Unexpected error: {other:?}", other = other),
        }
    }

    #[test]
    fn missing_allowed_schemes_returns_error() {
        let error =
            ServerConfig::load_from_path(fixture_path("config_missing_allowed_schemes.toml"))
                .expect_err("should error when allowed_schemes is missing");

        match error {
            ConfigError::MissingField { field, .. } => {
                assert_eq!(field, "visionos.allowed_schemes")
            }
            other => panic!("Unexpected error: {other:?}", other = other),
        }
    }

    #[test]
    fn missing_allowed_paths_returns_error() {
        let error = ServerConfig::load_from_path(fixture_path("config_missing_allowed_paths.toml"))
            .expect_err("should error when allowed_paths is missing");

        match error {
            ConfigError::MissingField { field, .. } => assert_eq!(field, "visionos.allowed_paths"),
            other => panic!("Unexpected error: {other:?}", other = other),
        }
    }

    #[test]
    fn empty_allowed_paths_is_accepted() {
        let config = ServerConfig::load_from_path(fixture_path("config_empty_allowed_paths.toml"))
            .expect("should accept empty allowed_paths to disable allowlist checks");

        assert!(config.visionos.allowed_paths.is_empty());
        assert!(!config.visionos.allowed_schemes.is_empty());
    }

    #[test]
    fn empty_allowed_schemes_is_accepted() {
        let config =
            ServerConfig::load_from_path(fixture_path("config_empty_allowed_schemes.toml"))
                .expect("should accept empty allowed_schemes to disable allowlist checks");

        assert!(config.visionos.allowed_schemes.is_empty());
        assert!(!config.visionos.allowed_paths.is_empty());
    }
}
