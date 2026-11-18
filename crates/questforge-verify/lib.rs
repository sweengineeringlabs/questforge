//! # questforge-verify
//!
//! Static APK release-readiness, signature, ABI, and Meta VRC compliance verification.
//!
//! ```compile_fail
//! use questforge_verify::error::VerifyError;
//! ```
//!
//! ```compile_fail
//! use questforge_verify::vo::ApkVerifier;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::VerifyError;
pub use self::vo::{
    ApkVerifier, PolicyFinding, ReportFormat, RuleStatus, VerifyOptions, VerifyReport,
};
