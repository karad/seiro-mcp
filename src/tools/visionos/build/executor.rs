use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use rmcp::model::ErrorData;
use serde_json::{json, Value};
use tokio::time;
use tracing::info;
use uuid::Uuid;

use crate::{
    lib::{
        errors::{SandboxState, ToolErrorDescriptor, VisionOsBuildError},
        fs as artifact_fs, visionos as visionos_helpers, xcodebuild as xcodebuild_helpers,
    },
    server::config::VisionOsConfig,
};

use super::{BuildRequestValidationError, VisionOsBuildRequest};

const LOG_EXCERPT_LIMIT: usize = 5_000;

const PATH_NOT_ALLOWED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "path_not_allowed",
    "project_path is outside the allowlist",
    "Update visionos.allowed_paths in config.toml and restart the MCP server.",
);
const INVALID_INPUT_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "invalid_request",
    "The visionOS build request format is invalid",
    "Check the constraints for destination, extra_args, and workspace.",
);
const SCHEME_NOT_ALLOWED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "scheme_not_allowed",
    "scheme is not in the allowlist",
    "Update visionos.allowed_schemes in config.toml or use an allowed scheme.",
);
const TIMEOUT_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "timeout",
    "Build was aborted after exceeding max_build_minutes",
    "Shorten the build time or increase max_build_minutes.",
);
const BUILD_FAILED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "build_failed",
    "xcodebuild exited with an error",
    "Review the log excerpt and fix the failing targets.",
);
const DESTINATION_AMBIGUOUS_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "destination_ambiguous",
    "The requested simulator destination matched multiple devices",
    "Retry build_visionos_app with an id-based destination such as `platform=visionOS Simulator,id:<device-id>`.",
);
const SANDBOX_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "sandbox_violation_blocked",
    "Build was blocked by the sandbox policy",
    "Verify allowed paths, SDK setup, and DevToolsSecurity.",
);

/// Response from `build_visionos_app`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct BuildVisionOsAppResponse {
    pub job_id: String,
    pub status: &'static str,
    pub artifact_path: String,
    pub artifact_sha256: String,
    pub log_excerpt: String,
    pub duration_ms: u128,
}

use schemars::JsonSchema;
use serde::Serialize;

/// Execute a visionOS build.
pub async fn run_build(
    request: &VisionOsBuildRequest,
    config: &VisionOsConfig,
    job_id: Uuid,
    artifact_root: PathBuf,
) -> Result<BuildVisionOsAppResponse, VisionOsBuildError> {
    let job_dir = artifact_fs::ensure_job_dir(&artifact_root, &job_id)?;
    let staging_dir = job_dir.join("staging");
    fs::create_dir_all(&staging_dir).map_err(|err| VisionOsBuildError::ArtifactFailure {
        message: format!("Failed to create artifact staging directory: {err}"),
    })?;

    let time_scale = env::var("VISIONOS_TEST_TIME_SCALE")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|scale| *scale > 0)
        .unwrap_or(60);
    let timeout_duration = Duration::from_secs(config.max_build_minutes as u64 * time_scale);
    let start = Instant::now();
    let output = time::timeout(
        timeout_duration,
        spawn_xcodebuild(request, config, &staging_dir),
    )
    .await
    .map_err(|_| VisionOsBuildError::Timeout {
        duration_secs: timeout_duration.as_secs(),
    })?
    .map_err(|err| VisionOsBuildError::CommandFailed {
        exit_code: None,
        message: err.to_string(),
    })?;

    let log_excerpt = collect_log_excerpt(&output.stdout, &output.stderr);
    if !output.status.success() {
        return Err(VisionOsBuildError::CommandFailed {
            exit_code: output.status.code(),
            message: log_excerpt,
        });
    }

    let artifact_zip = job_dir.join("artifact.zip");
    artifact_fs::zip_directory(&staging_dir, &artifact_zip)?;
    let artifact_sha256 = artifact_fs::compute_sha256(&artifact_zip)?;

    Ok(BuildVisionOsAppResponse {
        job_id: job_id.to_string(),
        status: "succeeded",
        artifact_path: artifact_zip.to_string_lossy().to_string(),
        artifact_sha256,
        log_excerpt,
        duration_ms: start.elapsed().as_millis(),
    })
}

async fn spawn_xcodebuild(
    request: &VisionOsBuildRequest,
    config: &VisionOsConfig,
    staging_dir: &Path,
) -> std::io::Result<std::process::Output> {
    let mut command = xcodebuild_helpers::build_visionos_xcodebuild_command(
        xcodebuild_helpers::VisionOsXcodebuildCommandConfig {
            xcodebuild_path: &config.xcodebuild_path,
            xcode_path: &config.xcode_path,
            staging_dir,
        },
        xcodebuild_helpers::VisionOsXcodebuildRequest {
            project_path: &request.project_path,
            workspace: request.workspace.as_deref(),
            scheme: &request.scheme,
            configuration: request.configuration.as_str(),
            destination: &request.destination,
            clean: request.clean,
            extra_args: &request.extra_args,
            env_overrides: &request.env_overrides,
        },
    );

    info!(
        target: "rmcp_sample::visionos",
        scheme = %request.scheme,
        destination = %request.destination,
        clean = request.clean,
        "Starting visionOS build"
    );

    command.output().await
}

fn collect_log_excerpt(stdout: &[u8], stderr: &[u8]) -> String {
    visionos_helpers::collect_log_excerpt(stdout, stderr, LOG_EXCERPT_LIMIT)
}

pub fn validation_error_to_error_data(err: BuildRequestValidationError) -> ErrorData {
    match err {
        BuildRequestValidationError::ProjectPathNotAllowed { path }
        | BuildRequestValidationError::WorkspaceNotAllowed { path } => build_error_data(
            &PATH_NOT_ALLOWED_ERROR,
            json!({ "path": path.to_string_lossy() }),
            SandboxState::Blocked,
            false,
        ),
        BuildRequestValidationError::SchemeNotAllowed { scheme } => build_error_data(
            &SCHEME_NOT_ALLOWED_ERROR,
            json!({ "scheme": scheme }),
            SandboxState::Blocked,
            false,
        ),
        _ => build_error_data(
            &INVALID_INPUT_ERROR,
            json!({ "details": err.to_string() }),
            SandboxState::NoViolation,
            false,
        ),
    }
}

pub fn runtime_error_to_error_data(err: VisionOsBuildError, job_id: Uuid) -> ErrorData {
    match err {
        VisionOsBuildError::PathNotAllowed { path } => build_error_data_with_job(
            &PATH_NOT_ALLOWED_ERROR,
            json!({ "path": path.to_string_lossy() }),
            SandboxState::Blocked,
            false,
            job_id,
        ),
        VisionOsBuildError::Timeout { duration_secs } => build_error_data_with_job(
            &TIMEOUT_ERROR,
            json!({ "duration_secs": duration_secs }),
            SandboxState::NoViolation,
            true,
            job_id,
        ),
        VisionOsBuildError::SandboxViolated { reason } => build_error_data_with_job(
            &SANDBOX_ERROR,
            json!({ "reason": reason }),
            SandboxState::Blocked,
            false,
            job_id,
        ),
        VisionOsBuildError::CommandFailed { exit_code, message } => {
            if let Some(details) = parse_ambiguous_destination_details(&message) {
                return build_error_data_with_job(
                    &DESTINATION_AMBIGUOUS_ERROR,
                    json!({
                        "details": message,
                        "exit_code": exit_code,
                        "matched_devices": details.matched_devices,
                        "available_destinations": details.available_destinations,
                        "suggested_destination": details.suggested_destination
                    }),
                    SandboxState::NoViolation,
                    true,
                    job_id,
                );
            }

            build_error_data_with_job(
                &BUILD_FAILED_ERROR,
                json!({
                    "details": message,
                    "diagnostics_hint": "inspect_build_diagnostics"
                }),
                SandboxState::NoViolation,
                true,
                job_id,
            )
        }
        _ => build_error_data_with_job(
            &BUILD_FAILED_ERROR,
            json!({
                "details": err.to_string(),
                "diagnostics_hint": "inspect_build_diagnostics"
            }),
            SandboxState::NoViolation,
            true,
            job_id,
        ),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct AmbiguousDestinationDetails {
    matched_devices: Vec<MatchedDevice>,
    available_destinations: Vec<AvailableDestination>,
    suggested_destination: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct MatchedDevice {
    name: String,
    id: String,
    os: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct AvailableDestination {
    platform: String,
    id: String,
    name: String,
    os: Option<String>,
    arch: Option<String>,
}

fn parse_ambiguous_destination_details(message: &str) -> Option<AmbiguousDestinationDetails> {
    if !message.contains("multiple devices matched the request") {
        return None;
    }

    let matched_devices = parse_matched_devices(message);
    let available_destinations = parse_available_destinations(message);
    let suggested_destination = available_destinations
        .iter()
        .find(|destination| {
            destination.platform == "visionOS Simulator"
                && !destination.id.contains("placeholder")
                && destination.id.len() >= 8
        })
        .map(|destination| format!("platform=visionOS Simulator,id:{}", destination.id));

    Some(AmbiguousDestinationDetails {
        matched_devices,
        available_destinations,
        suggested_destination,
    })
}

fn parse_matched_devices(message: &str) -> Vec<MatchedDevice> {
    message
        .lines()
        .filter_map(|line| {
            let marker = "SimDevice: ";
            let start = line.find(marker)?;
            let tail = &line[start + marker.len()..];
            let open_paren = tail.find(" (")?;
            let name = tail[..open_paren].trim().to_string();
            let inner = tail[open_paren + 2..].split(')').next()?.trim();
            let mut parts = inner.split(',').map(str::trim);
            let id = parts.next()?.to_string();
            let os = parts.next()?.to_string();
            Some(MatchedDevice { name, id, os })
        })
        .collect()
}

fn parse_available_destinations(message: &str) -> Vec<AvailableDestination> {
    let mut destinations = Vec::new();
    let mut in_section = false;

    for line in message.lines() {
        if line.contains("Available destinations for the") {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }

        let trimmed = line.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            if !trimmed.is_empty() {
                break;
            }
            continue;
        }

        let mut platform = None;
        let mut id = None;
        let mut name = None;
        let mut os = None;
        let mut arch = None;

        for field in trimmed
            .trim_start_matches('{')
            .trim_end_matches('}')
            .split(',')
            .map(str::trim)
        {
            if let Some((key, value)) = field.split_once(':') {
                let value = value.trim().to_string();
                match key.trim() {
                    "platform" => platform = Some(value),
                    "id" => id = Some(value),
                    "name" => name = Some(value),
                    "OS" => os = Some(value),
                    "arch" => arch = Some(value),
                    _ => {}
                }
            }
        }

        if let (Some(platform), Some(id), Some(name)) = (platform, id, name) {
            destinations.push(AvailableDestination {
                platform,
                id,
                name,
                os,
                arch,
            });
        }
    }

    destinations
}

fn build_error_data(
    desc: &'static ToolErrorDescriptor,
    details: Value,
    sandbox_state: SandboxState,
    retryable: bool,
) -> ErrorData {
    desc.builder()
        .details(details)
        .sandbox_state(sandbox_state)
        .retryable(retryable)
        .build()
        .expect("descriptor is valid")
}

fn build_error_data_with_job(
    desc: &'static ToolErrorDescriptor,
    details: Value,
    sandbox_state: SandboxState,
    retryable: bool,
    job_id: Uuid,
) -> ErrorData {
    desc.builder()
        .details(details)
        .sandbox_state(sandbox_state)
        .retryable(retryable)
        .with_context_field("job_id", json!(job_id.to_string()))
        .build()
        .expect("descriptor is valid")
}

#[cfg(test)]
mod tests {
    use rmcp::model::ErrorData;
    use serde_json::{Map, Value};

    use super::*;

    #[test]
    fn validation_error_maps_to_structured_error_fields() {
        let err = BuildRequestValidationError::SchemeNotAllowed {
            scheme: "Nope".into(),
        };
        let data = extract_data(&validation_error_to_error_data(err));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("scheme_not_allowed")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn runtime_timeout_maps_to_retryable_error_with_job_id() {
        let job_id = Uuid::new_v4();
        let expected_job_id = job_id.to_string();
        let err = VisionOsBuildError::Timeout { duration_secs: 123 };
        let data = extract_data(&runtime_error_to_error_data(err, job_id));
        assert_eq!(data.get("code").and_then(Value::as_str), Some("timeout"));
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("no_violation")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(true));
        assert_eq!(
            data.get("job_id").and_then(Value::as_str),
            Some(expected_job_id.as_str())
        );
    }

    #[test]
    fn runtime_sandbox_violation_maps_to_blocked_non_retryable_error() {
        let job_id = Uuid::new_v4();
        let err = VisionOsBuildError::SandboxViolated {
            reason: "nope".into(),
        };
        let data = extract_data(&runtime_error_to_error_data(err, job_id));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("sandbox_violation_blocked")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("blocked")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn runtime_build_failed_maps_to_retryable_error() {
        let job_id = Uuid::new_v4();
        let err = VisionOsBuildError::CommandFailed {
            exit_code: Some(1),
            message: "fail".into(),
        };
        let data = extract_data(&runtime_error_to_error_data(err, job_id));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("build_failed")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("no_violation")
        );
        assert_eq!(data.get("retryable").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn runtime_ambiguous_destination_maps_to_structured_error() {
        let job_id = Uuid::new_v4();
        let err = VisionOsBuildError::CommandFailed {
            exit_code: Some(70),
            message: r#"xcodebuild: error: Unable to find a device matching the provided destination specifier:
        { platform:visionOS Simulator, OS:latest, name:Apple Vision Pro }

    The requested device could not be found because multiple devices matched the request. (
 "<DVTiPhoneSimulator: 0xb57503480> {\n\t\tSimDevice: Apple Vision Pro (5BB47C97-BDBA-4DA7-BE30-F659C265F896, visionOS 2.5, Shutdown)\n}",
 "<DVTiPhoneSimulator: 0xb57503980> {\n\t\tSimDevice: Apple Vision Pro (F556D53F-412A-4778-AF81-3449D52F5A7F, visionOS 26.2, Shutdown)\n}"
)

    Available destinations for the "HelloSkills" scheme:
        { platform:visionOS, id:dvtdevice-DVTiOSDevicePlaceholder-xros:placeholder, name:Any visionOS Device }
        { platform:visionOS Simulator, id:dvtdevice-DVTiOSDeviceSimulatorPlaceholder-xrsimulator:placeholder, name:Any visionOS Simulator Device }
        { platform:visionOS Simulator, arch:arm64, id:F556D53F-412A-4778-AF81-3449D52F5A7F, OS:26.2, name:Apple Vision Pro }"#.into(),
        };

        let data = extract_data(&runtime_error_to_error_data(err, job_id));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("destination_ambiguous")
        );
        assert_eq!(
            data.get("details")
                .and_then(Value::as_object)
                .and_then(|details| details.get("suggested_destination"))
                .and_then(Value::as_str),
            Some("platform=visionOS Simulator,id:F556D53F-412A-4778-AF81-3449D52F5A7F")
        );
        assert_eq!(
            data.get("details")
                .and_then(Value::as_object)
                .and_then(|details| details.get("matched_devices"))
                .and_then(Value::as_array)
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn validation_invalid_request_maps_to_no_violation_non_retryable_error() {
        let err = BuildRequestValidationError::DestinationMissingPlatform;
        let data = extract_data(&validation_error_to_error_data(err));
        assert_eq!(
            data.get("code").and_then(Value::as_str),
            Some("invalid_request")
        );
        assert_eq!(
            data.get("sandbox_state").and_then(Value::as_str),
            Some("no_violation")
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
