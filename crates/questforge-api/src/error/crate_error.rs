//! Error definitions for `questforge-api`.

use thiserror::Error;

/// Domain error for QuestForge contracts.
#[derive(Debug, Error)]
pub enum CrateError {
    /// Internal or unexpected failure.
    #[error("internal error: {message}")]
    Internal {
        /// Human-readable error context.
        message: String,
    },
}
