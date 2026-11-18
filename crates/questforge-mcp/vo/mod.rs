//! Value objects module.
//!
//! ```compile_fail
//! use questforge_mcp::vo::mcp_server::QuestMcpServer;
//! ```

mod mcp_server;
mod xr_agent;

pub use self::mcp_server::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpToolDefinition, QuestMcpServer,
};
pub use self::xr_agent::{
    AgentGoal, AgentOptions, AgentRunReport, AgentStepLog, LlmAgentOptions, LlmRunReport,
    QuestXrAgent,
};
