//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_config::error::crate_error::ConfigError;
//! ```

mod crate_error;

pub use self::crate_error::ConfigError;
