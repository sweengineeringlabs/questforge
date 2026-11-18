//! Error types and definitions.
//!
//! ```compile_fail
//! use questforge_doctor::error::crate_error::DoctorError;
//! ```

mod crate_error;

pub use self::crate_error::DoctorError;
