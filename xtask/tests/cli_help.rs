use std::process::Command;
use std::{fs, path::PathBuf};

fn xtask_bin() -> &'static str {
    env!("CARGO_BIN_EXE_xtask")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should have repository parent")
        .to_path_buf()
}

fn run_seiro_help(args: &[&str]) -> std::process::Output {
    Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--")
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("seiro-mcp help command should run")
}

#[test]
fn xtask_help_lists_expected_commands() {
    let output = Command::new(xtask_bin())
        .arg("--help")
        .output()
        .expect("xtask should run");
    assert!(output.status.success(), "xtask --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in [
        "preflight",
        "langscan",
        "docs-langscan",
        "check-docs-links",
        "loc-baseline",
        "loc-guard",
        "api-baseline",
        "refactor-check-docs",
    ] {
        assert!(
            stdout.contains(needle),
            "xtask --help should list {needle}, got:\n{stdout}"
        );
    }
}

#[test]
fn xtask_preflight_help_is_present() {
    let output = Command::new(xtask_bin())
        .args(["preflight", "--help"])
        .output()
        .expect("xtask should run");
    assert!(
        output.status.success(),
        "xtask preflight --help should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("xtask preflight") || stdout.contains("Usage: xtask preflight"),
        "help should mention usage, got:\n{stdout}"
    );
}

#[test]
fn release_process_lists_required_publish_commands() {
    let root = repo_root();
    let release_doc = root.join("docs/release.md");
    let content = fs::read_to_string(&release_doc).expect("release process doc should be readable");

    for needle in [
        "cargo check",
        "cargo test --all",
        "cargo fmt -- --check",
        "cargo clippy -- -D warnings",
        "cargo build --release",
        "cargo package",
        "cargo publish --dry-run",
    ] {
        assert!(
            content.contains(needle),
            "docs/release.md should contain `{needle}`, got:\n{content}"
        );
    }
}

#[test]
fn seiro_root_help_lists_skill_subcommand() {
    let output = run_seiro_help(&["--help"]);
    assert!(output.status.success(), "seiro-mcp --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for needle in ["skill", "install", "remove"] {
        assert!(
            stdout.contains(needle),
            "--help should list {needle}, got:\n{stdout}"
        );
    }
}

#[test]
fn seiro_skill_help_lists_install_remove_and_dry_run() {
    let skill_output = run_seiro_help(&["skill", "--help"]);
    assert!(
        skill_output.status.success(),
        "seiro-mcp skill --help should succeed"
    );
    let skill_stdout = String::from_utf8_lossy(&skill_output.stdout);
    for needle in ["install", "remove"] {
        assert!(
            skill_stdout.contains(needle),
            "skill --help should list {needle}, got:\n{skill_stdout}"
        );
    }
    assert!(
        skill_stdout.contains("--dry-run"),
        "skill --help should mention --dry-run behavior, got:\n{skill_stdout}"
    );

    let install_output = run_seiro_help(&["skill", "install", "--help"]);
    assert!(
        install_output.status.success(),
        "seiro-mcp skill install --help should succeed"
    );
    let install_stdout = String::from_utf8_lossy(&install_output.stdout);
    assert!(
        install_stdout.contains("--dry-run"),
        "skill install --help should list --dry-run, got:\n{install_stdout}"
    );
}

#[test]
fn seiro_version_output_uses_name_and_semver_format() {
    let output = run_seiro_help(&["--version"]);
    assert!(
        output.status.success(),
        "seiro-mcp --version should succeed"
    );

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mut parts = stdout.split_whitespace();
    let name = parts.next().unwrap_or_default();
    let version = parts.next().unwrap_or_default();
    let no_extra = parts.next().is_none();

    assert_eq!(name, "seiro-mcp", "unexpected binary name: {stdout}");
    assert!(
        version.chars().all(|c| c.is_ascii_digit() || c == '.') && version.split('.').count() == 3,
        "version should look like SemVer (X.Y.Z), got: {stdout}"
    );
    assert!(
        no_extra,
        "version output should be two tokens, got: {stdout}"
    );
}
