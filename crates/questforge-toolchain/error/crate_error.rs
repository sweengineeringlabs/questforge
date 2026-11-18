//! Error types for `questforge-toolchain`.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced while resolving, downloading, verifying, or caching Quest toolchains.
#[derive(Debug, Error)]
pub enum ToolchainError {
    /// Failed to load `quest_forge.yaml`.
    #[error("failed to load quest_forge.yaml: {0}")]
    Config(#[from] questforge_config::ConfigError),

    /// Unsupported host OS or CPU architecture.
    #[error("unsupported host platform: {os}-{arch}")]
    UnsupportedPlatform {
        /// Operating system identifier.
        os: String,
        /// CPU architecture identifier.
        arch: String,
    },

    /// Missing download URL or checksum for a requested tool.
    #[error("missing download specification for {tool} {version}: {reason}")]
    MissingSpec {
        /// Tool name.
        tool: String,
        /// Requested version.
        version: String,
        /// Explanation of what is missing.
        reason: String,
    },

    /// HTTP download failure.
    #[error("failed to download {url}: {message}")]
    Download {
        /// Source URL.
        url: String,
        /// Error details.
        message: String,
    },

    /// SHA-256 integrity verification failed.
    #[error("checksum mismatch for {tool} {version}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// Tool name.
        tool: String,
        /// Tool version.
        version: String,
        /// Expected hex SHA-256 digest.
        expected: String,
        /// Actual hex SHA-256 digest.
        actual: String,
    },

    /// Archive extraction failure.
    #[error("failed to extract archive '{path}': {message}")]
    Extract {
        /// Archive or target path.
        path: PathBuf,
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

impl From<ToolchainError> for questforge_api::CrateError {
    fn from(err: ToolchainError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
