use std::process::Command;
use std::{fs, path::PathBuf};

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

#[test]
fn quickstart_publish_readiness_matches_release_process() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should have repository parent")
        .to_path_buf();
    let quickstart =
        fs::read_to_string(root.join("docs/quickstart.md")).expect("quickstart should be readable");
    let release =
        fs::read_to_string(root.join("docs/release.md")).expect("release doc should be readable");

    for needle in ["cargo package", "cargo publish --dry-run"] {
        assert!(
            quickstart.contains(needle),
            "docs/quickstart.md should contain `{needle}`, got:\n{quickstart}"
        );
        assert!(
            release.contains(needle),
            "docs/release.md should contain `{needle}`, got:\n{release}"
        );
    }
}

#[test]
fn runtime_env_requirements_are_consistent_across_docs() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should have repository parent")
        .to_path_buf();
    let readme = fs::read_to_string(root.join("README.md")).expect("README should be readable");
    let quickstart =
        fs::read_to_string(root.join("docs/quickstart.md")).expect("quickstart should be readable");
    let runbook =
        fs::read_to_string(root.join("docs/runbook.md")).expect("runbook should be readable");
    let config =
        fs::read_to_string(root.join("docs/config.md")).expect("config should be readable");

    for doc in [&readme, &quickstart, &runbook, &config] {
        assert!(
            doc.contains("MCP_CONFIG_PATH"),
            "document should mention MCP_CONFIG_PATH:\n{doc}"
        );
        assert!(
            doc.contains("MCP_SHARED_TOKEN"),
            "document should mention MCP_SHARED_TOKEN:\n{doc}"
        );
    }
}

#[test]
fn stdio_tcp_transport_examples_are_present() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should have repository parent")
        .to_path_buf();
    let readme = fs::read_to_string(root.join("README.md")).expect("README should be readable");
    let runbook =
        fs::read_to_string(root.join("docs/runbook.md")).expect("runbook should be readable");

    for needle in ["--transport=stdio", "--transport=tcp"] {
        assert!(
            readme.contains(needle),
            "README should contain `{needle}`, got:\n{readme}"
        );
        assert!(
            runbook.contains(needle),
            "docs/runbook.md should contain `{needle}`, got:\n{runbook}"
        );
    }
}

#[test]
fn skill_prefix_and_version_compat_guidance_are_documented() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should have repository parent")
        .to_path_buf();
    let readme = fs::read_to_string(root.join("README.md")).expect("README should be readable");
    let quickstart =
        fs::read_to_string(root.join("docs/quickstart.md")).expect("quickstart should be readable");
    let release =
        fs::read_to_string(root.join("docs/release.md")).expect("release should be readable");
    let skill = fs::read_to_string(root.join("skills/visionos-build-operator/SKILL.md"))
        .expect("skill file should be readable");

    assert!(
        readme.contains("seiro-mcp --version"),
        "README should mention --version compatibility check:\n{readme}"
    );
    for doc in [&readme, &quickstart, &release] {
        assert!(
            doc.contains("seiro-mcp-visionos-build-operator"),
            "document should mention bundled skill name:\n{doc}"
        );
        assert!(
            doc.contains("seiro-mcp-"),
            "document should mention skill name prefix policy:\n{doc}"
        );
    }
    assert!(
        skill.contains("name: seiro-mcp-visionos-build-operator"),
        "skill file front matter should use seiro-mcp- prefix:\n{skill}"
    );
}
