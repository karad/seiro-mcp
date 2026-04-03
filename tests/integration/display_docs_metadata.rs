use std::{fs, path::PathBuf};

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn docs_and_skill_metadata_share_product_terms() {
    let readme = fs::read_to_string(repo_file("README.md")).expect("read README.md");
    let docs_config =
        fs::read_to_string(repo_file("docs/_config.yml")).expect("read docs/_config.yml");
    let skill_metadata = fs::read_to_string(repo_file(
        "skills/seiro-mcp-visionos-build-operator/agents/openai.yaml",
    ))
    .expect("read skill metadata");

    for expected in ["Seiro MCP", "visionOS"] {
        assert!(
            readme.contains(expected),
            "README should mention {expected}"
        );
        assert!(
            docs_config.contains(expected),
            "docs/_config.yml should mention {expected}"
        );
        assert!(
            skill_metadata.contains(expected),
            "skill metadata should mention {expected}"
        );
    }
}

#[test]
fn docs_metadata_keeps_logo_and_skill_listing_uses_skill_assets() {
    let docs_config =
        fs::read_to_string(repo_file("docs/_config.yml")).expect("read docs/_config.yml");
    let skill_metadata = fs::read_to_string(repo_file(
        "skills/seiro-mcp-visionos-build-operator/agents/openai.yaml",
    ))
    .expect("read skill metadata");

    assert!(docs_config.contains("logo: /assets/seiro-mcp-logo.png"));
    assert!(skill_metadata.contains("./assets/seiro-mcp-logo-large.png"));
    assert!(skill_metadata.contains("./assets/seiro-mcp-logo-small.svg"));
}
