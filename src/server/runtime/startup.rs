use std::process::ExitCode;

use anyhow::{Context, Error};
use rmcp::ServiceExt;
use tokio::net::TcpListener;

use crate::{
    cli::{LaunchProfile, TransportMode},
    server::{
        auth::{self, ClientAuthContext},
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
    let auth_context = ClientAuthContext::new(
        config.auth.token.clone(),
        profile.shared_token.clone(),
        profile.token_source,
    );
    auth_context.ensure_authorized()?;

    let instructions = build_instructions(&profile, &config);
    let server = VisionOsServer::new(config.clone(), instructions.clone());
    let pending_jobs = server.pending_jobs().await;

    crate::lib::telemetry::emit_runtime_mode(&crate::lib::telemetry::RuntimeModeTelemetry {
        transport: profile.transport.as_str(),
        host: Some(config.server.host.as_str()),
        port: Some(config.server.port),
        config_path: config.source_path.to_string_lossy().as_ref(),
        pending_jobs,
        instructions: &instructions,
        launch_args: &profile.launch_args,
    });

    match profile.transport {
        TransportMode::Stdio => run_stdio(server).await,
        TransportMode::Tcp => run_tcp(server, &config).await,
    }
}

async fn run_stdio(server: VisionOsServer) -> Result<(), RuntimeExit> {
    let running = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(RuntimeExit::from_error)?;
    running.waiting().await.map_err(RuntimeExit::from_error)?;
    Ok(())
}

async fn run_tcp(server: VisionOsServer, config: &ServerConfig) -> Result<(), RuntimeExit> {
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind TCP port {addr}"))
        .map_err(RuntimeExit::from_error)?;
    tracing::info!(
        target: "rmcp_sample::runtime",
        transport = "tcp",
        bind_addr = %addr,
        "Started listening in TCP mode"
    );

    loop {
        let (stream, peer) = listener
            .accept()
            .await
            .with_context(|| format!("failed to accept TCP connection ({addr})"))
            .map_err(RuntimeExit::from_error)?;
        tracing::info!(
            target: "rmcp_sample::runtime",
            peer = %peer,
            "Accepted connection from MCP client"
        );
        let cloned = server.clone();
        let running = cloned
            .serve(stream)
            .await
            .map_err(RuntimeExit::from_error)?;
        running.waiting().await.map_err(RuntimeExit::from_error)?;
    }
}
