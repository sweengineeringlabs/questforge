//! Error types for `questforge-mcp`.

use thiserror::Error;

/// Errors produced by the QuestForge MCP server.
#[derive(Debug, Error)]
pub enum McpError {
    /// JSON-RPC parsing or serialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Standard I/O failure on stdio transport.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<McpError> for questforge_api::CrateError {
    fn from(err: McpError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
