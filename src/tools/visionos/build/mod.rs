//! visionOS build tool entrypoint.
pub mod executor;
pub mod queue;
pub mod request;

pub use executor::{
    run_build, runtime_error_to_error_data, validation_error_to_error_data,
    BuildVisionOsAppResponse,
};
pub use queue::{JobTicket, VisionOsJobQueue};
pub use request::{
    default_destination, BuildConfiguration, BuildRequestValidationError, VisionOsBuildRequest,
    ALLOWED_ENV_OVERRIDES, ALLOWED_EXTRA_ARGS,
};

pub const BUILD_TOOL_ID: &str = "build_visionos_app";
