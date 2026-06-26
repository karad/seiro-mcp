//! Client authentication and TTY checks performed at startup.
use std::process::ExitCode;

use anyhow::Result;
use serde_json::json;

use super::runtime::RuntimeExit;
use crate::{
    cli::LaunchProfile,
    lib::errors::{SandboxState, ToolErrorDescriptorBuilder, MCP_CLIENT_REQUIRED_ERROR},
};

pub fn ensure_invoked_via_mcp_client(profile: &LaunchProfile) -> Result<(), RuntimeExit> {
    use std::io::IsTerminal;
    let stdin_tty = std::io::stdin().is_terminal();
    let stdout_tty = std::io::stdout().is_terminal();
    if stdin_tty || stdout_tty {
        return Err(build_auth_exit(
            MCP_CLIENT_REQUIRED_ERROR.builder(),
            ExitCode::from(44),
            44,
            true,
            SandboxState::NotApplicable,
            json!({
                "transport": "stdio",
                "config_path": profile.config_path.to_string_lossy(),
                "stdin_is_tty": stdin_tty,
                "stdout_is_tty": stdout_tty
            }),
        ));
    }
    Ok(())
}

fn build_auth_exit(
    builder: ToolErrorDescriptorBuilder<'static>,
    exit_code: ExitCode,
    exit_code_raw: u8,
    retryable: bool,
    sandbox_state: SandboxState,
    details: serde_json::Value,
) -> RuntimeExit {
    let data = builder
        .retryable(retryable)
        .sandbox_state(sandbox_state)
        .details(details)
        .with_exit_code_value(exit_code_raw)
        .build()
        .expect("auth builder must succeed");
    RuntimeExit::structured(data, exit_code)
}

#[cfg(test)]
mod tests {
    use std::process::ExitCode;

    use super::*;

    #[test]
    fn build_auth_exit_preserves_exit_code() {
        let err = build_auth_exit(
            MCP_CLIENT_REQUIRED_ERROR.builder(),
            ExitCode::from(44),
            44,
            true,
            SandboxState::NotApplicable,
            json!({ "transport": "stdio" }),
        );
        assert_eq!(err.exit_code(), ExitCode::from(44));
        let data = err.error_data().expect("error data must exist");
        assert_eq!(
            data.data
                .as_ref()
                .and_then(|value| value.get("code"))
                .and_then(|v| v.as_str()),
            Some("MCP_CLIENT_REQUIRED")
        );
    }
}
