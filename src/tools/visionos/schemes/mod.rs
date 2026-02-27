use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    lib::errors::{SandboxState, ToolErrorDescriptor},
    server::config::VisionOsConfig,
};
use rmcp::model::ErrorData;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod xcodebuild_list;

use xcodebuild_list::{run_xcodebuild_list, ProjectKind};

const PROJECT_PATH_MISSING_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "project_path_missing",
    "project_path is missing",
    "Provide project_path in request, place a .xcodeproj in current directory, or set visionos.default_project_path in config.toml.",
);
const PROJECT_PATH_INVALID_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "project_path_invalid",
    "project_path is invalid",
    "Use an absolute .xcodeproj or .xcworkspace path.",
);
const PROJECT_NOT_FOUND_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "project_not_found",
    "project_path does not exist",
    "Verify the project path and retry.",
);
const XCODE_PATH_UNAVAILABLE_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "xcode_path_unavailable",
    "xcode_path is unavailable",
    "Provide an absolute xcode_path or remove it to use the server default.",
);
const XCODEBUILD_LIST_FAILED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "xcodebuild_list_failed",
    "xcodebuild -list -json failed",
    "Check xcodebuild output and Xcode environment, then retry.",
);
const SCHEME_PARSE_FAILED_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "scheme_parse_failed",
    "Failed to parse xcodebuild list output",
    "Verify xcodebuild -list -json output format and retry.",
);
const NO_SCHEMES_FOUND_ERROR: ToolErrorDescriptor = ToolErrorDescriptor::new(
    "no_schemes_found",
    "No schemes found",
    "Confirm that schemes are shared and visible to xcodebuild.",
);

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InspectXcodeSchemesRequest {
    #[serde(default)]
    pub project_path: Option<PathBuf>,
    #[serde(default)]
    pub xcode_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct InspectXcodeSchemesResponse {
    pub status: &'static str,
    pub project_path: String,
    pub project_path_source: String,
    pub schemes: Vec<String>,
    pub invocation: String,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
enum ProjectPathSource {
    Request,
    Cwd,
    Config,
}

impl ProjectPathSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Cwd => "cwd",
            Self::Config => "config",
        }
    }
}

pub async fn inspect_xcode_schemes(
    request: InspectXcodeSchemesRequest,
    config: &VisionOsConfig,
) -> Result<InspectXcodeSchemesResponse, ErrorData> {
    let (project_path, source) =
        resolve_project_path(&request, config.default_project_path.as_deref())?;

    validate_project_path(&project_path)?;

    let effective_xcode_path = request
        .xcode_path
        .unwrap_or_else(|| config.xcode_path.clone());
    if !effective_xcode_path.is_absolute() {
        return Err(build_error_data(
            &XCODE_PATH_UNAVAILABLE_ERROR,
            json!({ "xcode_path": effective_xcode_path.to_string_lossy() }),
            SandboxState::Blocked,
            false,
        ));
    }

    let project_kind = if project_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "xcworkspace")
    {
        ProjectKind::Workspace
    } else {
        ProjectKind::Project
    };

    let invocation_result = run_xcodebuild_list(
        &config.xcodebuild_path,
        &effective_xcode_path,
        &project_path,
        project_kind,
    )
    .await
    .map_err(|err| {
        build_error_data(
            &XCODEBUILD_LIST_FAILED_ERROR,
            json!({ "details": err.to_string() }),
            SandboxState::NoViolation,
            true,
        )
    })?;
    let invocation = invocation_result.invocation;
    let output = invocation_result.output;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(build_error_data(
            &XCODEBUILD_LIST_FAILED_ERROR,
            json!({
                "invocation": invocation,
                "exit_code": output.status.code(),
                "stderr": stderr
            }),
            SandboxState::NoViolation,
            true,
        ));
    }

    let parsed: Value = serde_json::from_slice(&output.stdout).map_err(|err| {
        build_error_data(
            &SCHEME_PARSE_FAILED_ERROR,
            json!({ "details": err.to_string(), "invocation": invocation }),
            SandboxState::NoViolation,
            true,
        )
    })?;

    let mut schemes = Vec::new();
    if let Some(project_schemes) = parsed
        .get("project")
        .and_then(|project| project.get("schemes"))
        .and_then(Value::as_array)
    {
        schemes.extend(
            project_schemes
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string),
        );
    }
    if let Some(workspace_schemes) = parsed
        .get("workspace")
        .and_then(|workspace| workspace.get("schemes"))
        .and_then(Value::as_array)
    {
        schemes.extend(
            workspace_schemes
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string),
        );
    }

    let mut seen = BTreeSet::new();
    schemes.retain(|scheme| seen.insert(scheme.clone()));

    if schemes.is_empty() {
        return Err(build_error_data(
            &NO_SCHEMES_FOUND_ERROR,
            json!({ "invocation": invocation }),
            SandboxState::NoViolation,
            false,
        ));
    }

    Ok(InspectXcodeSchemesResponse {
        status: "ok",
        project_path: project_path.display().to_string(),
        project_path_source: source.as_str().to_string(),
        schemes,
        invocation,
        notes: Vec::new(),
    })
}

fn resolve_project_path(
    request: &InspectXcodeSchemesRequest,
    config_default_project_path: Option<&Path>,
) -> Result<(PathBuf, ProjectPathSource), ErrorData> {
    let cwd = std::env::current_dir().map_err(|_| {
        build_error_data(
            &PROJECT_PATH_MISSING_ERROR,
            json!({ "details": "failed to read current directory" }),
            SandboxState::NoViolation,
            false,
        )
    })?;
    resolve_project_path_in_dir(request, &cwd, config_default_project_path)
}

fn resolve_project_path_in_dir(
    request: &InspectXcodeSchemesRequest,
    cwd: &Path,
    config_default_project_path: Option<&Path>,
) -> Result<(PathBuf, ProjectPathSource), ErrorData> {
    if let Some(path) = &request.project_path {
        return Ok((path.clone(), ProjectPathSource::Request));
    }

    if let Some(path) = find_xcodeproj_in_dir(cwd) {
        return Ok((path, ProjectPathSource::Cwd));
    }

    if let Some(path) = config_default_project_path {
        return Ok((path.to_path_buf(), ProjectPathSource::Config));
    }

    Err(build_error_data(
        &PROJECT_PATH_MISSING_ERROR,
        json!({ "details": "request.project_path, cwd .xcodeproj, and config.toml visionos.default_project_path are all missing" }),
        SandboxState::NoViolation,
        false,
    ))
}

fn find_xcodeproj_in_dir(base_dir: &Path) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = fs::read_dir(base_dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext == "xcodeproj")
        })
        .collect();
    candidates.sort();
    candidates.into_iter().next()
}

fn validate_project_path(path: &Path) -> Result<(), ErrorData> {
    if !path.is_absolute() {
        return Err(build_error_data(
            &PROJECT_PATH_INVALID_ERROR,
            json!({ "project_path": path.to_string_lossy() }),
            SandboxState::NoViolation,
            false,
        ));
    }
    let ext = path.extension().and_then(|value| value.to_str());
    if ext != Some("xcodeproj") && ext != Some("xcworkspace") {
        return Err(build_error_data(
            &PROJECT_PATH_INVALID_ERROR,
            json!({ "project_path": path.to_string_lossy() }),
            SandboxState::NoViolation,
            false,
        ));
    }
    if !path.exists() {
        return Err(build_error_data(
            &PROJECT_NOT_FOUND_ERROR,
            json!({ "project_path": path.to_string_lossy() }),
            SandboxState::NoViolation,
            false,
        ));
    }
    Ok(())
}

fn build_error_data(
    desc: &'static ToolErrorDescriptor,
    details: Value,
    sandbox_state: SandboxState,
    retryable: bool,
) -> ErrorData {
    desc.builder()
        .details(details)
        .sandbox_state(sandbox_state)
        .retryable(retryable)
        .build()
        .expect("descriptor is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_temp_dir(label: &str) -> PathBuf {
        let unique = Uuid::new_v4();
        let dir = std::env::temp_dir().join(format!("seiro-schemes-{label}-{unique}"));
        fs::create_dir_all(&dir).expect("temp dir should be created");
        dir
    }

    fn sample_request(project_path: Option<PathBuf>) -> InspectXcodeSchemesRequest {
        InspectXcodeSchemesRequest {
            project_path,
            xcode_path: None,
        }
    }

    #[test]
    fn resolve_path_prefers_request_over_cwd_and_config() {
        let base = make_temp_dir("request-priority");
        let cwd_project = base.join("CwdApp.xcodeproj");
        fs::create_dir_all(&cwd_project).expect("cwd xcodeproj should be created");

        let config_project = base.join("ConfigApp.xcodeproj");
        fs::create_dir_all(&config_project).expect("config xcodeproj should be created");

        let request_project = base.join("RequestApp.xcodeproj");
        fs::create_dir_all(&request_project).expect("request xcodeproj should be created");

        let (resolved, source) = resolve_project_path_in_dir(
            &sample_request(Some(request_project.clone())),
            &base,
            Some(config_project.as_path()),
        )
        .expect("path should resolve");
        assert_eq!(resolved, request_project);
        assert!(matches!(source, ProjectPathSource::Request));
    }

    #[test]
    fn resolve_path_uses_cwd_xcodeproj_when_request_absent() {
        let base = make_temp_dir("cwd-priority");
        let cwd_project = base.join("VisionApp.xcodeproj");
        fs::create_dir_all(&cwd_project).expect("cwd xcodeproj should be created");

        let (resolved, source) = resolve_project_path_in_dir(&sample_request(None), &base, None)
            .expect("path should resolve");
        assert_eq!(resolved, cwd_project);
        assert!(matches!(source, ProjectPathSource::Cwd));
    }

    #[test]
    fn resolve_path_falls_back_to_config_default_when_cwd_has_no_xcodeproj() {
        let base = make_temp_dir("config-fallback");
        let external = make_temp_dir("config-external");
        let config_project = external.join("FromConfig.xcodeproj");
        fs::create_dir_all(&config_project).expect("config xcodeproj should be created");

        let (resolved, source) = resolve_project_path_in_dir(
            &sample_request(None),
            &base,
            Some(config_project.as_path()),
        )
        .expect("path should resolve");
        assert_eq!(resolved, config_project);
        assert!(matches!(source, ProjectPathSource::Config));
    }
}
