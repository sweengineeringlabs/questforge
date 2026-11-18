//! # questforge-config
//!
//! Shared configuration schema and manifest definitions for QuestForge.
//!
//! ```compile_fail
//! use questforge_config::error::CrateError;
//! ```
//!
//! ```compile_fail
//! use questforge_config::vo::CrateData;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::CrateError;
pub use self::vo::CrateData;
