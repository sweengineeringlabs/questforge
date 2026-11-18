//! Error types for `questforge-server`.

use thiserror::Error;

/// Errors produced by the QuestForge control plane server.
#[derive(Debug, Error)]
pub enum ServerError {
    /// Failed to bind or accept TCP connections.
    #[error("network I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Verification engine error.
    #[error("verify error: {0}")]
    Verify(#[from] questforge_verify::VerifyError),
}

impl From<ServerError> for questforge_api::CrateError {
    fn from(err: ServerError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
