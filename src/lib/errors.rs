use std::{io, path::PathBuf};

use config::ConfigError as ConfigLoaderError;
use rmcp::model::ErrorData;
use serde::Serialize;
use serde_json::{Map, Number, Value};
use thiserror::Error;
use zip::result::ZipError;

/// Errors that can occur while loading or validating configuration files.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to build (read) the configuration file.
    #[error("Failed to read configuration file {path}: {source}")]
    FileRead {
        path: PathBuf,
        #[source]
        source: ConfigLoaderError,
    },
    /// Failed to deserialize TOML into a struct.
    #[error("Failed to parse configuration file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ConfigLoaderError,
    },
    /// Required field is missing.
    #[error("Configuration file {path} is missing `{field}`")]
    MissingField { path: PathBuf, field: &'static str },
    /// Field failed validation.
    #[error("Configuration file {path} has invalid `{field}`: {message}")]
    InvalidField {
        path: PathBuf,
        field: &'static str,
        message: String,
    },
}

impl ConfigError {
    /// Helper to wrap `config::ConfigError` as a read failure.
    pub fn from_read_error(path: PathBuf, source: ConfigLoaderError) -> Self {
        Self::FileRead { path, source }
    }

    /// Helper to wrap `config::ConfigError` as a parse failure.
    pub fn from_parse_error(path: PathBuf, source: ConfigLoaderError) -> Self {
        Self::Parse { path, source }
    }
}

/// High-level failure types returned during a visionOS build.
#[derive(Debug, Error)]
pub enum VisionOsBuildError {
    #[error("Path outside the allowed directories was provided: {path}")]
    PathNotAllowed { path: PathBuf },
    #[error("Required SDK `{required_sdk}` is not installed")]
    MissingSdk { required_sdk: String },
    #[error("xcodebuild exited abnormally (exit={exit_code:?}): {message}")]
    CommandFailed {
        exit_code: Option<i32>,
        message: String,
    },
    #[error("visionOS build timed out after {duration_secs} seconds")]
    Timeout { duration_secs: u64 },
    #[error("Build was blocked by sandbox policy: {reason}")]
    SandboxViolated { reason: String },
    #[error("Failed to process artifacts: {message}")]
    ArtifactFailure { message: String },
}

/// Failure reasons for sandbox policy validation.
#[derive(Debug, Error)]
pub enum SandboxPolicyError {
    #[error("Path is not allowed: {path}")]
    PathNotAllowed { path: PathBuf },
    #[error("SDK `{name}` could not be detected")]
    MissingSdk { name: String },
    #[error("Developer directory `{path}` not found")]
    XcodePathUnavailable { path: PathBuf },
    #[error("Xcode license has not been accepted")]
    LicenseNotAccepted,
    #[error("DevToolsSecurity is disabled")]
    DevToolsSecurityDisabled,
    #[error("Insufficient free space for visionOS build (available={available_bytes} bytes)")]
    DiskInsufficient { available_bytes: u64 },
    #[error("Internal sandbox policy error: {message}")]
    Internal { message: String },
}

/// Errors occurring while operating on artifact directories.
#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("Failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Failed to read directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("I/O failed for file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Failed to delete artifact {path}: {source}")]
    Cleanup {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Failed to create zip ({path}): {source}")]
    Zip {
        path: PathBuf,
        #[source]
        source: ZipError,
    },
    #[error("Artifact source {path} is not a directory")]
    InvalidSource { path: PathBuf },
}

impl From<ArtifactError> for VisionOsBuildError {
    fn from(value: ArtifactError) -> Self {
        VisionOsBuildError::ArtifactFailure {
            message: value.to_string(),
        }
    }
}

/// Structured error metadata returned by MCP tools.
#[derive(Debug, Clone, Serialize)]
pub struct ToolErrorDescriptor {
    /// Error code.
    pub code: &'static str,
    /// User-facing message.
    pub message: &'static str,
    /// Recommended remediation.
    pub remediation: &'static str,
}

impl ToolErrorDescriptor {
    /// Simple constructor.
    pub const fn new(code: &'static str, message: &'static str, remediation: &'static str) -> Self {
        Self {
            code,
            message,
            remediation,
        }
    }

    /// Create a builder.
    pub fn builder(&self) -> ToolErrorDescriptorBuilder<'_> {
        ToolErrorDescriptorBuilder::new(self)
    }
}

/// Sandbox state representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxState {
    NotApplicable,
    NoViolation,
    Blocked,
}

impl SandboxState {
    fn as_str(&self) -> &'static str {
        match self {
            SandboxState::NotApplicable => "not_applicable",
            SandboxState::NoViolation => "no_violation",
            SandboxState::Blocked => "blocked",
        }
    }
}

/// Builder for error data that fails if required fields are missing.
pub struct ToolErrorDescriptorBuilder<'a> {
    descriptor: &'a ToolErrorDescriptor,
    retryable: Option<bool>,
    sandbox_state: Option<SandboxState>,
    details: Option<Value>,
    extra_fields: Map<String, Value>,
}

impl<'a> ToolErrorDescriptorBuilder<'a> {
    pub fn new(descriptor: &'a ToolErrorDescriptor) -> Self {
        Self {
            descriptor,
            retryable: None,
            sandbox_state: None,
            details: None,
            extra_fields: Map::new(),
        }
    }

    pub fn retryable(mut self, retryable: bool) -> Self {
        self.retryable = Some(retryable);
        self
    }

    pub fn sandbox_state(mut self, state: SandboxState) -> Self {
        self.sandbox_state = Some(state);
        self
    }

    pub fn details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_context_field(mut self, key: &str, value: Value) -> Self {
        self.extra_fields.insert(key.to_string(), value);
        self
    }

    pub fn with_exit_code_value(mut self, exit_code: u8) -> Self {
        let numeric = Number::from(exit_code);
        self.extra_fields
            .insert("exit_code".into(), Value::Number(numeric));
        self
    }

    pub fn build(self) -> Result<ErrorData, ToolErrorBuilderError> {
        if self.descriptor.remediation.trim().is_empty() {
            return Err(ToolErrorBuilderError::MissingRemediation {
                code: self.descriptor.code,
            });
        }
        let retryable = self
            .retryable
            .ok_or(ToolErrorBuilderError::MissingRetryable {
                code: self.descriptor.code,
            })?;
        let sandbox_state =
            self.sandbox_state
                .ok_or(ToolErrorBuilderError::MissingSandboxState {
                    code: self.descriptor.code,
                })?;

        let mut data = Map::new();
        data.insert("code".into(), Value::String(self.descriptor.code.into()));
        data.insert(
            "remediation".into(),
            Value::String(self.descriptor.remediation.into()),
        );
        data.insert("retryable".into(), Value::Bool(retryable));
        data.insert(
            "sandbox_state".into(),
            Value::String(sandbox_state.as_str().into()),
        );
        if let Some(details) = self.details {
            data.insert("details".into(), details);
        }
        for (key, value) in self.extra_fields {
            data.insert(key, value);
        }

        Ok(ErrorData::invalid_params(
            self.descriptor.message,
            Some(Value::Object(data)),
        ))
    }
}

/// Errors when required builder fields are missing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolErrorBuilderError {
    #[error("retryable is missing (code={code})")]
    MissingRetryable { code: &'static str },
    #[error("sandbox_state is missing (code={code})")]
    MissingSandboxState { code: &'static str },
    #[error("remediation is empty (code={code})")]
    MissingRemediation { code: &'static str },
}

/// Standard error for authentication mismatches.
pub const AUTH_TOKEN_MISMATCH_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "AUTH_TOKEN_MISMATCH",
    "MCP_SHARED_TOKEN does not match config.toml [auth].token",
    "Set the same token in both Codex CLI and Inspector before retrying.",
);

/// Standard error when no token is provided.
pub const MCP_TOKEN_REQUIRED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "MCP_TOKEN_REQUIRED",
    "MCP_SHARED_TOKEN is unset or shorter than 16 characters",
    "Set MCP_SHARED_TOKEN to a random string at least 16 characters long that matches config.toml.",
);

/// Standard error when executed without an MCP client.
pub const MCP_CLIENT_REQUIRED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "MCP_CLIENT_REQUIRED",
    "This binary can only be executed via an MCP client",
    "Launch through an MCP client such as `npx @modelcontextprotocol/inspector target/release/seiro-mcp`.",
);

#[cfg(test)]
mod tests {
    use rmcp::model::ErrorData;
    use serde_json::json;

    use super::*;

    const BASE_DESCRIPTOR: ToolErrorDescriptor = ToolErrorDescriptor::new(
        "sample_error",
        "Sample error",
        "Check the input before retrying.",
    );

    #[test]
    fn builder_produces_error_data_with_required_fields() {
        let error = ToolErrorDescriptorBuilder::new(&BASE_DESCRIPTOR)
            .retryable(true)
            .sandbox_state(SandboxState::NoViolation)
            .details(json!({ "info": "details" }))
            .with_context_field("job_id", json!("1234"))
            .build()
            .expect("builder must succeed");

        assert_eq!(error.message, BASE_DESCRIPTOR.message);
        let data = extract_data(&error);
        assert_eq!(
            data.get("code").and_then(|v| v.as_str()),
            Some("sample_error")
        );
        assert_eq!(
            data.get("remediation").and_then(|v| v.as_str()),
            Some("Check the input before retrying.")
        );
        assert_eq!(data.get("retryable").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            data.get("sandbox_state").and_then(|v| v.as_str()),
            Some("no_violation")
        );
        assert_eq!(data.get("details"), Some(&json!({ "info": "details" })));
        assert_eq!(data.get("job_id"), Some(&json!("1234")));
    }

    #[test]
    fn builder_fails_when_sandbox_state_missing() {
        let result = ToolErrorDescriptorBuilder::new(&BASE_DESCRIPTOR)
            .retryable(false)
            .build();
        assert_eq!(
            result.unwrap_err(),
            ToolErrorBuilderError::MissingSandboxState {
                code: BASE_DESCRIPTOR.code
            }
        );
    }

    #[test]
    fn builder_fails_when_remediation_blank() {
        const BLANK_DESCRIPTOR: ToolErrorDescriptor =
            ToolErrorDescriptor::new("blank", "blank", "");
        let result = ToolErrorDescriptorBuilder::new(&BLANK_DESCRIPTOR)
            .retryable(false)
            .sandbox_state(SandboxState::Blocked)
            .build();
        assert_eq!(
            result.unwrap_err(),
            ToolErrorBuilderError::MissingRemediation {
                code: BLANK_DESCRIPTOR.code
            }
        );
    }

    fn extract_data(error: &ErrorData) -> Map<String, Value> {
        error
            .data
            .as_ref()
            .and_then(|value| value.as_object())
            .cloned()
            .expect("error data should be an object")
    }
}
