//! Management and retrieval tools for visionOS build artifacts.
pub mod store;

use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use uuid::Uuid;

use crate::lib::errors::{ArtifactError, SandboxState, ToolErrorDescriptor};

pub use store::{BuildJobRecord, BuildJobStatus, VisionOsArtifactStore, ARTIFACT_ROOT};

/// Input for `fetch_build_output`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct FetchBuildOutputRequest {
    pub job_id: String,
    #[serde(default = "default_include_logs")]
    pub include_logs: bool,
}

fn default_include_logs() -> bool {
    true
}

/// Response from `fetch_build_output`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct FetchBuildOutputResponse {
    pub job_id: String,
    pub status: &'static str,
    pub artifact_zip: Option<String>,
    pub sha256: Option<String>,
    pub download_ttl_seconds: u32,
    pub log_excerpt: Option<String>,
}

/// Error types for `fetch_build_output`.
#[derive(Debug, Error)]
pub enum FetchBuildOutputError {
    #[error("Invalid job ID format: {raw}")]
    InvalidJobId { raw: String },
    #[error("Job {job_id} not found")]
    JobNotFound { job_id: Uuid },
    #[error("Artifacts for job {job_id} have expired")]
    ArtifactExpired { job_id: Uuid },
    #[error("Job {job_id} did not produce artifacts because the build failed")]
    BuildFailedNoArtifact { job_id: Uuid },
    #[error(transparent)]
    Store(#[from] ArtifactError),
}

/// Core logic for the fetch tool.
pub async fn fetch_build_output(
    store: &VisionOsArtifactStore,
    request: FetchBuildOutputRequest,
) -> Result<FetchBuildOutputResponse, FetchBuildOutputError> {
    let job_id = Uuid::parse_str(request.job_id.trim()).map_err(|_| {
        FetchBuildOutputError::InvalidJobId {
            raw: request.job_id.clone(),
        }
    })?;
    let record = store.fetch_record(&job_id).await?;
    match record.status {
        BuildJobStatus::Succeeded => {
            let ttl = store.ttl_seconds_remaining(&record);
            Ok(FetchBuildOutputResponse {
                job_id: job_id.to_string(),
                status: "succeeded",
                artifact_zip: record
                    .artifact_zip
                    .as_ref()
                    .map(|path| path.to_string_lossy().to_string()),
                sha256: record.artifact_sha256.clone(),
                download_ttl_seconds: ttl,
                log_excerpt: request.include_logs.then(|| record.log_excerpt.clone()),
            })
        }
        BuildJobStatus::Failed => Err(FetchBuildOutputError::BuildFailedNoArtifact { job_id }),
    }
}

/// Convert fetch tool errors into MCP error data.
pub fn fetch_error_to_error_data(err: FetchBuildOutputError) -> ErrorData {
    match err {
        FetchBuildOutputError::InvalidJobId { raw } => fetch_error(
            &INVALID_JOB_ID_ERROR,
            None,
            json!({ "details": raw }),
            false,
        ),
        FetchBuildOutputError::JobNotFound { job_id } => {
            fetch_error(&JOB_NOT_FOUND_ERROR, Some(job_id), json!({}), false)
        }
        FetchBuildOutputError::ArtifactExpired { job_id } => {
            fetch_error(&ARTIFACT_EXPIRED_ERROR, Some(job_id), json!({}), true)
        }
        FetchBuildOutputError::BuildFailedNoArtifact { job_id } => {
            fetch_error(&BUILD_FAILED_ERROR, Some(job_id), json!({}), false)
        }
        FetchBuildOutputError::Store(err) => fetch_error(
            &ARTIFACT_EXPIRED_ERROR,
            None,
            json!({ "details": err.to_string() }),
            true,
        ),
    }
}

const INVALID_JOB_ID_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "invalid_job_id",
    "Invalid job_id format",
    "Provide a UUID-formatted job_id and run the command again.",
);

const JOB_NOT_FOUND_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "job_not_found",
    "The specified build job was not found",
    "Check the job_id and try again. Run a new build if needed.",
);

const ARTIFACT_EXPIRED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "artifact_expired",
    "Artifact TTL has expired",
    "Re-run build_visionos_app to generate fresh artifacts before downloading again.",
);

const BUILD_FAILED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "build_failed_no_artifact",
    "No artifacts are available because the build failed",
    "Review the logs, fix the issue, and build again.",
);

fn fetch_error(
    descriptor: &'static ToolErrorDescriptor,
    job_id: Option<Uuid>,
    details: serde_json::Value,
    retryable: bool,
) -> ErrorData {
    let mut builder = descriptor
        .builder()
        .sandbox_state(SandboxState::NoViolation)
        .details(details)
        .retryable(retryable);

    if let Some(job) = job_id {
        builder = builder.with_context_field("job_id", json!(job.to_string()));
    }

    builder.build().expect("descriptor is valid")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use chrono::{Duration, Utc};
    use tempfile::tempdir;
    use uuid::Uuid;

    use super::*;

    #[tokio::test]
    async fn fetch_returns_artifact_metadata() {
        let temp = tempdir().expect("temporary directory");
        let store = VisionOsArtifactStore::with_root(temp.path().to_path_buf(), 600, 60);
        let job_id = Uuid::new_v4();
        let artifact_path = temp.path().join("artifact.zip");
        fs::write(&artifact_path, b"artifact").expect("write artifact");

        store
            .record_success(
                job_id,
                artifact_path.clone(),
                "deadbeef".into(),
                "log excerpt".into(),
                Utc::now(),
            )
            .await
            .expect("record success");

        let response = fetch_build_output(
            &store,
            FetchBuildOutputRequest {
                job_id: job_id.to_string(),
                include_logs: true,
            },
        )
        .await
        .expect("fetch succeeds");

        assert_eq!(response.job_id, job_id.to_string());
        assert_eq!(response.status, "succeeded");
        assert_eq!(
            response.artifact_zip,
            Some(artifact_path.to_string_lossy().into())
        );
        assert_eq!(response.sha256.as_deref(), Some("deadbeef"));
        assert!(response.download_ttl_seconds <= 600);
        assert!(response.download_ttl_seconds > 0);
        assert_eq!(response.log_excerpt.as_deref(), Some("log excerpt"));
    }

    #[tokio::test]
    async fn fetch_errors_when_ttl_expired() {
        let temp = tempdir().expect("temporary directory");
        let store = VisionOsArtifactStore::with_root(temp.path().to_path_buf(), 60, 30);
        let job_id = Uuid::new_v4();
        let artifact_path = temp.path().join("artifact.zip");
        fs::write(&artifact_path, b"artifact").expect("write artifact");

        store
            .record_success(
                job_id,
                artifact_path,
                "deadbeef".into(),
                "log excerpt".into(),
                Utc::now() - Duration::seconds(70),
            )
            .await
            .expect("record success");

        let err = fetch_build_output(
            &store,
            FetchBuildOutputRequest {
                job_id: job_id.to_string(),
                include_logs: true,
            },
        )
        .await
        .expect_err("fetch should fail");

        assert!(matches!(err, FetchBuildOutputError::ArtifactExpired { .. }));
    }

    #[tokio::test]
    async fn fetch_errors_when_job_failed() {
        let temp = tempdir().expect("temporary directory");
        let store = VisionOsArtifactStore::with_root(temp.path().to_path_buf(), 60, 30);
        let job_id = Uuid::new_v4();

        store
            .record_failure(job_id, "failed".into(), Utc::now())
            .await
            .expect("record failure");

        let err = fetch_build_output(
            &store,
            FetchBuildOutputRequest {
                job_id: job_id.to_string(),
                include_logs: true,
            },
        )
        .await
        .expect_err("fetch should fail");

        assert!(matches!(
            err,
            FetchBuildOutputError::BuildFailedNoArtifact { .. }
        ));
    }
}
