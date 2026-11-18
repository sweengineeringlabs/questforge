//! Error types for `questforge-ops`.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced by `quest ops` device and fleet management operations.
#[derive(Debug, Error)]
pub enum OpsError {
    /// Device or group was not found in the fleet inventory.
    #[error("unknown device or group: '{target}'")]
    UnknownTarget {
        /// Device serial or group name.
        target: String,
    },

    /// Rollback requested when no previous APK is recorded.
    #[error("no previous APK recorded for rollback on '{serial}'")]
    NoRollbackTarget {
        /// Device serial.
        serial: String,
    },

    /// `adb` command execution failed.
    #[error("adb command failed ({args}): {message}")]
    AdbFailed {
        /// Command arguments.
        args: String,
        /// Error output.
        message: String,
    },

    /// Enterprise MDM / Meta HMS HTTP release failed.
    #[error("MDM publish failed ({provider} @ {endpoint}): {message}")]
    MdmFailed {
        /// Provider identifier (`meta-hms`, `managexr`, `arborxr`).
        provider: String,
        /// Endpoint URL.
        endpoint: String,
        /// Error details.
        message: String,
    },

    /// JSON serialization/deserialization error.
    #[error("inventory JSON error: {0}")]
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

impl From<OpsError> for questforge_api::CrateError {
    fn from(err: OpsError) -> Self {
        Self::Internal {
            message: err.to_string(),
        }
    }
}
