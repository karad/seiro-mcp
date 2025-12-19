use std::{
    env, fs,
    path::Path,
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
    tools::visionos::artifacts::ARTIFACT_ROOT,
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
) -> Result<BuildVisionOsAppResponse, VisionOsBuildError> {
    let job_dir = artifact_fs::ensure_job_dir(Path::new(ARTIFACT_ROOT), &job_id)?;
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
        _ => build_error_data_with_job(
            &BUILD_FAILED_ERROR,
            json!({ "details": err.to_string() }),
            SandboxState::NoViolation,
            true,
            job_id,
        ),
    }
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
