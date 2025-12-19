use std::process::Command;

fn xtask_bin() -> &'static str {
    env!("CARGO_BIN_EXE_xtask")
}

#[test]
fn langscan_ignores_excluded_paths_and_agents_md() {
    let temp = tempfile::tempdir().expect("create temp dir");
    std::fs::create_dir_all(temp.path().join("specs")).expect("create specs dir");
    std::fs::create_dir_all(temp.path().join(".codex/pr")).expect("create .codex dir");
    let japanese = "\u{65E5}\u{672C}\u{8A9E}";
    std::fs::write(temp.path().join("specs/ja.txt"), japanese).expect("write file");
    std::fs::write(temp.path().join(".codex/pr/ja.txt"), japanese).expect("write file");
    std::fs::write(temp.path().join("AGENTS.md"), japanese).expect("write file");

    let output = Command::new(xtask_bin())
        .args(["langscan", temp.path().to_string_lossy().as_ref()])
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
fn langscan_reports_japanese_text_in_scanned_paths() {
    let temp = tempfile::tempdir().expect("create temp dir");
    std::fs::create_dir_all(temp.path().join("src")).expect("create src dir");
    let japanese = "\u{65E5}\u{672C}\u{8A9E}";
    std::fs::write(temp.path().join("src/ja.txt"), japanese).expect("write file");

    let output = Command::new(xtask_bin())
        .args(["langscan", temp.path().to_string_lossy().as_ref()])
        .output()
        .expect("xtask should run");
    assert!(
        !output.status.success(),
        "expected failure; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
