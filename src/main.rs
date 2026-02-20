//! Entry point for Seiro MCP.
use std::process::ExitCode;

use anyhow::Error;
use clap::Parser;
use seiro_mcp::{
    cli::{execute_cli_command, CliCommand, LaunchProfileArgs, ParsedCommand},
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
    let command = args.into_command().map_err(RuntimeExit::from_error)?;

    match command {
        ParsedCommand::RunServer(profile) => run_server(profile).await,
        ParsedCommand::Cli(command) => handle_cli_command(command),
    }
}

async fn run_server(profile: seiro_mcp::cli::LaunchProfile) -> Result<(), RuntimeExit> {
    let config = ServerConfig::load_from_path(profile.config_path.clone())
        .map_err(|err| RuntimeExit::from_error(Error::new(err)))?;
    runtime::run_server(profile, config).await
}

fn handle_cli_command(command: CliCommand) -> Result<(), RuntimeExit> {
    let message = execute_cli_command(command).map_err(RuntimeExit::from_error)?;
    println!("{message}");
    Ok(())
}
