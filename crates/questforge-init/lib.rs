//! # questforge-init
//!
//! Quest-ready Unity and Unreal project scaffolding.
//!
//! ```compile_fail
//! use questforge_init::error::InitError;
//! ```
//!
//! ```compile_fail
//! use questforge_init::vo::ProjectScaffolder;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::InitError;
pub use self::vo::{
    InitOptions, InitReport, ProjectScaffolder, SourceCheckReport, SourceFinding, SyncReport,
};
