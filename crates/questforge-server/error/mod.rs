//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_server::error::crate_error::ServerError;
//! ```

mod crate_error;

pub use self::crate_error::ServerError;
