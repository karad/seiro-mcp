use std::{path::Path, process::Output};

use tokio::process::Command;

pub enum ProjectKind {
    Project,
    Workspace,
}

pub struct ListInvocationResult {
    pub invocation: String,
    pub output: Output,
}

pub async fn run_xcodebuild_list(
    xcodebuild_path: &Path,
    developer_dir: &Path,
    project_path: &Path,
    kind: ProjectKind,
) -> std::io::Result<ListInvocationResult> {
    let mut command = Command::new(xcodebuild_path);
    command.env("DEVELOPER_DIR", developer_dir);

    let invocation = match kind {
        ProjectKind::Project => {
            command
                .arg("-list")
                .arg("-json")
                .arg("-project")
                .arg(project_path);
            format!(
                "DEVELOPER_DIR={} {} -list -json -project {}",
                developer_dir.display(),
                xcodebuild_path.display(),
                project_path.display()
            )
        }
        ProjectKind::Workspace => {
            command
                .arg("-list")
                .arg("-json")
                .arg("-workspace")
                .arg(project_path);
            format!(
                "DEVELOPER_DIR={} {} -list -json -workspace {}",
                developer_dir.display(),
                xcodebuild_path.display(),
                project_path.display()
            )
        }
    };

    let output = command.output().await?;
    Ok(ListInvocationResult { invocation, output })
}
