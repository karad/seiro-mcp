//! Centralized error-to-ErrorData mapping for visionOS tools.
pub use crate::tools::visionos::artifacts::fetch_error_to_error_data;
pub use crate::tools::visionos::build::{
    runtime_error_to_error_data, validation_error_to_error_data,
};
pub use crate::tools::visionos::sandbox::sandbox_error_to_error_data;
