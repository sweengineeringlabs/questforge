//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_ops::error::crate_error::OpsError;
//! ```

mod crate_error;

pub use self::crate_error::OpsError;
