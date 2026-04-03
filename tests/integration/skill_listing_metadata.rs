use std::{fs, path::PathBuf};

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn skill_listing_metadata_declares_required_interface_fields() {
    let path = repo_file("skills/seiro-mcp-visionos-build-operator/agents/openai.yaml");
    let contents =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("failed to read {path:?}: {err}"));

    for expected in [
        "interface:",
        "display_name:",
        "short_description:",
        "icon_large: \"./assets/seiro-mcp-logo-large.png\"",
        "icon_small: \"./assets/seiro-mcp-logo-small.svg\"",
        "default_prompt:",
    ] {
        assert!(
            contents.contains(expected),
            "expected `{expected}` in {}",
            path.display()
        );
    }
}

#[test]
fn skill_listing_metadata_references_existing_assets() {
    let large =
        repo_file("skills/seiro-mcp-visionos-build-operator/assets/seiro-mcp-logo-large.png");
    let small =
        repo_file("skills/seiro-mcp-visionos-build-operator/assets/seiro-mcp-logo-small.svg");

    assert!(large.is_file(), "missing large icon: {}", large.display());
    assert!(small.is_file(), "missing small icon: {}", small.display());
}
