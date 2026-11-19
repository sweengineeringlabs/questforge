//! # questforge-config
//!
//! Shared `quest_forge.yaml` manifest schema, validation, and engine auto-detection.
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
    EngineKind, FoveationLevel, InputMode, InteractableKind, InteractableSpec, LocomotionMode,
    LocomotionSpec, MdmConfig, MdmTarget, MetaVrCliConfig, PanelActionSpec, ProjectConfig,
    QuestDevice, QuestforgeConfig, SdkConfig, SigningConfig, SpatialPanelSpec, StoreConfig,
    TargetsConfig, ToolchainConfig, XrFeaturesConfig, XrSceneSpec,
};
