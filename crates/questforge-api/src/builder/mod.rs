//! Domain builder module.
//!
//! ```compile_fail
//! use questforge_api::builder::error::BuilderError;
//! ```
//!
//! ```compile_fail
//! use questforge_api::builder::traits::QuestforgeBuilder;
//! ```

mod error;
mod traits;

pub use self::error::BuilderError;
pub use self::traits::QuestforgeBuilder;
