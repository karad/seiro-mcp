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

use crate::common::{fixture, spawn_server_process, BINARY_PATH};

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
            fixture("tests/fixtures/seiro_mcp_minimal.toml"),
        )
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
    assert!(
        stdout.contains("config"),
        "--help output should list config subcommand, got: {stdout}"
    );
    assert!(
        !stdout.contains("--transport"),
        "--help output should not expose removed transport flag, got: {stdout}"
    );
}

#[test]
fn config_mcp_prints_paste_ready_toml() {
    let output = StdCommand::new(BINARY_PATH)
        .arg("config")
        .arg("mcp")
        .output()
        .expect("config mcp should execute");

    assert!(
        output.status.success(),
        "config mcp should succeed, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("[mcp_servers.seiro_mcp]\n"));
    assert!(stdout.contains("command = \""));
    assert!(!stdout.contains("MCP_CONFIG_PATH"));
    assert!(!stdout.contains("MCP_SHARED_TOKEN"));
    assert!(!stdout.contains("working_directory"));
    assert!(
        output.stderr.is_empty(),
        "config mcp should not write stderr on success"
    );
}

#[test]
fn config_project_creates_minimal_seiro_mcp_toml() {
    let temp = tempdir().expect("can create project temp dir");
    let output = StdCommand::new(BINARY_PATH)
        .arg("config")
        .arg("project")
        .current_dir(temp.path())
        .output()
        .expect("config project should execute");

    assert!(
        output.status.success(),
        "config project should succeed, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("seiro-mcp.toml")).expect("config should exist"),
        "[visionos]\nallowed_paths = []\nallowed_schemes = []\nxcode_path = \"/Applications/Xcode.app/Contents/Developer\"\n"
    );
}

#[test]
fn config_project_preserves_existing_without_force() {
    let temp = tempdir().expect("can create project temp dir");
    let config = temp.path().join("seiro-mcp.toml");
    fs::write(&config, "existing").expect("can write existing config");

    let output = StdCommand::new(BINARY_PATH)
        .arg("config")
        .arg("project")
        .current_dir(temp.path())
        .output()
        .expect("config project should execute");

    assert!(
        !output.status.success(),
        "config project should fail when config exists"
    );
    assert_eq!(
        fs::read_to_string(&config).expect("config should still exist"),
        "existing"
    );
}

#[test]
fn config_project_force_overwrites_existing() {
    let temp = tempdir().expect("can create project temp dir");
    let config = temp.path().join("seiro-mcp.toml");
    fs::write(&config, "existing").expect("can write existing config");

    let output = StdCommand::new(BINARY_PATH)
        .arg("config")
        .arg("project")
        .arg("--force")
        .current_dir(temp.path())
        .output()
        .expect("config project should execute");

    assert!(
        output.status.success(),
        "config project --force should succeed, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_ne!(
        fs::read_to_string(&config).expect("config should exist"),
        "existing"
    );
}

#[test]
fn skill_install_creates_bundled_skill_file() {
    let temp = tempdir().expect("can create temp directory for CODEX_HOME");
    let codex_home = temp.path();

    let output = StdCommand::new(BINARY_PATH)
        .arg("skill")
        .arg("install")
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
    let canonical_skill = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".agents/skills/seiro-mcp-visionos-build-operator/SKILL.md"),
    )
    .expect("canonical skill source should exist");
    assert_eq!(
        fs::read_to_string(&installed_skill).expect("installed skill should be readable"),
        canonical_skill,
        "installed skill should match canonical bundled source"
    );
    assert!(
        codex_home
            .join(".codex/skills")
            .join("seiro-mcp-visionos-build-operator")
            .join("agents/openai.yaml")
            .exists(),
        "agents/openai.yaml should be created"
    );
    assert!(
        codex_home
            .join(".codex/skills")
            .join("seiro-mcp-visionos-build-operator")
            .join("assets/seiro-mcp-logo-large.png")
            .exists(),
        "large icon should be created"
    );
    assert!(
        codex_home
            .join(".codex/skills")
            .join("seiro-mcp-visionos-build-operator")
            .join("assets/seiro-mcp-logo-small.svg")
            .exists(),
        "small icon should be created"
    );
    let installed_metadata = codex_home
        .join(".codex/skills")
        .join("seiro-mcp-visionos-build-operator")
        .join("agents/openai.yaml");
    assert_eq!(
        fs::read_to_string(&installed_metadata).expect("installed metadata should be readable"),
        fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(".agents/skills/seiro-mcp-visionos-build-operator/agents/openai.yaml"),
        )
        .expect("canonical metadata should exist"),
        "installed metadata should match canonical bundled source"
    );
}

#[test]
fn skill_install_accepts_explicit_bundled_skill_name() {
    let temp = tempdir().expect("can create temp directory for CODEX_HOME");

    let output = StdCommand::new(BINARY_PATH)
        .arg("skill")
        .arg("install")
        .arg("seiro-mcp-visionos-build-operator")
        .arg("--dry-run")
        .env("CODEX_HOME", temp.path())
        .output()
        .expect("skill install should execute");

    assert!(
        output.status.success(),
        "explicit skill install should still succeed, but status was {:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\"skill_name\": \"seiro-mcp-visionos-build-operator\""),
        "payload should contain explicit skill name: {stdout}"
    );
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
