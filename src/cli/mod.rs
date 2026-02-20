//! CLI entrypoint module structure.
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde_json::json;

use crate::lib::fs::{
    install_skill_files, remove_skill_directory, resolve_codex_skills_root,
    resolve_skill_install_dir, BundledSkillFile, SkillInstallStatus, SkillRemoveStatus,
};

pub mod args;
pub mod profile;

pub use args::{
    CliCommand, LaunchProfileArgs, ParsedCommand, SkillArgs, SkillCommand, SkillInstallArgs,
    SkillRemoveArgs,
};
pub use profile::{
    build_launch_args, resolve_config_path, resolve_token, LaunchProfile, TokenSource,
    TransportMode,
};

/// Prefix reserved for Seiro-managed bundled skills.
pub const SKILL_NAME_PREFIX: &str = "seiro-mcp-";
/// Bundled skill currently shipped by this crate.
pub const BUNDLED_VISIONOS_SKILL_NAME: &str = "seiro-mcp-visionos-build-operator";
/// Canonical main markdown file name for bundled skills.
const BUNDLED_SKILL_MAIN_FILE: &str = "SKILL.md";
/// Embedded bundled skill content copied by `skill install`.
const BUNDLED_VISIONOS_SKILL_CONTENT: &str =
    include_str!("../../skills/seiro-mcp-visionos-build-operator/SKILL.md");

/// Validate skill name prefix.
pub fn validate_skill_name_prefix(skill_name: &str) -> bool {
    skill_name.starts_with(SKILL_NAME_PREFIX)
}

/// Execute CLI command mode and return a user-facing result payload.
pub fn execute_cli_command(command: CliCommand) -> Result<String> {
    match command {
        CliCommand::Skill(skill) => match skill.command {
            SkillCommand::Install(args) => {
                let destination_dir = resolve_skill_install_dir(&args.skill_name)
                    .map_err(|message| anyhow!(message))?;
                install_bundled_skill_to_destination(
                    &args.skill_name,
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

    let files = [BundledSkillFile {
        relative_path: BUNDLED_SKILL_MAIN_FILE,
        content: BUNDLED_VISIONOS_SKILL_CONTENT,
    }];

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
