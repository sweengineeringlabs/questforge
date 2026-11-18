//! # questforge-toolchain
//!
//! Quest toolchain provisioning (`adb`, `cmdline-tools`, `ovr-platform-util`).
//!
//! ```compile_fail
//! use questforge_toolchain::error::ToolchainError;
//! ```
//!
//! ```compile_fail
//! use questforge_toolchain::vo::ToolchainProvisioner;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::ToolchainError;
pub use self::vo::{
    InstalledTool, PlatformTarget, SetupOptions, SetupReport, ToolArtifactSpec, ToolKind,
    ToolchainProvisioner,
};
