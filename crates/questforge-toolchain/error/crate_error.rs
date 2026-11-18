// Template generated: error/error_file
//! [`CrateError`] -- Errors for `crate`.

use thiserror::Error;

/// Errors arising in internal operations.
#[derive(Debug, Error)]
pub enum CrateError {
    #[error("internal error: {message}")]
    Internal {
        /// What went wrong.
        message: String,
    },
}
