//! Value objects module.
//!
//! ```compile_fail
//! use questforge_toolchain::vo::toolchain_spec::ToolchainProvisioner;
//! ```

mod toolchain_spec;

pub use self::toolchain_spec::{
    InstalledTool, PlatformTarget, SetupOptions, SetupReport, ToolArtifactSpec, ToolKind,
    ToolchainProvisioner,
};
