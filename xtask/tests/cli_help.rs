use std::process::Command;

fn xtask_bin() -> &'static str {
    env!("CARGO_BIN_EXE_xtask")
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
