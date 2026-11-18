//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_mcp::error::crate_error::McpError;
//! ```

mod crate_error;

pub use self::crate_error::McpError;
