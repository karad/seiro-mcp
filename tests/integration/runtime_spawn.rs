use std::{
    process::{Command as StdCommand, Stdio},
    time::Duration,
};

use anyhow::Result;
use rmcp::{model::ClientInfo, serve_client};
use tokio::time::timeout;

use crate::common::{fixture, spawn_server_process, BINARY_PATH, VALID_TOKEN};

#[tokio::test]
async fn inspector_style_spawn_lists_tools() -> Result<()> {
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
    assert!(
        status.success(),
        "server should exit cleanly but exit status was {status:?}"
    );
    if let Some(handle) = stderr_task {
        let _ = handle.await;
    }
    Ok(())
}

#[test]
fn direct_execution_requires_mcp_client() {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        eprintln!("Skipping MCP_CLIENT_REQUIRED test because stdio is not a TTY");
        return;
    }
    let status = StdCommand::new(BINARY_PATH)
        .env(
            "MCP_CONFIG_PATH",
            fixture("tests/fixtures/config_valid.toml"),
        )
        .env("MCP_SHARED_TOKEN", VALID_TOKEN)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .status()
        .expect("process should start");
    assert_eq!(
        status.code(),
        Some(44),
        "MCP_CLIENT_REQUIRED exit code (44) expected, got {status:?}"
    );
}
