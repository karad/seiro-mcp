use std::{collections::BTreeMap, path::PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::server::config::VisionOsConfig;

const MAX_PROJECT_PATH_LEN: usize = 512;
const MAX_SCHEME_LEN: usize = 128;
const MAX_DESTINATION_LEN: usize = 256;
const MAX_EXTRA_ARGS: usize = 5;
const MAX_EXTRA_ARG_LEN: usize = 64;

/// `xcodebuild` flags allowed in `extra_args`.
pub const ALLOWED_EXTRA_ARGS: &[&str] = &[
    "-quiet",
    "-UseModernBuildSystem=YES",
    "-skipPackagePluginValidation",
    "-allowProvisioningUpdates",
];

/// Environment variables allowed in `env_overrides`.
pub const ALLOWED_ENV_OVERRIDES: &[&str] = &[
    "DEVELOPER_DIR",
    "NSUnbufferedIO",
    "CI",
    "MOCK_XCODEBUILD_BEHAVIOR",
];

/// Build configuration.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BuildConfiguration {
    #[default]
    Debug,
    Release,
}

impl BuildConfiguration {
    pub fn as_str(&self) -> &'static str {
        match self {
            BuildConfiguration::Debug => "Debug",
            BuildConfiguration::Release => "Release",
        }
    }
}

/// Input for `build_visionos_app`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VisionOsBuildRequest {
    pub project_path: PathBuf,
    #[serde(default)]
    pub workspace: Option<PathBuf>,
    pub scheme: String,
    #[serde(default)]
    pub configuration: BuildConfiguration,
    #[serde(default = "default_destination")]
    pub destination: String,
    #[serde(default)]
    pub clean: bool,
    #[serde(default)]
    pub extra_args: Vec<String>,
    #[serde(default)]
    pub env_overrides: BTreeMap<String, String>,
}

impl VisionOsBuildRequest {
    /// Validate the input and ensure it complies with the sandbox policy.
    pub fn validate(&self, policy: &VisionOsConfig) -> Result<(), BuildRequestValidationError> {
        if self.project_path.as_os_str().is_empty() {
            return Err(BuildRequestValidationError::MissingProjectPath);
        }
        if !crate::lib::paths::is_nonempty_absolute(&self.project_path) {
            return Err(BuildRequestValidationError::ProjectPathNotAbsolute);
        }
        if self.project_path.to_string_lossy().len() > MAX_PROJECT_PATH_LEN {
            return Err(BuildRequestValidationError::ProjectPathTooLong);
        }
        if !policy.allowed_paths.is_empty()
            && !crate::lib::visionos::is_allowed_path(&self.project_path, &policy.allowed_paths)
        {
            return Err(BuildRequestValidationError::ProjectPathNotAllowed {
                path: self.project_path.clone(),
            });
        }

        if let Some(workspace) = &self.workspace {
            if !crate::lib::paths::is_nonempty_absolute(workspace) {
                return Err(BuildRequestValidationError::WorkspaceNotAllowed {
                    path: workspace.clone(),
                });
            }
            if !policy.allowed_paths.is_empty()
                && !crate::lib::visionos::is_allowed_path(workspace, &policy.allowed_paths)
            {
                return Err(BuildRequestValidationError::WorkspaceNotAllowed {
                    path: workspace.clone(),
                });
            }
        }

        if self.scheme.trim().is_empty() {
            return Err(BuildRequestValidationError::MissingScheme);
        }
        if self.scheme.chars().count() > MAX_SCHEME_LEN {
            return Err(BuildRequestValidationError::SchemeTooLong {
                length: self.scheme.chars().count(),
            });
        }
        if !policy.allowed_schemes.is_empty()
            && !policy
                .allowed_schemes
                .iter()
                .any(|allowed| allowed == &self.scheme)
        {
            return Err(BuildRequestValidationError::SchemeNotAllowed {
                scheme: self.scheme.clone(),
            });
        }

        let destination = self.destination.trim();
        if destination.is_empty() {
            return Err(BuildRequestValidationError::DestinationEmpty);
        }
        if destination.len() > MAX_DESTINATION_LEN {
            return Err(BuildRequestValidationError::DestinationTooLong {
                length: destination.len(),
            });
        }
        if !destination.contains("platform=") {
            return Err(BuildRequestValidationError::DestinationMissingPlatform);
        }

        if self.extra_args.len() > MAX_EXTRA_ARGS {
            return Err(BuildRequestValidationError::TooManyExtraArgs {
                count: self.extra_args.len(),
            });
        }
        for arg in &self.extra_args {
            if arg.len() > MAX_EXTRA_ARG_LEN {
                return Err(BuildRequestValidationError::ExtraArgTooLong {
                    arg: arg.clone(),
                    length: arg.len(),
                });
            }
            if !ALLOWED_EXTRA_ARGS.contains(&arg.as_str()) {
                return Err(BuildRequestValidationError::ExtraArgNotAllowed { arg: arg.clone() });
            }
        }

        for key in self.env_overrides.keys() {
            if !ALLOWED_ENV_OVERRIDES.contains(&key.as_str()) {
                return Err(BuildRequestValidationError::EnvOverrideNotAllowed {
                    key: key.clone(),
                });
            }
        }

        Ok(())
    }
}

/// Default destination value.
pub fn default_destination() -> String {
    "platform=visionOS Simulator,name=Apple Vision Pro".to_string()
}

/// Input validation errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuildRequestValidationError {
    #[error("project_path is required")]
    MissingProjectPath,
    #[error("project_path must be absolute")]
    ProjectPathNotAbsolute,
    #[error("project_path is too long (max {MAX_PROJECT_PATH_LEN} characters)")]
    ProjectPathTooLong,
    #[error("project_path `{path}` is outside the allowlist")]
    ProjectPathNotAllowed { path: PathBuf },
    #[error("workspace `{path}` is outside the allowlist")]
    WorkspaceNotAllowed { path: PathBuf },
    #[error("scheme is required")]
    MissingScheme,
    #[error("scheme is too long ({length} characters)")]
    SchemeTooLong { length: usize },
    #[error("scheme `{scheme}` is not included in the config allowlist")]
    SchemeNotAllowed { scheme: String },
    #[error("destination is required")]
    DestinationEmpty,
    #[error("destination is too long ({length} characters)")]
    DestinationTooLong { length: usize },
    #[error("destination must include `platform=`")]
    DestinationMissingPlatform,
    #[error("extra_args contains a disallowed value `{arg}`")]
    ExtraArgNotAllowed { arg: String },
    #[error("extra_args exceeds the allowed count (count={count})")]
    TooManyExtraArgs { count: usize },
    #[error("extra_args `{arg}` is too long ({length} characters)")]
    ExtraArgTooLong { arg: String, length: usize },
    #[error("env_overrides `{key}` is not permitted")]
    EnvOverrideNotAllowed { key: String },
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::server::config::VisionOsConfig;

    use super::*;

    fn sample_config() -> VisionOsConfig {
        let workspace = absolute_fixtures_path("tests/fixtures/visionos/workspace");
        VisionOsConfig {
            allowed_paths: vec![workspace],
            allowed_schemes: vec!["VisionApp".into(), "VisionToolbox".into()],
            default_destination: "platform=visionOS Simulator,name=Apple Vision Pro".into(),
            required_sdks: vec!["visionOS".into(), "visionOS Simulator".into()],
            xcode_path: PathBuf::from("/Applications/Xcode.app/Contents/Developer"),
            xcodebuild_path: PathBuf::from("/usr/bin/xcodebuild"),
            max_build_minutes: 20,
            artifact_ttl_secs: 600,
            cleanup_schedule_secs: 60,
        }
    }

    fn absolute_fixtures_path(relative: &str) -> PathBuf {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root.join(relative)
    }

    fn base_request() -> VisionOsBuildRequest {
        VisionOsBuildRequest {
            project_path: absolute_fixtures_path("tests/fixtures/visionos/workspace/VisionApp"),
            workspace: None,
            scheme: "VisionApp".into(),
            configuration: BuildConfiguration::Debug,
            destination: "platform=visionOS Simulator,name=Apple Vision Pro".into(),
            clean: false,
            extra_args: vec![],
            env_overrides: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn missing_project_path_is_rejected() {
        let mut request = base_request();
        request.project_path = PathBuf::new();

        let error = request
            .validate(&sample_config())
            .expect_err("missing project_path should produce an error");

        assert_eq!(error, BuildRequestValidationError::MissingProjectPath);
    }

    #[test]
    fn extra_args_outside_allowlist_are_rejected() {
        let mut request = base_request();
        request.extra_args = vec!["--unsupported-flag".into()];

        let error = request
            .validate(&sample_config())
            .expect_err("disallowed extra_args should produce an error");

        assert_eq!(
            error,
            BuildRequestValidationError::ExtraArgNotAllowed {
                arg: "--unsupported-flag".into()
            }
        );
    }

    #[test]
    fn scheme_not_in_allowlist_is_rejected() {
        let mut request = base_request();
        request.scheme = "UnknownScheme".into();

        let error = request
            .validate(&sample_config())
            .expect_err("disallowed scheme should produce an error");

        assert_eq!(
            error,
            BuildRequestValidationError::SchemeNotAllowed {
                scheme: "UnknownScheme".into()
            }
        );
    }

    #[test]
    fn destination_longer_than_limit_is_rejected() {
        let mut request = base_request();
        request.destination = "x".repeat(300);

        let error = request
            .validate(&sample_config())
            .expect_err("destination exceeding limit should produce an error");

        assert_eq!(
            error,
            BuildRequestValidationError::DestinationTooLong { length: 300 }
        );
    }

    #[test]
    fn allowlist_checks_are_skipped_when_policy_lists_are_empty() {
        let mut request = base_request();
        request.project_path = PathBuf::from("/tmp/project-outside-allowlist");
        request.workspace = Some(PathBuf::from("/tmp/workspace-outside-allowlist"));
        request.scheme = "UnknownScheme".into();

        let mut config = sample_config();
        config.allowed_paths = vec![];
        config.allowed_schemes = vec![];

        request
            .validate(&config)
            .expect("allowlist checks should be skipped when lists are empty");
    }
}
