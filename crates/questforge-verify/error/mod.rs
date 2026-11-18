//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_verify::error::crate_error::VerifyError;
//! ```

mod crate_error;

pub use self::crate_error::VerifyError;
