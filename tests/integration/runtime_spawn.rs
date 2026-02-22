use std::{
    fs,
    path::Path,
    process::{Command as StdCommand, Stdio},
    time::Duration,
};

use anyhow::Result;
use rmcp::{model::ClientInfo, serve_client};
use tempfile::tempdir;
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

#[test]
fn cargo_install_path_locked_produces_binary() {
    let temp = tempdir().expect("can create temp directory for cargo install root");
    let root = temp.path();

    let status = StdCommand::new("cargo")
        .arg("install")
        .arg("--path")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--locked")
        .arg("--offline")
        .arg("--root")
        .arg(root)
        .status()
        .expect("cargo install --path --locked should execute");

    assert!(
        status.success(),
        "cargo install should succeed, but status was {status:?}"
    );
    assert!(
        installed_binary(root).exists(),
        "installed binary must exist under <root>/bin"
    );
}

#[test]
fn cargo_install_binary_supports_help() {
    let temp = tempdir().expect("can create temp directory for cargo install root");
    let root = temp.path();

    let install_status = StdCommand::new("cargo")
        .arg("install")
        .arg("--path")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--locked")
        .arg("--offline")
        .arg("--root")
        .arg(root)
        .status()
        .expect("cargo install --path --locked should execute");
    assert!(
        install_status.success(),
        "cargo install should succeed before help check, but status was {install_status:?}"
    );

    let output = StdCommand::new(installed_binary(root))
        .arg("--help")
        .output()
        .expect("installed binary --help should execute");

    assert!(
        output.status.success(),
        "installed binary --help must succeed, but status was {:?}",
        output.status
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Seiro MCP"),
        "--help output should contain command description, got: {stdout}"
    );
}

#[test]
fn skill_install_creates_bundled_skill_file() {
    let temp = tempdir().expect("can create temp directory for CODEX_HOME");
    let codex_home = temp.path();

    let output = StdCommand::new(BINARY_PATH)
        .arg("skill")
        .arg("install")
        .arg("seiro-mcp-visionos-build-operator")
        .env("CODEX_HOME", codex_home)
        .output()
        .expect("skill install should execute");

    assert!(
        output.status.success(),
        "skill install should succeed, but status was {:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let installed_skill = codex_home
        .join(".codex/skills")
        .join("seiro-mcp-visionos-build-operator")
        .join("SKILL.md");
    assert!(installed_skill.exists(), "SKILL.md should be created");
}

#[test]
fn skill_install_preserves_existing_file_without_force() {
    let temp = tempdir().expect("can create temp directory for CODEX_HOME");
    let destination = temp
        .path()
        .join(".codex/skills")
        .join("seiro-mcp-visionos-build-operator");
    fs::create_dir_all(&destination).expect("can create destination directory");
    let skill_file = destination.join("SKILL.md");
    fs::write(&skill_file, "existing-content").expect("can create existing skill file");

    let output = StdCommand::new(BINARY_PATH)
        .arg("skill")
        .arg("install")
        .arg("seiro-mcp-visionos-build-operator")
        .env("CODEX_HOME", temp.path())
        .output()
        .expect("skill install should execute");

    assert!(
        output.status.success(),
        "skill install should succeed with skipped_existing, but status was {:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let content = fs::read_to_string(&skill_file).expect("existing skill file should remain");
    assert_eq!(content, "existing-content");
}

#[test]
fn skill_remove_deletes_only_target_directory() {
    let temp = tempdir().expect("can create temp directory for CODEX_HOME");
    let skills_root = temp.path().join(".codex/skills");
    let target_dir = skills_root.join("seiro-mcp-visionos-build-operator");
    let sibling_dir = skills_root.join("seiro-mcp-keep");
    fs::create_dir_all(&target_dir).expect("can create target directory");
    fs::create_dir_all(&sibling_dir).expect("can create sibling directory");
    fs::write(target_dir.join("SKILL.md"), "target").expect("can write target skill");
    fs::write(sibling_dir.join("SKILL.md"), "sibling").expect("can write sibling skill");

    let output = StdCommand::new(BINARY_PATH)
        .arg("skill")
        .arg("remove")
        .arg("seiro-mcp-visionos-build-operator")
        .env("CODEX_HOME", temp.path())
        .output()
        .expect("skill remove should execute");

    assert!(
        output.status.success(),
        "skill remove should succeed, but status was {:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!target_dir.exists(), "target directory should be removed");
    assert!(sibling_dir.exists(), "sibling directory should remain");
}

fn installed_binary(root: &Path) -> std::path::PathBuf {
    root.join("bin").join("seiro-mcp")
}
