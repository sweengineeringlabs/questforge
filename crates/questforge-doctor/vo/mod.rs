//! Value objects module.
//!
//! ```compile_fail
//! use questforge_doctor::vo::doctor_report::DoctorRunner;
//! ```

mod doctor_report;

pub use self::doctor_report::{
    CheckResult, CheckStatus, CmdOutput, DoctorEnv, DoctorOptions, DoctorReport, DoctorRunner,
    MockDoctorEnv,
};
