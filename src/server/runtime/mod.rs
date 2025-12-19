//! MCP server startup and tool registration.
mod server_info;
mod startup;
mod tool_registry;

pub use server_info::build_instructions;
pub use startup::{run_server, RuntimeExit};
pub use tool_registry::HelloWorldServer;
pub use tool_registry::VisionOsServer;
