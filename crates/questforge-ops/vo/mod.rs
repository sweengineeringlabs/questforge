//! Value objects module.
//!
//! ```compile_fail
//! use questforge_ops::vo::fleet_manager::FleetManager;
//! ```

mod fleet_manager;

pub use self::fleet_manager::{
    extract_build_id_from_stdout, save_latest_deployment, AdbOutput, AgentSyncReport,
    ChannelTesterAction, ChannelTesterReceipt, ChannelTesterRequest, DeploymentRecord,
    DeviceRecord, FleetInventory, FleetManager, HmsReleaseReceipt, HmsReleaseRequest, MdmProvider,
    MockOpsEnv, OpsEnv, OvrStoreAction, OvrStoreReceipt, OvrStoreRequest, StoreUploadReceipt,
    StoreUploadRequest,
};
