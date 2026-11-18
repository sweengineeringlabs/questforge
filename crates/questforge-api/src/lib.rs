//! # questforge-api
//!
//! Domain contracts and error definitions for QuestForge.
//!
//! ```compile_fail
//! use questforge_api::builder::QuestforgeBuilder;
//! ```
//!
//! ```compile_fail
//! use questforge_api::error::CrateError;
//! ```
//!
//! ```compile_fail
//! use questforge_api::traits::QuestforgeSvc;
//! ```

#![deny(unsafe_code)]

mod builder;
mod error;
mod traits;

pub use self::builder::{BuilderError, QuestforgeBuilder};
pub use self::error::CrateError;
pub use self::traits::{
    LlmCredentialProvider, LlmEndpointConfig, QuestforgeCapability, QuestforgeSvc,
};
