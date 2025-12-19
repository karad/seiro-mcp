//! Telemetry initialization and visionOS job span helpers.

use std::time::Instant;

use anyhow::Result;
use serde::Serialize;
use tracing::{info, info_span, Span};
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;

/// Initialize `tracing` and format developer logs.
pub fn init_tracing() -> Result<()> {
    if tracing::dispatcher::has_been_set() {
        return Ok(());
    }

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_writer(std::io::stderr)
        .try_init()
        .map_err(|err| anyhow::anyhow!("failed to initialize tracing: {err}"))
}

/// Span helper to record start and finish of a visionOS job.
pub struct JobSpan {
    span: Span,
    started_at: Instant,
    job_id: Uuid,
}

impl JobSpan {
    /// Start a job span.
    pub fn start(job_id: Uuid, job_kind: &'static str) -> Self {
        let span = info_span!(
            target: "rmcp_sample::visionos",
            "visionos_job",
            %job_id,
            job_kind
        );
        Self {
            span,
            started_at: Instant::now(),
            job_id,
        }
    }

    /// Close the span while recording status and completion info.
    pub fn finish(self, status: &'static str, exit_code: Option<i32>) {
        let elapsed_ms = self.started_at.elapsed().as_millis();
        let _entered = self.span.enter();
        info!(
            target: "rmcp_sample::visionos",
            job_id = %self.job_id,
            status = status,
            exit_code = exit_code,
            elapsed_ms = elapsed_ms,
            "Completed visionOS job"
        );
    }
}

/// Payload for logging MCP runtime state as structured telemetry.
#[derive(Debug, Serialize)]
pub struct RuntimeModeTelemetry<'a> {
    pub transport: &'a str,
    pub host: Option<&'a str>,
    pub port: Option<u16>,
    pub config_path: &'a str,
    pub pending_jobs: usize,
    pub instructions: &'a str,
    pub launch_args: &'a [String],
}

/// Emit runtime mode to `tracing`.
pub fn emit_runtime_mode(telemetry: &RuntimeModeTelemetry<'_>) {
    info!(
        target: "rmcp_sample::runtime",
        transport = telemetry.transport,
        host = telemetry.host.unwrap_or(""),
        port = telemetry.port.unwrap_or_default(),
        config_path = telemetry.config_path,
        pending_jobs = telemetry.pending_jobs,
        instructions = telemetry.instructions,
        launch_args = ?telemetry.launch_args,
        "Started MCP server"
    );
}
