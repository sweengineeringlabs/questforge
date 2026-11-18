//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_init::error::crate_error::InitError;
//! ```

mod crate_error;

pub use self::crate_error::InitError;
