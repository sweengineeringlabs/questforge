//! Value objects module.
//!
//! ```compile_fail
//! use questforge_init::vo::init_plan::ProjectScaffolder;
//! ```

mod init_plan;

pub use self::init_plan::{InitOptions, InitReport, ProjectScaffolder};
