//! MCP tools for visionOS.
//!
//! Phase 3 introduces the build tool; later phases add sandbox and artifact tools.

pub mod artifacts;
pub mod build;
pub mod diagnostics;
pub mod errors;
pub mod registry;
pub mod sandbox;
pub mod schemes;

pub use artifacts::{
    fetch_build_output, fetch_error_to_error_data, BuildFailureContext, FetchBuildOutputRequest,
    FetchBuildOutputResponse, VisionOsArtifactStore,
};
pub use build::{
    run_build, runtime_error_to_error_data, validation_error_to_error_data,
    BuildRequestValidationError, BuildVisionOsAppResponse, VisionOsBuildRequest, VisionOsJobQueue,
    BUILD_TOOL_ID,
};
pub use diagnostics::{
    inspect_build_diagnostics, BuildFailureSummary, FailureLocation,
    InspectBuildDiagnosticsRequest, InspectBuildDiagnosticsResponse,
};
pub use errors::{
    fetch_error_to_error_data as visionos_fetch_error,
    runtime_error_to_error_data as visionos_runtime_error,
    sandbox_error_to_error_data as visionos_sandbox_error,
    validation_error_to_error_data as visionos_validation_error,
};
pub use registry::VisionOsToolRouter;
pub use sandbox::{
    inspect_xcode_sdks, sandbox_error_to_error_data, validate_sandbox_policy,
    InspectXcodeSdksRequest, InspectXcodeSdksResponse, SandboxPolicyRequest, SandboxPolicyResponse,
};
pub use schemes::{inspect_xcode_schemes, InspectXcodeSchemesRequest, InspectXcodeSchemesResponse};
