//! # questforge-config
//!
//! Shared `questforge.yaml` manifest schema, validation, and engine auto-detection.
//!
//! ```compile_fail
//! use questforge_config::error::ConfigError;
//! ```
//!
//! ```compile_fail
//! use questforge_config::vo::QuestforgeConfig;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::ConfigError;
pub use self::vo::{
    EngineKind, MetaVrCliConfig, ProjectConfig, QuestDevice, QuestforgeConfig, SdkConfig,
    SigningConfig, TargetsConfig, ToolchainConfig,
};
