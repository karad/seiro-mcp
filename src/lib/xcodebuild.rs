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
        if key == "DEVELOPER_DIR" {
            continue;
        }
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

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::PathBuf};

    use super::*;

    #[test]
    fn env_overrides_cannot_replace_configured_developer_dir() {
        let xcodebuild_path = PathBuf::from("/usr/bin/xcodebuild");
        let xcode_path = PathBuf::from("/Applications/Xcode.app/Contents/Developer");
        let staging_dir = PathBuf::from("/tmp/staging");
        let project_path = PathBuf::from("/tmp/project");
        let extra_args = Vec::new();
        let env_overrides = BTreeMap::from([
            (
                "DEVELOPER_DIR".to_string(),
                "/tmp/untrusted/Contents/Developer".to_string(),
            ),
            ("CI".to_string(), "true".to_string()),
        ]);

        let command = build_visionos_xcodebuild_command(
            VisionOsXcodebuildCommandConfig {
                xcodebuild_path: &xcodebuild_path,
                xcode_path: &xcode_path,
                staging_dir: &staging_dir,
            },
            VisionOsXcodebuildRequest {
                project_path: &project_path,
                workspace: None,
                scheme: "VisionApp",
                configuration: "Debug",
                destination: "platform=visionOS Simulator,name=Apple Vision Pro",
                clean: false,
                extra_args: &extra_args,
                env_overrides: &env_overrides,
            },
        );

        let envs: BTreeMap<_, _> = command
            .as_std()
            .get_envs()
            .filter_map(|(key, value)| {
                value.map(|value| (key.to_os_string(), value.to_os_string()))
            })
            .collect();

        assert_eq!(
            envs.get(std::ffi::OsStr::new("DEVELOPER_DIR"))
                .map(|value| value.as_os_str()),
            Some(xcode_path.as_os_str())
        );
        assert_eq!(
            envs.get(std::ffi::OsStr::new("CI"))
                .map(|value| value.as_os_str()),
            Some(std::ffi::OsStr::new("true"))
        );
    }
}
