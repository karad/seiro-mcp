use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::lib::errors::ConfigError;

pub const DEFAULT_VISIONOS_DESTINATION: &str = "platform=visionOS Simulator,name=Apple Vision Pro";
pub const DEFAULT_MAX_BUILD_MINUTES: u16 = 20;
pub const DEFAULT_ARTIFACT_TTL_SECS: u32 = 600;
pub const DEFAULT_CLEANUP_SCHEDULE_SECS: u32 = 60;
pub const DEFAULT_REQUIRED_SDKS: &[&str] = &["visionOS", "visionOS Simulator"];
pub const DEFAULT_XCODEBUILD_PATH: &str = "/usr/bin/xcodebuild";

/// visionOS configuration section.
#[derive(Debug, Clone)]
pub struct VisionOsConfig {
    pub allowed_paths: Vec<PathBuf>,
    pub allowed_schemes: Vec<String>,
    pub default_destination: String,
    pub required_sdks: Vec<String>,
    pub xcode_path: PathBuf,
    pub xcodebuild_path: PathBuf,
    pub max_build_minutes: u16,
    pub artifact_ttl_secs: u32,
    pub cleanup_schedule_secs: u32,
}

#[derive(Debug, Deserialize)]
pub struct RawVisionOsConfig {
    pub allowed_paths: Option<Vec<PathBuf>>,
    pub allowed_schemes: Option<Vec<String>>,
    pub default_destination: Option<String>,
    pub required_sdks: Option<Vec<String>>,
    pub xcode_path: Option<PathBuf>,
    pub xcodebuild_path: Option<PathBuf>,
    pub max_build_minutes: Option<u16>,
    pub artifact_ttl_secs: Option<u32>,
    pub cleanup_schedule_secs: Option<u32>,
}

pub fn parse_visionos_section(
    path: PathBuf,
    raw: Option<RawVisionOsConfig>,
) -> Result<VisionOsConfig, ConfigError> {
    let visionos_raw = raw.ok_or(ConfigError::MissingField {
        path: path.clone(),
        field: "visionos",
    })?;

    let allowed_paths = visionos_raw
        .allowed_paths
        .ok_or(ConfigError::MissingField {
            path: path.clone(),
            field: "visionos.allowed_paths",
        })?;
    validate_allowed_paths(path.as_path(), &allowed_paths)?;

    let allowed_schemes = visionos_raw
        .allowed_schemes
        .ok_or(ConfigError::MissingField {
            path: path.clone(),
            field: "visionos.allowed_schemes",
        })?;
    validate_allowed_schemes(path.as_path(), &allowed_schemes)?;

    let default_destination = visionos_raw
        .default_destination
        .unwrap_or_else(|| DEFAULT_VISIONOS_DESTINATION.to_string());
    validate_destination(path.as_path(), &default_destination)?;

    let required_sdks = visionos_raw.required_sdks.unwrap_or_else(|| {
        DEFAULT_REQUIRED_SDKS
            .iter()
            .map(|sdk| sdk.to_string())
            .collect()
    });
    validate_required_sdks(path.as_path(), &required_sdks)?;

    let xcode_path = visionos_raw.xcode_path.ok_or(ConfigError::MissingField {
        path: path.clone(),
        field: "visionos.xcode_path",
    })?;
    validate_xcode_path(path.as_path(), &xcode_path)?;

    let xcodebuild_path = visionos_raw
        .xcodebuild_path
        .unwrap_or_else(|| PathBuf::from(DEFAULT_XCODEBUILD_PATH));
    validate_xcodebuild_path(path.as_path(), &xcodebuild_path)?;

    let max_build_minutes = visionos_raw
        .max_build_minutes
        .unwrap_or(DEFAULT_MAX_BUILD_MINUTES);
    validate_build_minutes(path.as_path(), max_build_minutes)?;

    let artifact_ttl_secs = visionos_raw
        .artifact_ttl_secs
        .unwrap_or(DEFAULT_ARTIFACT_TTL_SECS);
    validate_ttl_secs(path.as_path(), artifact_ttl_secs)?;

    let cleanup_schedule_secs = visionos_raw
        .cleanup_schedule_secs
        .unwrap_or(DEFAULT_CLEANUP_SCHEDULE_SECS);
    validate_cleanup_interval(path.as_path(), cleanup_schedule_secs)?;

    Ok(VisionOsConfig {
        allowed_paths,
        allowed_schemes,
        default_destination,
        required_sdks,
        xcode_path,
        xcodebuild_path,
        max_build_minutes,
        artifact_ttl_secs,
        cleanup_schedule_secs,
    })
}

fn validate_allowed_paths(path: &Path, allowed_paths: &[PathBuf]) -> Result<(), ConfigError> {
    if allowed_paths.is_empty() {
        return Ok(());
    }
    for entry in allowed_paths {
        if entry.as_os_str().is_empty() || !entry.is_absolute() {
            return Err(ConfigError::InvalidField {
                path: path.to_path_buf(),
                field: "visionos.allowed_paths",
                message: format!("Only absolute paths are allowed: {}", entry.display()),
            });
        }
    }
    Ok(())
}

fn validate_allowed_schemes(path: &Path, schemes: &[String]) -> Result<(), ConfigError> {
    if schemes.is_empty() {
        return Ok(());
    }
    for scheme in schemes {
        if scheme.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                path: path.to_path_buf(),
                field: "visionos.allowed_schemes",
                message: "Scheme names cannot be empty".into(),
            });
        }
        if scheme.chars().count() > 128 {
            return Err(ConfigError::InvalidField {
                path: path.to_path_buf(),
                field: "visionos.allowed_schemes",
                message: format!("Scheme length exceeds 128 characters: {scheme}"),
            });
        }
    }
    Ok(())
}

fn validate_destination(path: &Path, destination: &str) -> Result<(), ConfigError> {
    let trimmed = destination.trim();
    if trimmed.is_empty() || trimmed.len() > 256 || !trimmed.contains("platform=") {
        return Err(ConfigError::InvalidField {
            path: path.to_path_buf(),
            field: "visionos.default_destination",
            message: "Provide a 1-256 character string that includes platform=".into(),
        });
    }
    Ok(())
}

fn validate_required_sdks(path: &Path, sdks: &[String]) -> Result<(), ConfigError> {
    if sdks.is_empty() {
        return Err(ConfigError::InvalidField {
            path: path.to_path_buf(),
            field: "visionos.required_sdks",
            message: "Specify at least one SDK name".into(),
        });
    }

    for sdk in sdks {
        if sdk.trim().is_empty() {
            return Err(ConfigError::InvalidField {
                path: path.to_path_buf(),
                field: "visionos.required_sdks",
                message: "SDK names cannot be empty".into(),
            });
        }
    }
    Ok(())
}

fn validate_xcode_path(path: &Path, xcode_path: &Path) -> Result<(), ConfigError> {
    if xcode_path.as_os_str().is_empty() || !xcode_path.is_absolute() {
        return Err(ConfigError::InvalidField {
            path: path.to_path_buf(),
            field: "visionos.xcode_path",
            message: "Provide an absolute path to the Developer directory".into(),
        });
    }
    Ok(())
}

fn validate_xcodebuild_path(path: &Path, xcodebuild_path: &Path) -> Result<(), ConfigError> {
    if xcodebuild_path.as_os_str().is_empty() || !xcodebuild_path.is_absolute() {
        return Err(ConfigError::InvalidField {
            path: path.to_path_buf(),
            field: "visionos.xcodebuild_path",
            message: "Provide an absolute path to the xcodebuild executable".into(),
        });
    }
    Ok(())
}

fn validate_build_minutes(path: &Path, minutes: u16) -> Result<(), ConfigError> {
    if !(1..=60).contains(&minutes) {
        return Err(ConfigError::InvalidField {
            path: path.to_path_buf(),
            field: "visionos.max_build_minutes",
            message: "Specify a value between 1 and 60 minutes".into(),
        });
    }
    Ok(())
}

fn validate_ttl_secs(path: &Path, ttl: u32) -> Result<(), ConfigError> {
    if !(60..=3600).contains(&ttl) {
        return Err(ConfigError::InvalidField {
            path: path.to_path_buf(),
            field: "visionos.artifact_ttl_secs",
            message: "Specify a value between 60 and 3600 seconds".into(),
        });
    }
    Ok(())
}

fn validate_cleanup_interval(path: &Path, interval: u32) -> Result<(), ConfigError> {
    if !(30..=1800).contains(&interval) {
        return Err(ConfigError::InvalidField {
            path: path.to_path_buf(),
            field: "visionos.cleanup_schedule_secs",
            message: "Specify a value between 30 and 1800 seconds".into(),
        });
    }
    Ok(())
}
