use schemars::JsonSchema;
use serde::Deserialize;

/// Input for `inspect_build_diagnostics`.
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InspectBuildDiagnosticsRequest {
    pub job_id: String,
    #[serde(default = "default_include_log_excerpt")]
    pub include_log_excerpt: bool,
    #[serde(default = "default_prefer_typecheck")]
    pub prefer_typecheck: bool,
}

fn default_include_log_excerpt() -> bool {
    true
}

fn default_prefer_typecheck() -> bool {
    true
}
