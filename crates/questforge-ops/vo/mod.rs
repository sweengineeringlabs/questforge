//! Value objects module.
//!
//! ```compile_fail
//! use questforge_ops::vo::fleet_manager::FleetManager;
//! ```

mod fleet_manager;

pub use self::fleet_manager::{
    AdbOutput, AgentSyncReport, DeploymentRecord, DeviceRecord, FleetInventory, FleetManager,
    HmsReleaseReceipt, HmsReleaseRequest, MdmProvider, MockOpsEnv, OpsEnv, StoreUploadReceipt,
    StoreUploadRequest,
};
