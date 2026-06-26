use std::{fs, path::PathBuf};

fn repo_file(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn docs_and_skill_metadata_share_product_terms() {
    let readme = fs::read_to_string(repo_file("README.md")).expect("read README.md");
    let quickstart =
        fs::read_to_string(repo_file("docs/quickstart.md")).expect("read docs/quickstart.md");
    let runbook = fs::read_to_string(repo_file("docs/runbook.md")).expect("read docs/runbook.md");
    let docs_config =
        fs::read_to_string(repo_file("docs/_config.yml")).expect("read docs/_config.yml");
    let skill_metadata = fs::read_to_string(repo_file(
        ".agents/skills/seiro-mcp-visionos-build-operator/agents/openai.yaml",
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
            quickstart.contains(expected),
            "docs/quickstart.md should mention {expected}"
        );
        assert!(
            runbook.contains(expected),
            "docs/runbook.md should mention {expected}"
        );
        assert!(
            skill_metadata.contains(expected),
            "skill metadata should mention {expected}"
        );
    }
}

#[test]
fn docs_metadata_keeps_logo_and_skill_listing_uses_skill_assets() {
    let readme = fs::read_to_string(repo_file("README.md")).expect("read README.md");
    let quickstart =
        fs::read_to_string(repo_file("docs/quickstart.md")).expect("read docs/quickstart.md");
    let runbook = fs::read_to_string(repo_file("docs/runbook.md")).expect("read docs/runbook.md");
    let docs_config =
        fs::read_to_string(repo_file("docs/_config.yml")).expect("read docs/_config.yml");
    let skill_metadata = fs::read_to_string(repo_file(
        ".agents/skills/seiro-mcp-visionos-build-operator/agents/openai.yaml",
    ))
    .expect("read skill metadata");

    assert!(docs_config.contains("logo: /assets/seiro-mcp-logo.png"));
    assert!(skill_metadata.contains("./assets/seiro-mcp-logo-large.png"));
    assert!(skill_metadata.contains("./assets/seiro-mcp-logo-small.svg"));
    assert!(readme.contains(".agents/skills/seiro-mcp-visionos-build-operator/"));
    assert!(quickstart.contains(".agents/skills/seiro-mcp-visionos-build-operator/"));
    assert!(runbook.contains(".agents/skills/seiro-mcp-visionos-build-operator/"));
}

#[test]
fn docs_explain_skill_installation_boundary() {
    let readme = fs::read_to_string(repo_file("README.md")).expect("read README.md");
    let quickstart =
        fs::read_to_string(repo_file("docs/quickstart.md")).expect("read docs/quickstart.md");
    let runbook = fs::read_to_string(repo_file("docs/runbook.md")).expect("read docs/runbook.md");

    for contents in [&readme, &quickstart, &runbook] {
        assert!(
            contents.contains("does not install the Seiro MCP server"),
            "docs must explain that skill installation does not install the server"
        );
    }

    assert!(
        readme.contains("skill-installer"),
        "README should mention skill-installer"
    );
}

#[test]
fn docs_present_minimal_config_setup_commands() {
    let readme = fs::read_to_string(repo_file("README.md")).expect("read README.md");
    let quickstart =
        fs::read_to_string(repo_file("docs/quickstart.md")).expect("read docs/quickstart.md");
    let runbook = fs::read_to_string(repo_file("docs/runbook.md")).expect("read docs/runbook.md");
    let config = fs::read_to_string(repo_file("docs/config.md")).expect("read docs/config.md");

    for contents in [&readme, &quickstart, &runbook, &config] {
        assert!(
            contents.contains("seiro-mcp config mcp"),
            "docs must show the paste-ready Codex config command"
        );
        assert!(
            contents.contains("seiro-mcp config project"),
            "docs must show the project config generator command"
        );
        assert!(
            contents.contains("seiro-mcp.toml"),
            "docs must name the project-local config file"
        );
    }

    assert!(readme.contains("[mcp_servers.seiro_mcp]"));
    assert!(config.contains("[visionos]"));
}

#[test]
fn docs_do_not_present_token_or_tcp_as_normal_setup() {
    let readme = fs::read_to_string(repo_file("README.md")).expect("read README.md");
    let quickstart =
        fs::read_to_string(repo_file("docs/quickstart.md")).expect("read docs/quickstart.md");
    let runbook = fs::read_to_string(repo_file("docs/runbook.md")).expect("read docs/runbook.md");
    let compatibility =
        fs::read_to_string(repo_file("docs/compatibility.md")).expect("read docs/compatibility.md");

    for contents in [&readme, &quickstart, &runbook] {
        assert!(
            !contents.contains("MCP_SHARED_TOKEN"),
            "normal setup docs must not require tokens"
        );
        assert!(
            !contents.contains("--transport"),
            "normal setup docs must not expose transport switching"
        );
        assert!(
            !contents.contains("tcp://"),
            "normal setup docs must not expose TCP endpoints"
        );
    }

    assert!(
        compatibility.contains("not part of the current supported runtime"),
        "compatibility docs must explicitly defer TCP support"
    );
}
