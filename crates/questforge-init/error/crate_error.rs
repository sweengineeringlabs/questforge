//! Error types for `questforge-init`.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced when scaffolding a QuestForge project.
#[derive(Debug, Error)]
pub enum InitError {
    /// Target directory already contains files and `--force` was not passed.
    #[error("target directory '{path}' is not empty (pass --force to overwrite)")]
    TargetNotEmpty {
        /// Conflicting target directory.
        path: PathBuf,
    },

    /// Failed to serialize `quest_forge.yaml`.
    #[error("manifest serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Failed to clone an external `--template` Git repository.
    #[error("failed to clone template '{url}': {message}")]
    TemplateClone {
        /// Git template URL.
        url: String,
        /// Error details.
        message: String,
    },

    /// Filesystem I/O failure.
    #[error("I/O error at '{path}': {source}")]
    Io {
        /// Target path.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },
}

impl From<InitError> for questforge_api::CrateError {
    fn from(err: InitError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
