//! # questforge-ops
//!
//! Device discovery, lab/fleet inventory, APK rollout/rollback, and telemetry for QuestForge.
//!
//! ```compile_fail
//! use questforge_ops::error::OpsError;
//! ```
//!
//! ```compile_fail
//! use questforge_ops::vo::FleetManager;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::OpsError;
pub use self::vo::{
    extract_build_id_from_stdout, save_latest_deployment, AdbOutput, AgentSyncReport,
    ChannelTesterAction, ChannelTesterReceipt, ChannelTesterRequest, DeploymentRecord,
    DeviceRecord, FleetInventory, FleetManager, HmsReleaseReceipt, HmsReleaseRequest, MdmProvider,
    MockOpsEnv, OpsEnv, OvrStoreAction, OvrStoreReceipt, OvrStoreRequest, StoreUploadReceipt,
    StoreUploadRequest,
};
