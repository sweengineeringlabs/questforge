//! # questforge-mcp
//!
//! Model Context Protocol (MCP) server exposing QuestForge capabilities to AI agents.
//!
//! ```compile_fail
//! use questforge_mcp::error::McpError;
//! ```
//!
//! ```compile_fail
//! use questforge_mcp::traits::LlmProviderSpi;
//! ```
//!
//! ```compile_fail
//! use questforge_mcp::vo::QuestMcpServer;
//! ```

#![deny(unsafe_code)]

mod error;
mod traits;
mod vo;

pub use self::error::McpError;
pub use self::traits::LlmProviderSpi;
pub use self::vo::{
    AgentGoal, AgentOptions, AgentRunReport, AgentStepLog, AutoLlmCredentialResolver, JsonRpcError,
    JsonRpcRequest, JsonRpcResponse, LlmAgentOptions, LlmChatMessage, LlmCredentialProvider,
    LlmEndpointConfig, LlmHttpRequest, LlmRunReport, LlmToolInvocation, LlmToolSchema,
    LlmTurnResponse, McpToolDefinition, QuestMcpServer, QuestXrAgent,
};
