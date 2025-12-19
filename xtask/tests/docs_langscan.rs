use std::process::Command;

fn xtask_bin() -> &'static str {
    env!("CARGO_BIN_EXE_xtask")
}

#[test]
fn docs_langscan_ignores_docs_ja() {
    let temp = tempfile::tempdir().expect("create temp dir");
    std::fs::create_dir_all(temp.path().join("docs/ja")).expect("create docs/ja");
    std::fs::create_dir_all(temp.path().join("docs/en")).expect("create docs/en");

    let japanese = "\u{65E5}\u{672C}\u{8A9E}";
    std::fs::write(temp.path().join("docs/ja/page.md"), japanese).expect("write file");
    std::fs::write(temp.path().join("docs/en/page.md"), "# English\n").expect("write file");

    let output = Command::new(xtask_bin())
        .args([
            "docs-langscan",
            temp.path().join("docs").to_string_lossy().as_ref(),
        ])
        .output()
        .expect("xtask should run");
    assert!(
        output.status.success(),
        "expected success; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn docs_langscan_reports_japanese_text_outside_docs_ja() {
    let temp = tempfile::tempdir().expect("create temp dir");
    std::fs::create_dir_all(temp.path().join("docs/en")).expect("create docs/en");

    let japanese = "\u{65E5}\u{672C}\u{8A9E}";
    std::fs::write(temp.path().join("docs/en/page.md"), japanese).expect("write file");

    let output = Command::new(xtask_bin())
        .args([
            "docs-langscan",
            temp.path().join("docs").to_string_lossy().as_ref(),
        ])
        .output()
        .expect("xtask should run");
    assert!(
        !output.status.success(),
        "expected failure; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
