//! Request and response definitions for the sandbox policy validation tool.
//!
//! Phase 4 implements the `validate_sandbox_policy` tool to validate allowed paths,
//! required SDKs, DevToolsSecurity, and disk space.
mod probe;

use std::{env, path::Path, path::PathBuf};

use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    lib::{
        errors::{SandboxPolicyError, SandboxState, ToolErrorDescriptor},
        visionos as visionos_helpers,
    },
    server::config::VisionOsConfig,
};

use probe::SdkInventory;
pub use probe::{EnvSandboxProbe, SandboxProbe, SystemSandboxProbe};

const PATH_NOT_ALLOWED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "path_not_allowed",
    "project_path is outside the allowed paths",
    "Update visionos.allowed_paths in config.toml and restart the MCP server.",
);
const SDK_MISSING_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "sdk_missing",
    "Required SDK not found",
    "Inspect details.diagnostics (probe_mode, effective_required_sdks, detected_sdks_*), then add the visionOS SDK via Xcode > Settings > Platforms.",
);
const XCODE_UNLICENSED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "xcode_unlicensed",
    "Xcode license has not been accepted",
    "Run `sudo xcodebuild -license` to accept the license.",
);
const DEVTOOLS_DISABLED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "devtools_security_disabled",
    "DevToolsSecurity is disabled",
    "Run `DevToolsSecurity -enable` to allow debugging from Xcode.",
);
const DISK_INSUFFICIENT_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "disk_insufficient",
    "Insufficient free space for a visionOS build",
    "Remove unnecessary files where the project is stored and ensure enough free space.",
);
const SANDBOX_INTERNAL_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "sandbox_internal_error",
    "Internal error occurred during sandbox policy validation",
    "Check the logs and contact a developer if retrying does not resolve the issue.",
);

const MIN_DISK_BYTES: u64 = 20 * 1024 * 1024 * 1024; // 20GB

/// Input for `validate_sandbox_policy`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxPolicyRequest {
    pub project_path: PathBuf,
    #[serde(default = "default_required_sdks")]
    pub required_sdks: Vec<String>,
    #[serde(default)]
    pub xcode_path: Option<PathBuf>,
}

fn default_required_sdks() -> Vec<String> {
    vec!["visionOS".into(), "visionOS Simulator".into()]
}

/// Overall status of the validation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SandboxStatus {
    Ok,
    Error,
}

/// Result of an individual check.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SandboxCheckResult {
    Pass,
    Fail,
}

/// Details for a single validation check.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxCheck {
    pub name: String,
    pub result: SandboxCheckResult,
    pub details: String,
}

/// Response from `validate_sandbox_policy`.
///
/// Compatibility note:
/// - Existing `status`/`checks` fields are kept stable.
/// - Diagnostics-related fields must be additive so existing clients continue to
///   parse responses without schema-breaking changes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SandboxPolicyResponse {
    pub status: SandboxStatus,
    pub checks: Vec<SandboxCheck>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<SandboxDiagnostics>,
}

/// Diagnostics data captured during sandbox validation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SandboxDiagnostics {
    pub probe_mode: String,
    pub effective_developer_dir: String,
    pub effective_required_sdks: Vec<String>,
    pub detected_sdks_raw: Vec<String>,
    pub detected_sdks_normalized: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xcodebuild_invocation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Input for `inspect_xcode_sdks`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InspectXcodeSdksRequest {
    #[serde(default = "default_required_sdks")]
    pub required_sdks: Vec<String>,
    #[serde(default)]
    pub xcode_path: Option<PathBuf>,
}

/// Response from `inspect_xcode_sdks`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InspectXcodeSdksResponse {
    pub status: SandboxStatus,
    pub probe_mode: String,
    pub developer_dir: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xcode_select_path: Option<String>,
    pub required_sdks: Vec<String>,
    pub detected_sdks_raw: Vec<String>,
    pub detected_sdks_normalized: Vec<String>,
    pub missing_required_sdks: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Structured failure for sandbox validation with optional diagnostics context.
#[derive(Debug)]
pub struct SandboxValidationFailure {
    pub error: SandboxPolicyError,
    pub diagnostics: Option<SandboxDiagnostics>,
}

/// Execute sandbox policy validation.
pub async fn validate_sandbox_policy(
    request: SandboxPolicyRequest,
    config: &VisionOsConfig,
) -> Result<SandboxPolicyResponse, SandboxValidationFailure> {
    match env::var("VISIONOS_SANDBOX_PROBE").ok().as_deref() {
        Some("env") | Some("mock") => {
            let probe = EnvSandboxProbe;
            validate_sandbox_policy_with_probe_mode(request, config, &probe, "env").await
        }
        _ => {
            let probe = SystemSandboxProbe;
            validate_sandbox_policy_with_probe_mode(request, config, &probe, "system").await
        }
    }
}

/// Inspect SDK detection context using the same probe path as sandbox validation.
pub async fn inspect_xcode_sdks(
    request: InspectXcodeSdksRequest,
    config: &VisionOsConfig,
) -> Result<InspectXcodeSdksResponse, SandboxValidationFailure> {
    match env::var("VISIONOS_SANDBOX_PROBE").ok().as_deref() {
        Some("env") | Some("mock") => {
            let probe = EnvSandboxProbe;
            inspect_xcode_sdks_with_probe_mode(request, config, &probe, "env").await
        }
        _ => {
            let probe = SystemSandboxProbe;
            inspect_xcode_sdks_with_probe_mode(request, config, &probe, "system").await
        }
    }
}

async fn inspect_xcode_sdks_with_probe_mode<P: SandboxProbe>(
    request: InspectXcodeSdksRequest,
    config: &VisionOsConfig,
    probe: &P,
    probe_mode: &str,
) -> Result<InspectXcodeSdksResponse, SandboxValidationFailure> {
    let developer_dir = request
        .xcode_path
        .clone()
        .unwrap_or_else(|| config.xcode_path.clone());
    if probe.requires_developer_dir() && !developer_dir.exists() {
        return Err(SandboxValidationFailure {
            error: SandboxPolicyError::XcodePathUnavailable {
                path: developer_dir,
            },
            diagnostics: None,
        });
    }
    let required_sdks = if request.required_sdks.is_empty() {
        config.required_sdks.clone()
    } else {
        request.required_sdks.clone()
    };
    let sdk_inventory =
        probe
            .list_sdks(&developer_dir)
            .map_err(|error| SandboxValidationFailure {
                error,
                diagnostics: None,
            })?;
    let missing_required_sdks: Vec<String> = required_sdks
        .iter()
        .filter(|sdk| !sdk_is_present(&sdk_inventory.normalized, sdk))
        .cloned()
        .collect();
    let status = if missing_required_sdks.is_empty() {
        SandboxStatus::Ok
    } else {
        SandboxStatus::Error
    };
    Ok(InspectXcodeSdksResponse {
        status,
        probe_mode: probe_mode.to_string(),
        developer_dir: developer_dir.display().to_string(),
        xcode_select_path: None,
        required_sdks,
        detected_sdks_raw: sdk_inventory.raw,
        detected_sdks_normalized: sdk_inventory.normalized,
        missing_required_sdks,
        notes: sdk_inventory.notes,
    })
}

/// Version that allows injecting a test double.
pub async fn validate_sandbox_policy_with_probe<P: SandboxProbe>(
    request: SandboxPolicyRequest,
    config: &VisionOsConfig,
    probe: &P,
) -> Result<SandboxPolicyResponse, SandboxValidationFailure> {
    validate_sandbox_policy_with_probe_mode(request, config, probe, "system").await
}

async fn validate_sandbox_policy_with_probe_mode<P: SandboxProbe>(
    request: SandboxPolicyRequest,
    config: &VisionOsConfig,
    probe: &P,
    probe_mode: &str,
) -> Result<SandboxPolicyResponse, SandboxValidationFailure> {
    let project_path = normalize_project_path(&request.project_path).map_err(|error| {
        SandboxValidationFailure {
            error,
            diagnostics: None,
        }
    })?;
    if !config.allowed_paths.is_empty()
        && !visionos_helpers::is_allowed_path(&project_path, &config.allowed_paths)
    {
        return Err(SandboxValidationFailure {
            error: SandboxPolicyError::PathNotAllowed { path: project_path },
            diagnostics: None,
        });
    }

    let mut checks = Vec::new();
    checks.push(SandboxCheck {
        name: "allowed_path".into(),
        result: SandboxCheckResult::Pass,
        details: if config.allowed_paths.is_empty() {
            "allowlist check skipped (visionos.allowed_paths is empty)".into()
        } else {
            format!("{} is within the allowlist", project_path.display())
        },
    });

    let developer_dir = request
        .xcode_path
        .clone()
        .unwrap_or_else(|| config.xcode_path.clone());

    if probe.requires_developer_dir() && !developer_dir.exists() {
        return Err(SandboxValidationFailure {
            error: SandboxPolicyError::XcodePathUnavailable {
                path: developer_dir,
            },
            diagnostics: None,
        });
    }

    let sdk_inventory =
        probe
            .list_sdks(&developer_dir)
            .map_err(|error| SandboxValidationFailure {
                error,
                diagnostics: None,
            })?;
    let required_sdks = if request.required_sdks.is_empty() {
        config.required_sdks.clone()
    } else {
        request.required_sdks.clone()
    };
    let diagnostics = build_diagnostics(
        probe_mode,
        developer_dir.clone(),
        required_sdks.clone(),
        &sdk_inventory,
    );
    for sdk in &required_sdks {
        if !sdk_is_present(&sdk_inventory.normalized, sdk) {
            return Err(SandboxValidationFailure {
                error: SandboxPolicyError::MissingSdk { name: sdk.clone() },
                diagnostics: Some(diagnostics),
            });
        }
    }
    checks.push(SandboxCheck {
        name: "sdk".into(),
        result: SandboxCheckResult::Pass,
        details: format!("SDK: {}", sdk_inventory.normalized.join(", ")),
    });

    if !probe
        .devtools_security_enabled()
        .map_err(|error| SandboxValidationFailure {
            error,
            diagnostics: Some(diagnostics.clone()),
        })?
    {
        return Err(SandboxValidationFailure {
            error: SandboxPolicyError::DevToolsSecurityDisabled,
            diagnostics: Some(diagnostics),
        });
    }
    checks.push(SandboxCheck {
        name: "devtools_security".into(),
        result: SandboxCheckResult::Pass,
        details: "DevToolsSecurity is enabled".into(),
    });

    if !probe
        .xcode_license_accepted()
        .map_err(|error| SandboxValidationFailure {
            error,
            diagnostics: Some(diagnostics.clone()),
        })?
    {
        return Err(SandboxValidationFailure {
            error: SandboxPolicyError::LicenseNotAccepted,
            diagnostics: Some(diagnostics),
        });
    }
    checks.push(SandboxCheck {
        name: "xcode_license".into(),
        result: SandboxCheckResult::Pass,
        details: "Xcode license accepted".into(),
    });

    let disk_root = project_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_path.clone());
    let free_bytes =
        probe
            .disk_free_bytes(&disk_root)
            .map_err(|error| SandboxValidationFailure {
                error,
                diagnostics: Some(diagnostics.clone()),
            })?;
    if free_bytes < MIN_DISK_BYTES {
        return Err(SandboxValidationFailure {
            error: SandboxPolicyError::DiskInsufficient {
                available_bytes: free_bytes,
            },
            diagnostics: Some(diagnostics),
        });
    }
    checks.push(SandboxCheck {
        name: "disk_space".into(),
        result: SandboxCheckResult::Pass,
        details: format!("{} bytes free", free_bytes),
    });

    Ok(SandboxPolicyResponse {
        status: SandboxStatus::Ok,
        checks,
        diagnostics: Some(diagnostics),
    })
}

/// Map check results to error codes.
pub fn sandbox_error_descriptor(error: &SandboxPolicyError) -> &'static ToolErrorDescriptor {
    match error {
        SandboxPolicyError::PathNotAllowed { .. } => &PATH_NOT_ALLOWED_ERROR,
        SandboxPolicyError::MissingSdk { .. } => &SDK_MISSING_ERROR,
        SandboxPolicyError::XcodePathUnavailable { .. } => &XCODE_UNLICENSED_ERROR,
        SandboxPolicyError::LicenseNotAccepted => &XCODE_UNLICENSED_ERROR,
        SandboxPolicyError::DevToolsSecurityDisabled => &DEVTOOLS_DISABLED_ERROR,
        SandboxPolicyError::DiskInsufficient { .. } => &DISK_INSUFFICIENT_ERROR,
        SandboxPolicyError::Internal { .. } => &SANDBOX_INTERNAL_ERROR,
    }
}

/// Convert sandbox errors into MCP error data.
pub fn sandbox_error_to_error_data(failure: SandboxValidationFailure) -> ErrorData {
    let descriptor = sandbox_error_descriptor(&failure.error);
    let mut details = json!({ "details": failure.error.to_string() });
    if let Some(diagnostics) = failure.diagnostics {
        details["diagnostics"] =
            serde_json::to_value(diagnostics).expect("diagnostics should serialize");
    }
    descriptor
        .builder()
        .sandbox_state(SandboxState::Blocked)
        .retryable(false)
        .details(details)
        .build()
        .expect("descriptor is valid")
}

fn normalize_project_path(path: &Path) -> Result<PathBuf, SandboxPolicyError> {
    if !crate::lib::paths::is_nonempty_absolute(path) {
        return Err(SandboxPolicyError::PathNotAllowed {
            path: path.to_path_buf(),
        });
    }
    Ok(path.to_path_buf())
}

fn build_diagnostics(
    probe_mode: &str,
    developer_dir: PathBuf,
    required_sdks: Vec<String>,
    sdk_inventory: &SdkInventory,
) -> SandboxDiagnostics {
    SandboxDiagnostics {
        probe_mode: probe_mode.to_string(),
        effective_developer_dir: developer_dir.display().to_string(),
        effective_required_sdks: required_sdks,
        detected_sdks_raw: sdk_inventory.raw.clone(),
        detected_sdks_normalized: sdk_inventory.normalized.clone(),
        xcodebuild_invocation: sdk_inventory.invocation.clone(),
        notes: sdk_inventory.notes.clone(),
    }
}

fn sdk_is_present(sdks: &[String], required: &str) -> bool {
    let required = required.trim();
    if required.is_empty() {
        return false;
    }
    let required_lower = required.to_lowercase();
    sdks.iter().any(|sdk| {
        let sdk_trimmed = sdk.trim();
        if sdk_trimmed.is_empty() {
            return false;
        }
        let sdk_lower = sdk_trimmed.to_lowercase();
        sdk_lower == required_lower || sdk_lower.starts_with(&required_lower)
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rmcp::model::ErrorData;
    use serde_json::{Map, Value};
    use tempfile::tempdir;

    use crate::server::config::VisionOsConfig;

    use super::*;

    struct FakeProbe {
        sdks: Vec<String>,
        devtools_enabled: bool,
        license_ok: bool,
        disk_bytes: u64,
    }

    impl SandboxProbe for FakeProbe {
        fn list_sdks(
            &self,
            _developer_dir: &std::path::Path,
        ) -> Result<SdkInventory, crate::lib::errors::SandboxPolicyError> {
            Ok(SdkInventory {
                raw: self.sdks.clone(),
                normalized: self.sdks.clone(),
                invocation: Some("fake-xcodebuild -showsdks".into()),
                notes: Vec::new(),
            })
        }

        fn devtools_security_enabled(
            &self,
        ) -> Result<bool, crate::lib::errors::SandboxPolicyError> {
            Ok(self.devtools_enabled)
        }

        fn xcode_license_accepted(&self) -> Result<bool, crate::lib::errors::SandboxPolicyError> {
            Ok(self.license_ok)
        }

        fn disk_free_bytes(
            &self,
            _path: &std::path::Path,
        ) -> Result<u64, crate::lib::errors::SandboxPolicyError> {
            Ok(self.disk_bytes)
        }
    }

    fn sample_config() -> VisionOsConfig {
        VisionOsConfig {
            allowed_paths: vec![allowed_project_path().parent().unwrap().to_path_buf()],
            allowed_schemes: vec!["VisionApp".into()],
            default_destination: "platform=visionOS Simulator,name=Apple Vision Pro".into(),
            required_sdks: vec!["visionOS".into(), "visionOS Simulator".into()],
            xcode_path: PathBuf::from("/Applications/Xcode.app/Contents/Developer"),
            xcodebuild_path: PathBuf::from("/usr/bin/xcodebuild"),
            max_build_minutes: 20,
            artifact_ttl_secs: 600,
            cleanup_schedule_secs: 60,
        }
    }

    fn allowed_project_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/visionos/workspace/VisionApp")
    }

    #[tokio::test]
    async fn sandbox_policy_reports_missing_sdk() {
        let temp = tempdir().expect("can create temp directory");
        let request = SandboxPolicyRequest {
            project_path: allowed_project_path(),
            required_sdks: vec!["visionOS".into()],
            xcode_path: Some(temp.path().to_path_buf()),
        };
        let probe = FakeProbe {
            sdks: vec![],
            devtools_enabled: true,
            license_ok: true,
            disk_bytes: 500 * 1024 * 1024,
        };

        let failure = validate_sandbox_policy_with_probe(request, &sample_config(), &probe)
            .await
            .expect_err("should error when SDK is missing");

        match failure.error {
            crate::lib::errors::SandboxPolicyError::MissingSdk { name } => {
                assert_eq!(name, "visionOS")
            }
            other => panic!("Unexpected error: {other:?}", other = other),
        }
        let diagnostics = failure
            .diagnostics
            .expect("missing sdk should include diagnostics");
        assert_eq!(diagnostics.probe_mode, "system");
        assert_eq!(diagnostics.effective_required_sdks, vec!["visionOS"]);
        assert!(diagnostics.detected_sdks_raw.is_empty());
    }

    #[tokio::test]
    async fn sandbox_policy_accepts_prefix_sdk_match() {
        let temp = tempdir().expect("can create temp directory");
        let request = SandboxPolicyRequest {
            project_path: allowed_project_path(),
            required_sdks: vec!["xros".into(), "macosx".into()],
            xcode_path: Some(temp.path().to_path_buf()),
        };
        let probe = FakeProbe {
            sdks: vec!["xros26.0".into(), "macosx15.0".into()],
            devtools_enabled: true,
            license_ok: true,
            disk_bytes: MIN_DISK_BYTES + 1,
        };

        let response = validate_sandbox_policy_with_probe(request, &sample_config(), &probe)
            .await
            .expect("prefix match should be accepted");

        assert_eq!(response.status, SandboxStatus::Ok);
        let diagnostics = response
            .diagnostics
            .expect("successful response should include diagnostics");
        assert!(diagnostics
            .detected_sdks_raw
            .contains(&"xros26.0".to_string()));
        assert!(diagnostics
            .detected_sdks_normalized
            .contains(&"xros26.0".to_string()));
    }

    #[tokio::test]
    async fn sandbox_policy_rejects_path_outside_allowlist() {
        let temp = tempdir().expect("can create temp directory");
        let request = SandboxPolicyRequest {
            project_path: PathBuf::from("/tmp/disallowed-project"),
            required_sdks: vec!["visionOS".into()],
            xcode_path: Some(temp.path().to_path_buf()),
        };
        let probe = FakeProbe {
            sdks: vec!["visionOS".into()],
            devtools_enabled: true,
            license_ok: true,
            disk_bytes: 500 * 1024 * 1024,
        };

        let failure = validate_sandbox_policy_with_probe(request, &sample_config(), &probe)
            .await
            .expect_err("should error for disallowed path");

        match failure.error {
            crate::lib::errors::SandboxPolicyError::PathNotAllowed { path } => {
                assert_eq!(path, PathBuf::from("/tmp/disallowed-project"))
            }
            other => panic!("Unexpected error: {other:?}", other = other),
        }
    }

    #[tokio::test]
    async fn sandbox_policy_skips_allowlist_when_allowed_paths_empty() {
        let temp = tempdir().expect("can create temp directory");
        let request = SandboxPolicyRequest {
            project_path: PathBuf::from("/tmp/disallowed-project"),
            required_sdks: vec!["visionOS".into()],
            xcode_path: Some(temp.path().to_path_buf()),
        };
        let probe = FakeProbe {
            sdks: vec!["visionOS".into()],
            devtools_enabled: true,
            license_ok: true,
            disk_bytes: MIN_DISK_BYTES + 1,
        };

        let mut config = sample_config();
        config.allowed_paths = vec![];

        let response = validate_sandbox_policy_with_probe(request, &config, &probe)
            .await
            .expect("allowlist check should be skipped when allowed_paths is empty");

        assert_eq!(response.status, SandboxStatus::Ok);
        let allowed_path_check = response
            .checks
            .iter()
            .find(|check| check.name == "allowed_path")
            .expect("should include allowed_path check");
        assert_eq!(allowed_path_check.result, SandboxCheckResult::Pass);
        assert_eq!(
            allowed_path_check.details,
            "allowlist check skipped (visionos.allowed_paths is empty)"
        );
    }

    #[test]
    fn sandbox_error_data_includes_structured_fields_for_missing_sdk() {
        let error = SandboxPolicyError::MissingSdk {
            name: "visionOS".into(),
        };
        let data = extract_data(&sandbox_error_to_error_data(SandboxValidationFailure {
            error,
            diagnostics: Some(SandboxDiagnostics {
                probe_mode: "env".into(),
                effective_developer_dir: "/Applications/Xcode.app/Contents/Developer".into(),
                effective_required_sdks: vec!["visionOS".into()],
                detected_sdks_raw: vec![],
                detected_sdks_normalized: vec![],
                xcodebuild_invocation: None,
                notes: vec!["VISIONOS_SANDBOX_SDKS is empty".into()],
            }),
        }));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("sdk_missing")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
        assert!(data.get("remediation").and_then(Value::as_str).is_some());
        assert!(data
            .get("details")
            .and_then(Value::as_object)
            .and_then(|details| details.get("diagnostics"))
            .is_some());
    }

    #[test]
    fn sandbox_error_data_includes_structured_fields_for_path_not_allowed() {
        let error = SandboxPolicyError::PathNotAllowed {
            path: PathBuf::from("/tmp/disallowed-project"),
        };
        let data = extract_data(&sandbox_error_to_error_data(SandboxValidationFailure {
            error,
            diagnostics: None,
        }));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("path_not_allowed")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn sandbox_error_data_includes_structured_fields_for_devtools_disabled() {
        let data = extract_data(&sandbox_error_to_error_data(SandboxValidationFailure {
            error: SandboxPolicyError::DevToolsSecurityDisabled,
            diagnostics: None,
        }));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("devtools_security_disabled")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn sandbox_error_data_includes_structured_fields_for_disk_insufficient() {
        let data = extract_data(&sandbox_error_to_error_data(SandboxValidationFailure {
            error: SandboxPolicyError::DiskInsufficient { available_bytes: 1 },
            diagnostics: None,
        }));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("disk_insufficient")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn sandbox_error_data_includes_structured_fields_for_xcode_unlicensed() {
        let data = extract_data(&sandbox_error_to_error_data(SandboxValidationFailure {
            error: SandboxPolicyError::LicenseNotAccepted,
            diagnostics: None,
        }));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("xcode_unlicensed")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn sandbox_error_data_includes_structured_fields_for_internal_error() {
        let data = extract_data(&sandbox_error_to_error_data(SandboxValidationFailure {
            error: SandboxPolicyError::Internal {
                message: "oops".into(),
            },
            diagnostics: None,
        }));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("sandbox_internal_error")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
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
