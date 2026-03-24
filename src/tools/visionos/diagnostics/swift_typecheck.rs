use std::{
    env, fs,
    path::{Path, PathBuf},
};

use tokio::process::Command;

use crate::tools::visionos::artifacts::BuildFailureContext;

const DEFAULT_TARGET: &str = "arm64-apple-xros26.2-simulator";

/// Typecheck command execution result.
#[derive(Debug, Clone)]
pub struct TypecheckRunOutput {
    pub invocation: String,
    pub stdout: String,
    pub stderr: String,
}

/// Run `swiftc -typecheck` for build diagnostics.
pub async fn run_typecheck(context: &BuildFailureContext) -> Result<TypecheckRunOutput, String> {
    if let Some(mocked) = run_mocked_typecheck(context) {
        return mocked;
    }

    let swift_files = collect_swift_files(project_source_root(&context.project_path))?;
    if swift_files.is_empty() {
        return Err("No Swift source files found for diagnostics".into());
    }

    let sdk_path = resolve_sdk_path(context).await?;

    let mut command = Command::new("xcrun");
    command.arg("swiftc");
    command.arg("-typecheck");
    command.arg("-sdk").arg(&sdk_path);
    command.arg("-target").arg(DEFAULT_TARGET);
    for file in &swift_files {
        command.arg(file);
    }

    apply_developer_dir_overrides(&mut command, context);

    let output = command
        .output()
        .await
        .map_err(|err| format!("Failed to run swiftc -typecheck: {err}"))?;

    Ok(TypecheckRunOutput {
        invocation: format!(
            "xcrun swiftc -typecheck -sdk {} -target {} <{} swift files>",
            sdk_path,
            DEFAULT_TARGET,
            swift_files.len()
        ),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn run_mocked_typecheck(
    context: &BuildFailureContext,
) -> Option<Result<TypecheckRunOutput, String>> {
    let behavior = env::var("MOCK_DIAGNOSTICS_BEHAVIOR").ok()?;
    match behavior.as_str() {
        "typecheck_error" => {
            let file = find_first_swift_file(context).unwrap_or_else(|| {
                context
                    .project_path
                    .join("Sources")
                    .join("ContentView.swift")
            });
            Some(Ok(TypecheckRunOutput {
                invocation: "xcrun swiftc -typecheck -sdk <mock-sdk> -target arm64-apple-xros26.2-simulator <mock-files>".into(),
                stdout: String::new(),
                stderr: format!(
                    "{}:105:22: error: type 'ShapeResource' has no member 'generateCylinder'",
                    file.display()
                ),
            }))
        }
        "typecheck_unavailable" => Some(Err("mocked typecheck failure".into())),
        "typecheck_no_location" => Some(Ok(TypecheckRunOutput {
            invocation:
                "xcrun swiftc -typecheck -sdk <mock-sdk> -target arm64-apple-xros26.2-simulator <mock-files>"
                    .into(),
            stdout: String::new(),
            stderr: "error: linker command failed with exit code 1".into(),
        })),
        _ => Some(Err(format!("Unsupported MOCK_DIAGNOSTICS_BEHAVIOR={behavior}"))),
    }
}

fn find_first_swift_file(context: &BuildFailureContext) -> Option<PathBuf> {
    collect_swift_files(project_source_root(&context.project_path))
        .ok()
        .and_then(|mut files| files.drain(..).next())
}

async fn resolve_sdk_path(context: &BuildFailureContext) -> Result<String, String> {
    let mut command = Command::new("xcrun");
    command
        .arg("--sdk")
        .arg("xrsimulator")
        .arg("--show-sdk-path");
    apply_developer_dir_overrides(&mut command, context);

    let output = command
        .output()
        .await
        .map_err(|err| format!("Failed to run xcrun --show-sdk-path: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "xcrun --show-sdk-path failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let sdk_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sdk_path.is_empty() {
        return Err("xcrun returned an empty SDK path".into());
    }

    Ok(sdk_path)
}

fn project_source_root(project_path: &Path) -> PathBuf {
    match project_path.extension().and_then(|value| value.to_str()) {
        Some("xcodeproj") | Some("xcworkspace") => {
            project_path.parent().unwrap_or(project_path).to_path_buf()
        }
        _ => project_path.to_path_buf(),
    }
}

fn collect_swift_files(root: PathBuf) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    let mut stack = vec![root];

    while let Some(dir) = stack.pop() {
        let read_dir = fs::read_dir(&dir)
            .map_err(|err| format!("Failed to scan source directory {}: {err}", dir.display()))?;

        for entry in read_dir {
            let entry =
                entry.map_err(|err| format!("Failed to read source directory entry: {err}"))?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|err| {
                format!("Failed to inspect source entry {}: {err}", path.display())
            })?;

            if file_type.is_dir() {
                if should_skip_dir(&path) {
                    continue;
                }
                stack.push(path);
                continue;
            }

            if file_type.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("swift"))
            {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn should_skip_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git") | Some("target") | Some("build") | Some(".build") | Some("DerivedData")
    )
}

fn apply_developer_dir_overrides(command: &mut Command, context: &BuildFailureContext) {
    command.env("DEVELOPER_DIR", &context.xcode_path);
    for (key, value) in &context.env_overrides {
        command.env(key, value);
    }
}
