use std::{
    process::{Command as StdCommand, Stdio},
    time::Duration,
};

use anyhow::Result;
use rmcp::{model::ClientInfo, serve_client};
use tokio::time::timeout;

use crate::common::{fixture, spawn_server_process, BINARY_PATH};

#[tokio::test]
async fn token_is_not_required_for_stdio_handshake() -> Result<()> {
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

#[test]
fn transport_argument_is_not_supported() {
    let output = StdCommand::new(BINARY_PATH)
        .arg("--transport=tcp")
        .env(
            "MCP_CONFIG_PATH",
            fixture("tests/fixtures/seiro_mcp_minimal.toml"),
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("process should start");

    assert!(
        !output.status.success(),
        "--transport should be rejected after TCP removal"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unexpected argument") || stderr.contains("Usage:"),
        "stderr should explain unsupported argument, got: {stderr}"
    );
}
