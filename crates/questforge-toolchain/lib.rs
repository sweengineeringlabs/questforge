//! # questforge-toolchain
//!
//! Quest toolchain provisioning (`adb`, `cmdline-tools`, `ovr-platform-util`).
//!
//! ```compile_fail
//! use questforge_toolchain::error::CrateError;
//! ```
//!
//! ```compile_fail
//! use questforge_toolchain::vo::CrateData;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::CrateError;
pub use self::vo::CrateData;
