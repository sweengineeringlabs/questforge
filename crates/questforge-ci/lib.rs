//! # questforge-ci
//!
//! Build orchestration, APK metadata, SBOM generation, and CI templates for QuestForge.
//!
//! ```compile_fail
//! use questforge_ci::error::CiError;
//! ```
//!
//! ```compile_fail
//! use questforge_ci::vo::CiOrchestrator;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::CiError;
pub use self::vo::{
    BuildMetadata, CiCmdOutput, CiEnv, CiOptions, CiOrchestrator, CiProvider, CiReport, MockCiEnv,
    SbomComponent, SbomDocument, VulnerabilityFinding,
};
