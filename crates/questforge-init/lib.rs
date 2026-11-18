//! # questforge-init
//!
//! Quest-ready Unity and Unreal project scaffolding.
//!
//! ```compile_fail
//! use questforge_init::error::CrateError;
//! ```
//!
//! ```compile_fail
//! use questforge_init::vo::CrateData;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::CrateError;
pub use self::vo::CrateData;
