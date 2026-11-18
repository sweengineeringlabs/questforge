//! # questforge-server
//!
//! Hosted control plane REST API, RBAC, audit logging, and metrics for QuestForge.
//!
//! ```compile_fail
//! use questforge_server::error::ServerError;
//! ```
//!
//! ```compile_fail
//! use questforge_server::vo::ControlPlaneServer;
//! ```

#![deny(unsafe_code)]

mod error;
mod vo;

pub use self::error::ServerError;
pub use self::vo::{
    AuditLogEntry, BuildRecord, ControlPlaneServer, ControlPlaneState, HttpRequest, HttpResponse,
    ProjectRecord, Role, VerificationRecord,
};
