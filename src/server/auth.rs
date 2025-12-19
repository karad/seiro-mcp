//! Client authentication and TTY checks performed at startup.
use std::process::ExitCode;

use anyhow::Result;
use serde_json::json;

use super::runtime::RuntimeExit;
use crate::{
    cli::{LaunchProfile, TokenSource},
    lib::errors::{
        SandboxState, ToolErrorDescriptorBuilder, AUTH_TOKEN_MISMATCH_ERROR,
        MCP_CLIENT_REQUIRED_ERROR, MCP_TOKEN_REQUIRED_ERROR,
    },
};

/// Authentication status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthStatus {
    Matched,
    Missing,
    Mismatch,
}

/// Context for comparing client-provided tokens against configuration.
#[derive(Debug, Clone)]
pub struct ClientAuthContext {
    expected_token: String,
    provided_token: Option<String>,
    token_source: TokenSource,
}

impl ClientAuthContext {
    pub fn new(
        expected_token: String,
        provided_token: Option<String>,
        token_source: TokenSource,
    ) -> Self {
        Self {
            expected_token,
            provided_token,
            token_source,
        }
    }

    pub fn status(&self) -> AuthStatus {
        match (&self.provided_token, self.expected_token.as_str()) {
            (Some(provided), expected) if provided == expected => AuthStatus::Matched,
            (Some(_), _) => AuthStatus::Mismatch,
            (None, _) => AuthStatus::Missing,
        }
    }

    /// Compare tokens and return a `RuntimeExit` on failure.
    pub fn ensure_authorized(&self) -> Result<(), RuntimeExit> {
        match self.status() {
            AuthStatus::Matched => Ok(()),
            AuthStatus::Missing => Err(build_auth_exit(
                MCP_TOKEN_REQUIRED_ERROR.builder(),
                ExitCode::from(43),
                43,
                true,
                SandboxState::NotApplicable,
                json!({ "token_source": format!("{:?}", self.token_source) }),
            )),
            AuthStatus::Mismatch => Err(build_auth_exit(
                AUTH_TOKEN_MISMATCH_ERROR.builder(),
                ExitCode::from(42),
                42,
                false,
                SandboxState::Blocked,
                json!({ "token_source": format!("{:?}", self.token_source) }),
            )),
        }
    }
}

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
                "transport": profile.transport.as_str(),
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
    fn client_auth_status_reflects_missing_token() {
        let ctx = ClientAuthContext::new("expected-token".into(), None, TokenSource::Missing);
        assert_eq!(ctx.status(), AuthStatus::Missing);
    }

    #[test]
    fn ensure_authorized_allows_matching_token() {
        let ctx = ClientAuthContext::new(
            "expected-token".into(),
            Some("expected-token".into()),
            TokenSource::Env,
        );
        ctx.ensure_authorized()
            .expect("matching token should succeed");
    }

    #[test]
    fn ensure_authorized_rejects_mismatch() {
        let ctx = ClientAuthContext::new(
            "expected-token".into(),
            Some("wrong-token".into()),
            TokenSource::Cli,
        );
        let err = ctx.ensure_authorized().expect_err("mismatch must fail");
        assert_eq!(err.exit_code(), ExitCode::from(42));
        let data = err.error_data().expect("error data must exist");
        assert_eq!(
            data.data
                .as_ref()
                .and_then(|value| value.get("code"))
                .and_then(|v| v.as_str()),
            Some("AUTH_TOKEN_MISMATCH")
        );
    }
}
