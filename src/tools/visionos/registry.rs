//! Placeholder for visionOS tool registration. No extra wiring is needed yet.
use rmcp::handler::server::router::tool::ToolRouter;

/// ToolRouter type for visionOS tools.
pub type VisionOsToolRouter<S> = ToolRouter<S>;

/// Hook for future registration extensions (currently no-op).
pub fn register<S>(router: ToolRouter<S>) -> ToolRouter<S> {
    router
}
