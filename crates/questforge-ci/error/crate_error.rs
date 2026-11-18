//! Error types for `questforge-ci`.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced by `quest ci` build, packaging, SBOM, or upload steps.
#[derive(Debug, Error)]
pub enum CiError {
    /// Failed to load `quest_forge.yaml`.
    #[error("manifest error: {0}")]
    Config(#[from] questforge_config::ConfigError),

    /// Build or upload command failed with structured diagnostics.
    #[error("step '{step}' failed (exit {exit_code:?}): {summary}")]
    StepFailed {
        /// Pipeline step name (`compile`, `sign`, `scan`, `upload`).
        step: String,
        /// Process exit code if available.
        exit_code: Option<i32>,
        /// Extracted diagnostic summary.
        summary: String,
        /// Raw stderr/stdout tail for troubleshooting.
        log_excerpt: String,
    },

    /// Missing required output artifact (e.g., APK).
    #[error("expected build artifact missing at '{path}'")]
    MissingArtifact {
        /// Expected artifact path.
        path: PathBuf,
    },

    /// Serialization error for metadata or SBOM.
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

impl From<CiError> for questforge_api::CrateError {
    fn from(err: CiError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
