//! # questforge-mcp
//!
//! Model Context Protocol (MCP) server exposing QuestForge capabilities to AI agents.
//!
//! ```compile_fail
//! use questforge_mcp::error::McpError;
//! ```
//!
//! ```compile_fail
//! use questforge_mcp::vo::QuestMcpServer;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::McpError;
pub use self::vo::{
    AgentGoal, AgentOptions, AgentRunReport, AgentStepLog, AnthropicCredentialProvider,
    AutoLlmCredentialResolver, GeminiCredentialProvider, JsonRpcError, JsonRpcRequest,
    JsonRpcResponse, LlmAgentOptions, LlmRunReport, McpToolDefinition, OpenAiCredentialProvider,
    QuestMcpServer, QuestXrAgent,
};
