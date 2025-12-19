//! Shared helpers for building `xcodebuild` commands.

use std::{collections::BTreeMap, path::Path};

use tokio::process::Command;

pub struct VisionOsXcodebuildCommandConfig<'a> {
    pub xcodebuild_path: &'a Path,
    pub xcode_path: &'a Path,
    pub staging_dir: &'a Path,
}

pub struct VisionOsXcodebuildRequest<'a> {
    pub project_path: &'a Path,
    pub workspace: Option<&'a Path>,
    pub scheme: &'a str,
    pub configuration: &'a str,
    pub destination: &'a str,
    pub clean: bool,
    pub extra_args: &'a [String],
    pub env_overrides: &'a BTreeMap<String, String>,
}

/// Build an `xcodebuild` command for a visionOS build.
pub fn build_visionos_xcodebuild_command(
    config: VisionOsXcodebuildCommandConfig<'_>,
    request: VisionOsXcodebuildRequest<'_>,
) -> Command {
    let mut command = Command::new(config.xcodebuild_path);
    command.kill_on_drop(true);
    command.current_dir(request.project_path);
    command.env_clear();
    command.env("NSUnbufferedIO", "YES");
    command.env("DEVELOPER_DIR", config.xcode_path);
    command.env("VISIONOS_BUILD_ARTIFACT_DIR", config.staging_dir);
    for (key, value) in request.env_overrides {
        command.env(key, value);
    }

    if let Some(workspace) = request.workspace {
        command.arg("-workspace").arg(workspace);
    } else if request
        .project_path
        .extension()
        .and_then(|ext| ext.to_str())
        == Some("xcodeproj")
    {
        command.arg("-project").arg(request.project_path);
    }

    command.arg("-scheme").arg(request.scheme);
    command.arg("-configuration").arg(request.configuration);
    command.arg("-destination").arg(request.destination);

    if request.clean {
        command.arg("clean");
    }
    command.arg("build");

    for arg in request.extra_args {
        command.arg(arg);
    }

    command
}
