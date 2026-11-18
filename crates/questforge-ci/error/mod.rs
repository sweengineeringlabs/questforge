//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_ci::error::crate_error::CiError;
//! ```

mod crate_error;

pub use self::crate_error::CiError;
