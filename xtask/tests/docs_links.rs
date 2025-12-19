use std::process::Command;

fn xtask_bin() -> &'static str {
    env!("CARGO_BIN_EXE_xtask")
}

#[test]
fn check_docs_links_succeeds_for_repository_docs() {
    let output = Command::new(xtask_bin())
        .args(["check-docs-links"])
        .output()
        .expect("xtask should run");
    assert!(
        output.status.success(),
        "xtask check-docs-links should succeed, stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
