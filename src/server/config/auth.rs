use std::path::Path;

use serde::Deserialize;

use crate::lib::errors::ConfigError;

/// Authentication settings.
#[derive(Debug, Clone)]
pub struct AuthSection {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct RawAuthSection {
    pub token: Option<String>,
}

pub fn parse_auth_section(
    raw: Option<RawAuthSection>,
    path: &Path,
) -> Result<AuthSection, ConfigError> {
    let auth_raw = raw.ok_or(ConfigError::MissingField {
        path: path.to_path_buf(),
        field: "auth",
    })?;
    let token = auth_raw
        .token
        .filter(|value| !value.trim().is_empty())
        .ok_or(ConfigError::MissingField {
            path: path.to_path_buf(),
            field: "auth.token",
        })?;

    Ok(AuthSection { token })
}
