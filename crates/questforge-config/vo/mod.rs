//! Value objects module.
//!
//! ```compile_fail
//! use questforge_config::vo::questforge_config::QuestforgeConfig;
//! ```

mod questforge_config;

pub use self::questforge_config::{
    EngineKind, FoveationLevel, InputMode, InteractableKind, InteractableSpec, LocomotionMode,
    LocomotionSpec, MdmConfig, MdmTarget, MetaVrCliConfig, PanelActionSpec, ProjectConfig,
    QuestDevice, QuestforgeConfig, SdkConfig, SigningConfig, SpatialPanelSpec, StoreConfig,
    TargetsConfig, ToolchainConfig, XrFeaturesConfig, XrSceneSpec,
};
