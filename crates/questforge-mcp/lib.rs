//! # questforge-mcp
//!
//! Model Context Protocol (MCP) server exposing QuestForge capabilities to AI agents.
//!
//! ```compile_fail
//! use questforge_mcp::error::McpError;
//! ```
//!
//! ```compile_fail
//! use questforge_mcp::spi::GeminiProvider;
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
mod spi;
mod traits;
mod vo;

pub use self::error::McpError;
pub use self::spi::{AnthropicProvider, AutoLlmCredentialResolver, GeminiProvider, OpenAiProvider};
pub use self::traits::LlmProviderSpi;
pub use self::vo::{
    AgentGoal, AgentOptions, AgentRole, AgentRunReport, AgentStepLog, JsonRpcError, JsonRpcRequest,
    JsonRpcResponse, LlmAgentOptions, LlmChatMessage, LlmEndpointConfig, LlmHttpRequest,
    LlmRunReport, LlmToolInvocation, LlmToolSchema, LlmTurnResponse, McpToolDefinition,
    QuestMcpServer, QuestXrAgent,
};
