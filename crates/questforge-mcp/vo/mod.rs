//! Value objects module.
//!
//! ```compile_fail
//! use questforge_mcp::vo::mcp_server::QuestMcpServer;
//! ```

mod mcp_server;

pub use self::mcp_server::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpToolDefinition, QuestMcpServer,
};
