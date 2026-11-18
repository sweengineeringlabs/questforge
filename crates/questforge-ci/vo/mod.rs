//! Value objects module.
//!
//! ```compile_fail
//! use questforge_ci::vo::ci_pipeline::CiOrchestrator;
//! ```

mod ci_pipeline;

pub use self::ci_pipeline::{
    BuildMetadata, CiCmdOutput, CiEnv, CiOptions, CiOrchestrator, CiProvider, CiReport, MockCiEnv,
    SbomComponent, SbomDocument, VulnerabilityFinding,
};
