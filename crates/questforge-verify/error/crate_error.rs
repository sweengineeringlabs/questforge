//! Error types for `questforge-verify`.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced when inspecting or reporting on an APK release artifact.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Failed to load `quest_forge.yaml`.
    #[error("manifest error: {0}")]
    Config(#[from] questforge_config::ConfigError),

    /// Invalid or unreadable APK archive.
    #[error("invalid APK '{path}': {message}")]
    InvalidApk {
        /// APK path.
        path: PathBuf,
        /// Failure details.
        message: String,
    },

    /// JSON/SARIF serialization failure.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// Filesystem I/O failure.
    #[error("I/O error at '{path}': {source}")]
    Io {
        /// Target path.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

impl From<VerifyError> for questforge_api::CrateError {
    fn from(err: VerifyError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
