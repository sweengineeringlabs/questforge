//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_api::error::crate_error::CrateError;
//! ```

mod crate_error;

pub use self::crate_error::CrateError;
