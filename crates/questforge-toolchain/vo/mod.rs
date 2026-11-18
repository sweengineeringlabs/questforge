//! Value objects module.
//!
//! ```compile_fail
//! use questforge_toolchain::vo::crate_data::CrateData;
//! ```

mod crate_data;

pub use self::crate_data::CrateData;
