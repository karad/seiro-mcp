//! Failure diagnostics for visionOS builds.

mod parser;
mod request;
mod swift_typecheck;

use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    lib::errors::{SandboxState, ToolErrorDescriptor},
    tools::visionos::{
        artifacts::{BuildJobStatus, FetchBuildOutputError, VisionOsArtifactStore},
        visionos_fetch_error,
    },
};

use parser::parse_primary_error;
pub use request::InspectBuildDiagnosticsRequest;
use swift_typecheck::run_typecheck;

const INVALID_JOB_ID_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "invalid_job_id",
    "Invalid job_id format",
    "Provide a UUID-formatted job_id and retry.",
);
const JOB_NOT_FOUND_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "job_not_found",
    "The specified build job was not found",
    "Check the job_id and run a new build if needed.",
);
const DIAGNOSTICS_EXPIRED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "diagnostics_expired",
    "Diagnostic context expired for this job",
    "Re-run build_visionos_app and inspect diagnostics again.",
);
const DIAGNOSTICS_UNAVAILABLE_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "diagnostics_unavailable",
    "Typecheck diagnostics are unavailable for this job",
    "Re-run build_visionos_app, then inspect diagnostics again with the new job_id.",
);

/// Failure summary payload.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BuildFailureSummary {
    pub error_code: String,
    pub headline: String,
    pub source: String,
}

/// File/line location for a primary failure.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FailureLocation {
    pub file: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

/// Response from `inspect_build_diagnostics`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InspectBuildDiagnosticsResponse {
    pub job_id: String,
    pub status: &'static str,
    pub availability: &'static str,
    pub failure_summary: BuildFailureSummary,
    pub primary_location: Option<FailureLocation>,
    pub invocation: Option<String>,
    pub diagnostic_excerpt: Option<String>,
    pub log_excerpt: Option<String>,
    pub notes: Vec<String>,
}

/// Execute diagnostics for a failed build.
pub async fn inspect_build_diagnostics(
    store: &VisionOsArtifactStore,
    request: InspectBuildDiagnosticsRequest,
) -> Result<InspectBuildDiagnosticsResponse, ErrorData> {
    let job_id = Uuid::parse_str(request.job_id.trim()).map_err(|_| {
        build_error_data(
            &INVALID_JOB_ID_ERROR,
            json!({ "details": request.job_id }),
            SandboxState::NoViolation,
            false,
        )
    })?;

    let record = store.fetch_record(&job_id).await.map_err(map_fetch_error)?;
    if record.status != BuildJobStatus::Failed {
        return Ok(InspectBuildDiagnosticsResponse {
            job_id: job_id.to_string(),
            status: "ok",
            availability: "unsupported",
            failure_summary: BuildFailureSummary {
                error_code: "diagnostics_unsupported".into(),
                headline: "Diagnostics are only available for failed builds".into(),
                source: "xcodebuild_log".into(),
            },
            primary_location: None,
            invocation: None,
            diagnostic_excerpt: None,
            log_excerpt: request
                .include_log_excerpt
                .then(|| record.log_excerpt.clone()),
            notes: vec!["job status is succeeded".into()],
        });
    }

    let mut notes = Vec::new();
    if request.prefer_typecheck {
        if let Some(context) = &record.failure_context {
            match run_typecheck(context).await {
                Ok(output) => {
                    if let Some(primary) = parse_primary_error(&output.stderr, &output.stdout) {
                        return Ok(InspectBuildDiagnosticsResponse {
                            job_id: job_id.to_string(),
                            status: "ok",
                            availability: "available",
                            failure_summary: BuildFailureSummary {
                                error_code: "typecheck_failed".into(),
                                headline: primary.headline.clone(),
                                source: "typecheck".into(),
                            },
                            primary_location: primary.file.map(|file| FailureLocation {
                                file,
                                line: primary.line,
                                column: primary.column,
                            }),
                            invocation: Some(output.invocation),
                            diagnostic_excerpt: Some(primary.excerpt),
                            log_excerpt: request
                                .include_log_excerpt
                                .then(|| record.log_excerpt.clone()),
                            notes,
                        });
                    }
                    notes
                        .push("typecheck completed but no actionable error line was parsed".into());
                }
                Err(err) => {
                    notes.push(format!("typecheck diagnostics unavailable: {err}"));
                }
            }
        } else {
            return Err(build_error_data(
                &DIAGNOSTICS_UNAVAILABLE_ERROR,
                json!({
                    "job_id": job_id.to_string(),
                    "details": "failed job has no stored build context for typecheck replay"
                }),
                SandboxState::NoViolation,
                true,
            ));
        }
    } else {
        notes.push("typecheck replay is disabled by request.prefer_typecheck=false".into());
    }

    Ok(InspectBuildDiagnosticsResponse {
        job_id: job_id.to_string(),
        status: "ok",
        availability: "unavailable",
        failure_summary: BuildFailureSummary {
            error_code: "build_failed".into(),
            headline: build_log_headline(&record.log_excerpt),
            source: "xcodebuild_log".into(),
        },
        primary_location: None,
        invocation: None,
        diagnostic_excerpt: request
            .include_log_excerpt
            .then(|| truncate_excerpt(&record.log_excerpt, 1024)),
        log_excerpt: request.include_log_excerpt.then_some(record.log_excerpt),
        notes,
    })
}

fn truncate_excerpt(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    text.chars().take(max_chars).collect()
}

fn build_log_headline(log_excerpt: &str) -> String {
    for line in log_excerpt.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "Build failed".into()
}

fn map_fetch_error(err: FetchBuildOutputError) -> ErrorData {
    match err {
        FetchBuildOutputError::JobNotFound { job_id } => build_error_data(
            &JOB_NOT_FOUND_ERROR,
            json!({ "job_id": job_id.to_string() }),
            SandboxState::NoViolation,
            false,
        ),
        FetchBuildOutputError::ArtifactExpired { job_id } => build_error_data(
            &DIAGNOSTICS_EXPIRED_ERROR,
            json!({ "job_id": job_id.to_string() }),
            SandboxState::NoViolation,
            true,
        ),
        other => visionos_fetch_error(other),
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

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempfile::tempdir;

    use crate::tools::visionos::artifacts::VisionOsArtifactStore;

    use super::*;

    #[test]
    fn build_log_headline_uses_first_non_empty_line() {
        let headline = build_log_headline("\n\n first line\nsecond");
        assert_eq!(headline, "first line");
    }

    #[tokio::test]
    async fn inspect_returns_unavailable_error_when_failure_context_missing() {
        let temp = tempdir().expect("tempdir");
        let store = VisionOsArtifactStore::with_root(temp.path().to_path_buf(), 600, 60);
        let job_id = Uuid::new_v4();
        store
            .record_failure(job_id, "failed".into(), None, Utc::now())
            .await
            .expect("record failure");

        let err = inspect_build_diagnostics(
            &store,
            InspectBuildDiagnosticsRequest {
                job_id: job_id.to_string(),
                include_log_excerpt: true,
                prefer_typecheck: true,
            },
        )
        .await
        .expect_err("should return diagnostics_unavailable");

        assert_eq!(
            err.data
                .as_ref()
                .and_then(|data| data.get("code"))
                .and_then(Value::as_str),
            Some("diagnostics_unavailable")
        );
    }
}
