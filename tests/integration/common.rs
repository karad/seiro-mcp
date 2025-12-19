use std::{io, path::PathBuf, process::Stdio};

use anyhow::{Context, Result};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, ReadBuf},
    process::{Child, ChildStdin, ChildStdout, Command},
    task::JoinHandle,
};

pub const BINARY_PATH: &str = env!("CARGO_BIN_EXE_seiro-mcp");
pub const VALID_TOKEN: &str = "valid-token-123456";

pub async fn spawn_server_process() -> Result<(Child, ChildIoBridge, Option<JoinHandle<()>>)> {
    let mut command = Command::new(BINARY_PATH);
    command
        .env(
            "MCP_CONFIG_PATH",
            fixture("tests/fixtures/config_valid.toml"),
        )
        .env("MCP_SHARED_TOKEN", VALID_TOKEN)
        .stdout(Stdio::piped())
        .stdin(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().context("failed to spawn server process")?;
    let stdout = child.stdout.take().expect("child stdout");
    let stdin = child.stdin.take().expect("child stdin");
    let bridge = ChildIoBridge::new(stdout, stdin);
    let stderr_handle = child.stderr.take().map(|mut stderr| {
        tokio::spawn(async move {
            let mut buf = Vec::new();
            let _ = stderr.read_to_end(&mut buf).await;
        })
    });
    Ok((child, bridge, stderr_handle))
}

pub fn fixture(relative: &str) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.join(relative).display().to_string()
}

pub struct ChildIoBridge {
    stdout: ChildStdout,
    stdin: ChildStdin,
}

impl ChildIoBridge {
    pub fn new(stdout: ChildStdout, stdin: ChildStdin) -> Self {
        Self { stdout, stdin }
    }
}

impl AsyncRead for ChildIoBridge {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.stdout).poll_read(cx, buf)
    }
}

impl AsyncWrite for ChildIoBridge {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        data: &[u8],
    ) -> std::task::Poll<io::Result<usize>> {
        std::pin::Pin::new(&mut self.stdin).poll_write(cx, data)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        std::pin::Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
}
