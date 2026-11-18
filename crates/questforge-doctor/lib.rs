//! # questforge-doctor
//!
//! Environment diagnostics (`QD-001` through `QD-009`) for Quest development.
//!
//! ```compile_fail
//! use questforge_doctor::error::DoctorError;
//! ```
//!
//! ```compile_fail
//! use questforge_doctor::vo::DoctorRunner;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::DoctorError;
pub use self::vo::{
    CheckResult, CheckStatus, CmdOutput, DoctorEnv, DoctorOptions, DoctorReport, DoctorRunner,
    MockDoctorEnv,
};
