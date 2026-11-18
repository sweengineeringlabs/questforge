//! Error types for `questforge-config`.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced when loading or validating `questforge.yaml`.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Failed to read manifest or engine project file.
    #[error("failed to read '{path}': {source}")]
    Io {
        /// File path that could not be read.
        path: PathBuf,
        /// Underlying I/O error.
        source: std::io::Error,
    },

    /// YAML syntax or type deserialization error.
    #[error("invalid questforge.yaml syntax: {0}")]
    YamlParse(#[from] serde_yaml::Error),

    /// Unsupported manifest schema version.
    #[error("unsupported manifest version '{version}' (expected '1')")]
    UnsupportedVersion {
        /// Version value found in the manifest.
        version: String,
    },

    /// Field-level validation error.
    #[error("validation failed for '{field}': {message}")]
    Validation {
        /// Dotted field path in `questforge.yaml`.
        field: &'static str,
        /// Explanation of the invalid value.
        message: String,
    },
}

impl From<ConfigError> for questforge_api::CrateError {
    fn from(err: ConfigError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
