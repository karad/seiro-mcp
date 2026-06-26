use std::process::ExitCode;

use anyhow::Error;
use rmcp::ServiceExt;

use crate::{
    cli::LaunchProfile,
    server::{
        auth,
        config::ServerConfig,
        runtime::{build_instructions, VisionOsServer},
    },
};

/// Bundles a runtime error message with an exit code and optional structured error data.
#[derive(Debug)]
pub struct RuntimeExit {
    message: String,
    exit_code: ExitCode,
    error_data: Option<rmcp::model::ErrorData>,
}

impl RuntimeExit {
    pub fn structured(error: rmcp::model::ErrorData, exit_code: ExitCode) -> Self {
        Self {
            message: error.message.to_string(),
            exit_code,
            error_data: Some(error),
        }
    }

    pub fn from_error(err: impl Into<Error>) -> Self {
        let err = err.into();
        Self {
            message: format!("{err:?}"),
            exit_code: ExitCode::FAILURE,
            error_data: None,
        }
    }

    pub fn report(self) -> ExitCode {
        if let Some(data) = self.error_data {
            if let Ok(serialized) = serde_json::to_string(&data) {
                eprintln!("{serialized}");
            } else {
                eprintln!("{}", data.message);
            }
        } else {
            eprintln!("{}", self.message);
        }
        self.exit_code
    }

    pub fn exit_code(&self) -> ExitCode {
        self.exit_code
    }

    pub fn error_data(&self) -> Option<&rmcp::model::ErrorData> {
        self.error_data.as_ref()
    }
}

/// Start the MCP server and select stdio/TCP based on the launch profile.
pub async fn run_server(profile: LaunchProfile, config: ServerConfig) -> Result<(), RuntimeExit> {
    auth::ensure_invoked_via_mcp_client(&profile)?;

    let instructions = build_instructions(&profile, &config);
    let server = VisionOsServer::new(config.clone(), instructions.clone());
    let pending_jobs = server.pending_jobs().await;

    crate::lib::telemetry::emit_runtime_mode(&crate::lib::telemetry::RuntimeModeTelemetry {
        transport: "stdio",
        config_path: config.source_path.to_string_lossy().as_ref(),
        pending_jobs,
        instructions: &instructions,
        launch_args: &profile.launch_args,
    });

    run_stdio(server).await
}

async fn run_stdio(server: VisionOsServer) -> Result<(), RuntimeExit> {
    let running = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(RuntimeExit::from_error)?;
    running.waiting().await.map_err(RuntimeExit::from_error)?;
    Ok(())
}
