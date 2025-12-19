use std::{
    process::{Command as StdCommand, Stdio},
    time::Duration,
};

use anyhow::Result;
use rmcp::{model::ClientInfo, serve_client};
use tokio::time::timeout;

use crate::common::{fixture, spawn_server_process, BINARY_PATH};

#[test]
fn token_mismatch_causes_auth_token_mismatch_exit() {
    let status = StdCommand::new(BINARY_PATH)
        .env(
            "MCP_CONFIG_PATH",
            fixture("tests/fixtures/config_token_mismatch.toml"),
        )
        .env("MCP_SHARED_TOKEN", "wrong-token-000000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .expect("process should start");
    assert_eq!(
        status.code(),
        Some(42),
        "AUTH_TOKEN_MISMATCH exit code (42) expected"
    );
}

#[test]
fn missing_token_causes_mcp_token_required_exit() {
    let status = StdCommand::new(BINARY_PATH)
        .env(
            "MCP_CONFIG_PATH",
            fixture("tests/fixtures/config_valid.toml"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .status()
        .expect("process should start");
    assert_eq!(
        status.code(),
        Some(43),
        "MCP_TOKEN_REQUIRED exit code (43) expected"
    );
}

#[tokio::test]
async fn matching_token_allows_handshake() -> Result<()> {
    let (mut child, transport, stderr_task) = spawn_server_process().await?;
    let client = serve_client(ClientInfo::default(), transport).await?;
    let list = client.list_tools(None).await?;
    assert!(
        list.tools
            .iter()
            .any(|tool| tool.name.as_ref() == "build_visionos_app"),
        "list_tools should include build_visionos_app: {:?}",
        list.tools
    );
    client.cancel().await?;

    let status = timeout(Duration::from_secs(5), child.wait()).await??;
    assert!(status.success(), "expected clean server shutdown");
    if let Some(handle) = stderr_task {
        let _ = handle.await;
    }
    Ok(())
}
