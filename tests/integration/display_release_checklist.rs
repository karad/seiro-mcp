use std::{fs, path::PathBuf};

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn feature_quickstart_covers_display_metadata_review() {
    let quickstart = fs::read_to_string(repo_file("specs/017-display-metadata/quickstart.md"))
        .expect("read feature quickstart");

    for expected in [
        "seiro-mcp-logo-large.png",
        "seiro-mcp-logo-small.svg",
        "Open the Skill",
        "docs/_config.yml",
    ] {
        assert!(
            quickstart.contains(expected),
            "quickstart should mention {expected}"
        );
    }
}

#[test]
fn release_process_mentions_external_display_metadata_checks() {
    let release = fs::read_to_string(repo_file("docs/release.md")).expect("read docs/release.md");

    for expected in [
        "external display metadata",
        "seiro-mcp-logo-large.png",
        "seiro-mcp-logo-small.svg",
        "agents/openai.yaml",
    ] {
        assert!(
            release.contains(expected),
            "release doc should mention {expected}"
        );
    }
}
