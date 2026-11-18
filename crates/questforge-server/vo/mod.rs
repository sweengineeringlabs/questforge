//! Value objects module.
//!
//! ```compile_fail
//! use questforge_server::vo::control_plane::ControlPlaneServer;
//! ```

mod control_plane;

pub use self::control_plane::{
    AuditLogEntry, BuildRecord, ControlPlaneServer, ControlPlaneState, HttpRequest, HttpResponse,
    ProjectRecord, Role, VerificationRecord,
};
