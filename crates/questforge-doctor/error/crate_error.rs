//! Error types for `questforge-doctor`.

use thiserror::Error;

/// Fatal errors preventing `quest doctor` from executing diagnostics.
#[derive(Debug, Error)]
pub enum DoctorError {
    /// Failed to load or validate `quest_forge.yaml`.
    #[error("manifest error: {0}")]
    Config(#[from] questforge_config::ConfigError),

    /// Failed to serialize JSON report.
    #[error("serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<DoctorError> for questforge_api::CrateError {
    fn from(err: DoctorError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
