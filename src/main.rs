//! Entry point for Seiro MCP.
use std::process::ExitCode;

use anyhow::Error;
use clap::Parser;
use seiro_mcp::{
    cli::LaunchProfileArgs,
    lib::telemetry,
    server::{
        config::ServerConfig,
        runtime::{self, RuntimeExit},
    },
};

#[tokio::main]
async fn main() -> ExitCode {
    match bootstrap().await {
        Ok(_) => ExitCode::SUCCESS,
        Err(exit) => exit.report(),
    }
}

async fn bootstrap() -> Result<(), RuntimeExit> {
    telemetry::init_tracing().map_err(RuntimeExit::from_error)?;
    let args = LaunchProfileArgs::parse();
    let profile = args.build().map_err(RuntimeExit::from_error)?;
    let config = ServerConfig::load_from_path(profile.config_path.clone())
        .map_err(|err| RuntimeExit::from_error(Error::new(err)))?;
    runtime::run_server(profile, config).await
}
