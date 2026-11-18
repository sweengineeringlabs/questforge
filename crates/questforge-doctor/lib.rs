//! # questforge-doctor
//!
//! Environment diagnostics (`QD-001` through `QD-009`) for Quest development.
//!
//! ```compile_fail
//! use questforge_doctor::error::CrateError;
//! ```
//!
//! ```compile_fail
//! use questforge_doctor::vo::CrateData;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::CrateError;
pub use self::vo::CrateData;
