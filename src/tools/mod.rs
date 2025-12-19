//! MCP tools registered on the server and helper functions for the router.

pub mod visionos;

use rmcp::handler::server::router::tool::ToolRouter;

pub type ServerToolRouter<S> = ToolRouter<S>;

/// Helper for building a tool router.
pub fn build_router<S>(builder: impl FnOnce() -> ServerToolRouter<S>) -> ServerToolRouter<S>
where
    S: Send + Sync + 'static,
{
    builder()
}
