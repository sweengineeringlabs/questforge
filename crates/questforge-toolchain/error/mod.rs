//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_toolchain::error::crate_error::ToolchainError;
//! ```

mod crate_error;

pub use self::crate_error::ToolchainError;
