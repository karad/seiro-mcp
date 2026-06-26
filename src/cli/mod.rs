//! CLI entrypoint module structure.
use std::{fs, path::Path};

use anyhow::{anyhow, Context, Result};
use serde_json::json;

use crate::lib::fs::{
    install_skill_files, remove_skill_directory, resolve_codex_skills_root,
    resolve_skill_install_dir, BundledSkillFile, SkillInstallStatus, SkillRemoveStatus,
};

pub mod args;
pub mod profile;

pub use args::{
    CliCommand, ConfigArgs, ConfigCommand, ConfigProjectArgs, LaunchProfileArgs, ParsedCommand,
    SkillArgs, SkillCommand, SkillInstallArgs, SkillRemoveArgs,
};
pub use profile::{build_launch_args, resolve_config_path, LaunchProfile};

/// Prefix reserved for Seiro-managed bundled skills.
pub const SKILL_NAME_PREFIX: &str = "seiro-mcp-";
/// Bundled skill currently shipped by this crate.
pub const BUNDLED_VISIONOS_SKILL_NAME: &str = "seiro-mcp-visionos-build-operator";
/// Canonical main markdown file name for bundled skills.
const BUNDLED_SKILL_MAIN_FILE: &str = "SKILL.md";
/// Embedded bundled skill content copied by `skill install`.
const BUNDLED_VISIONOS_SKILL_CONTENT: &[u8] =
    include_bytes!("../../.agents/skills/seiro-mcp-visionos-build-operator/SKILL.md");
/// Embedded OpenAI skill interface metadata copied by `skill install`.
const BUNDLED_VISIONOS_OPENAI_METADATA_PATH: &str = "agents/openai.yaml";
const BUNDLED_VISIONOS_OPENAI_METADATA: &[u8] =
    include_bytes!("../../.agents/skills/seiro-mcp-visionos-build-operator/agents/openai.yaml");
/// Embedded skill listing icon assets copied by `skill install`.
const BUNDLED_VISIONOS_LARGE_ICON_PATH: &str = "assets/seiro-mcp-logo-large.png";
const BUNDLED_VISIONOS_LARGE_ICON: &[u8] = include_bytes!(
    "../../.agents/skills/seiro-mcp-visionos-build-operator/assets/seiro-mcp-logo-large.png"
);
const BUNDLED_VISIONOS_SMALL_ICON_PATH: &str = "assets/seiro-mcp-logo-small.svg";
const BUNDLED_VISIONOS_SMALL_ICON: &[u8] = include_bytes!(
    "../../.agents/skills/seiro-mcp-visionos-build-operator/assets/seiro-mcp-logo-small.svg"
);
const PROJECT_CONFIG_FILE: &str = "seiro-mcp.toml";
const PROJECT_CONFIG_TEMPLATE: &str = r#"[visionos]
allowed_paths = []
allowed_schemes = []
xcode_path = "/Applications/Xcode.app/Contents/Developer"
"#;

/// Validate skill name prefix.
pub fn validate_skill_name_prefix(skill_name: &str) -> bool {
    skill_name.starts_with(SKILL_NAME_PREFIX)
}

fn resolve_install_skill_name(skill_name: Option<String>) -> String {
    skill_name.unwrap_or_else(|| BUNDLED_VISIONOS_SKILL_NAME.to_string())
}

/// Execute CLI command mode and return a user-facing result payload.
pub fn execute_cli_command(command: CliCommand) -> Result<String> {
    match command {
        CliCommand::Config(config) => match config.command {
            ConfigCommand::Mcp => render_mcp_config_snippet(),
            ConfigCommand::Project(args) => write_project_config(args.force),
        },
        CliCommand::Skill(skill) => match skill.command {
            SkillCommand::Install(args) => {
                let skill_name = resolve_install_skill_name(args.skill_name);
                let destination_dir =
                    resolve_skill_install_dir(&skill_name).map_err(|message| anyhow!(message))?;
                install_bundled_skill_to_destination(
                    &skill_name,
                    &destination_dir,
                    args.force,
                    args.dry_run,
                )
            }
            SkillCommand::Remove(args) => {
                let skills_root =
                    resolve_codex_skills_root().map_err(|message| anyhow!(message))?;
                let destination_dir = skills_root.join(&args.skill_name);
                remove_bundled_skill_from_destination(
                    &args.skill_name,
                    &skills_root,
                    &destination_dir,
                )
            }
        },
    }
}

/// Render TOML that can be pasted into Codex MCP settings.
pub fn render_mcp_config_snippet() -> Result<String> {
    let current_exe =
        std::env::current_exe().context("failed to resolve current executable path")?;
    if !current_exe.is_absolute() {
        return Err(anyhow!("resolved executable path is not absolute"));
    }
    Ok(format!(
        "[mcp_servers.seiro_mcp]\ncommand = \"{}\"",
        current_exe.display()
    ))
}

/// Create the project-local Seiro MCP config file.
pub fn write_project_config(force: bool) -> Result<String> {
    let cwd = std::env::current_dir().context("failed to obtain current directory")?;
    write_project_config_in_dir(&cwd, force)
}

fn write_project_config_in_dir(directory: &Path, force: bool) -> Result<String> {
    let destination = directory.join(PROJECT_CONFIG_FILE);
    if destination.exists() && !force {
        return Err(anyhow!(
            "{} already exists; re-run with --force to overwrite",
            PROJECT_CONFIG_FILE
        ));
    }
    fs::write(&destination, PROJECT_CONFIG_TEMPLATE).with_context(|| {
        format!(
            "failed to write project config to {}",
            destination.to_string_lossy()
        )
    })?;
    Ok(format!(
        "created {}",
        destination
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(PROJECT_CONFIG_FILE)
    ))
}

/// Install bundled skill files and format a JSON response payload.
fn install_bundled_skill_to_destination(
    skill_name: &str,
    destination_dir: &Path,
    force: bool,
    dry_run: bool,
) -> Result<String> {
    if skill_name != BUNDLED_VISIONOS_SKILL_NAME {
        return Err(anyhow!(
            "unsupported bundled skill: only `{BUNDLED_VISIONOS_SKILL_NAME}` is available"
        ));
    }

    let files = [
        BundledSkillFile {
            relative_path: BUNDLED_SKILL_MAIN_FILE,
            content: BUNDLED_VISIONOS_SKILL_CONTENT,
        },
        BundledSkillFile {
            relative_path: BUNDLED_VISIONOS_OPENAI_METADATA_PATH,
            content: BUNDLED_VISIONOS_OPENAI_METADATA,
        },
        BundledSkillFile {
            relative_path: BUNDLED_VISIONOS_LARGE_ICON_PATH,
            content: BUNDLED_VISIONOS_LARGE_ICON,
        },
        BundledSkillFile {
            relative_path: BUNDLED_VISIONOS_SMALL_ICON_PATH,
            content: BUNDLED_VISIONOS_SMALL_ICON,
        },
    ];

    let result =
        install_skill_files(destination_dir, &files, force, dry_run).with_context(|| {
            format!(
                "failed to write skill files to {}",
                destination_dir.to_string_lossy()
            )
        })?;

    let (status, message) = match result.status {
        SkillInstallStatus::Planned => ("planned", "dry-run: no files were modified"),
        SkillInstallStatus::Installed => ("installed", "skill installed"),
        SkillInstallStatus::SkippedExisting => (
            "skipped_existing",
            "skill files already exist; re-run with --force to overwrite",
        ),
    };

    let payload = json!({
        "status": status,
        "skill_name": skill_name,
        "destination_dir": destination_dir.to_string_lossy(),
        "written_files": result.written_files,
        "message": message
    });

    Ok(serde_json::to_string_pretty(&payload)?)
}

/// Remove bundled skill files and format a JSON response payload.
fn remove_bundled_skill_from_destination(
    skill_name: &str,
    skills_root: &Path,
    destination_dir: &Path,
) -> Result<String> {
    if skill_name != BUNDLED_VISIONOS_SKILL_NAME {
        return Err(anyhow!(
            "unsupported bundled skill: only `{BUNDLED_VISIONOS_SKILL_NAME}` is available"
        ));
    }
    if destination_dir.parent() != Some(skills_root) {
        return Err(anyhow!(
            "refusing to remove skill outside of codex skills root"
        ));
    }

    let result = remove_skill_directory(destination_dir).with_context(|| {
        format!(
            "failed to remove skill directory {}",
            destination_dir.to_string_lossy()
        )
    })?;

    let (status, message) = match result.status {
        SkillRemoveStatus::Removed => ("removed", "skill removed"),
        SkillRemoveStatus::NotFound => ("not_found", "skill not found"),
    };

    let payload = json!({
        "status": status,
        "skill_name": skill_name,
        "destination_dir": destination_dir.to_string_lossy(),
        "removed_files": result.removed_files,
        "message": message
    });

    Ok(serde_json::to_string_pretty(&payload)?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn skill_install_dry_run_does_not_create_files() {
        let temp = tempdir().expect("can create temporary directory");
        let destination = temp.path().join(BUNDLED_VISIONOS_SKILL_NAME);

        let payload = install_bundled_skill_to_destination(
            BUNDLED_VISIONOS_SKILL_NAME,
            &destination,
            false,
            true,
        )
        .expect("dry-run should succeed");

        assert!(
            !destination.exists(),
            "destination directory must not be created in dry-run"
        );
        assert!(
            payload.contains("\"status\": \"planned\""),
            "payload: {payload}"
        );
    }

    #[test]
    fn resolve_install_skill_name_defaults_to_bundled_visionos_skill() {
        assert_eq!(
            resolve_install_skill_name(None),
            BUNDLED_VISIONOS_SKILL_NAME
        );
        assert_eq!(
            resolve_install_skill_name(Some("seiro-mcp-custom".to_string())),
            "seiro-mcp-custom"
        );
    }

    #[test]
    fn render_mcp_config_snippet_contains_only_codex_mcp_toml() {
        let snippet = render_mcp_config_snippet().expect("snippet should render");
        assert!(snippet.starts_with("[mcp_servers.seiro_mcp]\n"));
        assert!(snippet.contains("command = \""));
        assert!(!snippet.contains("MCP_CONFIG_PATH"));
        assert!(!snippet.contains("MCP_SHARED_TOKEN"));
        assert!(!snippet.contains("working_directory"));
    }

    #[test]
    fn write_project_config_creates_minimal_file() {
        let temp = tempdir().expect("can create temporary directory");
        let result = write_project_config_in_dir(temp.path(), false);

        result.expect("project config should be created");
        assert_eq!(
            fs::read_to_string(temp.path().join(PROJECT_CONFIG_FILE)).expect("can read config"),
            PROJECT_CONFIG_TEMPLATE
        );
    }

    #[test]
    fn write_project_config_preserves_existing_without_force() {
        let temp = tempdir().expect("can create temporary directory");
        fs::write(temp.path().join(PROJECT_CONFIG_FILE), "existing").expect("can write existing");
        let result = write_project_config_in_dir(temp.path(), false);

        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(temp.path().join(PROJECT_CONFIG_FILE)).expect("can read config"),
            "existing"
        );
    }

    #[test]
    fn write_project_config_overwrites_with_force() {
        let temp = tempdir().expect("can create temporary directory");
        fs::write(temp.path().join(PROJECT_CONFIG_FILE), "existing").expect("can write existing");
        let result = write_project_config_in_dir(temp.path(), true);

        result.expect("force should overwrite");
        assert_eq!(
            fs::read_to_string(temp.path().join(PROJECT_CONFIG_FILE)).expect("can read config"),
            PROJECT_CONFIG_TEMPLATE
        );
    }

    #[test]
    fn skill_remove_only_targets_named_directory() {
        let temp = tempdir().expect("can create temporary directory");
        let skills_root = temp.path().join("skills");
        let target_dir = skills_root.join(BUNDLED_VISIONOS_SKILL_NAME);
        let sibling_dir = skills_root.join("seiro-mcp-other-skill");
        fs::create_dir_all(&target_dir).expect("can create target directory");
        fs::create_dir_all(&sibling_dir).expect("can create sibling directory");
        fs::write(target_dir.join("SKILL.md"), "target").expect("can write target skill");
        fs::write(sibling_dir.join("SKILL.md"), "sibling").expect("can write sibling skill");

        let payload = remove_bundled_skill_from_destination(
            BUNDLED_VISIONOS_SKILL_NAME,
            &skills_root,
            &target_dir,
        )
        .expect("remove should succeed");

        assert!(!target_dir.exists(), "target directory should be removed");
        assert!(sibling_dir.exists(), "sibling directory must remain");
        assert!(
            payload.contains("\"status\": \"removed\""),
            "payload should indicate removed: {payload}"
        );
    }

    #[test]
    fn skill_remove_returns_not_found_without_error() {
        let temp = tempdir().expect("can create temporary directory");
        let skills_root = temp.path().join("skills");
        let destination = skills_root.join(BUNDLED_VISIONOS_SKILL_NAME);

        let payload = remove_bundled_skill_from_destination(
            BUNDLED_VISIONOS_SKILL_NAME,
            &skills_root,
            &destination,
        )
        .expect("not found should not be an error");

        assert!(
            payload.contains("\"status\": \"not_found\""),
            "payload should indicate not_found: {payload}"
        );
    }
}
