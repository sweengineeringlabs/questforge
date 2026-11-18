//! Value objects module.
//!
//! ```compile_fail
//! use questforge_config::vo::questforge_config::QuestforgeConfig;
//! ```

mod questforge_config;

pub use self::questforge_config::{
    EngineKind, FoveationLevel, InputMode, MetaVrCliConfig, ProjectConfig, QuestDevice,
    QuestforgeConfig, SdkConfig, SigningConfig, TargetsConfig, ToolchainConfig, XrFeaturesConfig,
};
