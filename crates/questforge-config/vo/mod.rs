//! Value objects module.
//!
//! ```compile_fail
//! use questforge_config::vo::questforge_config::QuestforgeConfig;
//! ```

mod questforge_config;

pub use self::questforge_config::{
    EngineKind, FoveationLevel, InputMode, InteractableKind, InteractableSpec, LocomotionMode,
    LocomotionSpec, MetaVrCliConfig, PanelActionSpec, ProjectConfig, QuestDevice, QuestforgeConfig,
    SdkConfig, SigningConfig, SpatialPanelSpec, TargetsConfig, ToolchainConfig, XrFeaturesConfig,
    XrSceneSpec,
};
