//! # questforge-server
//!
//! Hosted control plane REST API, swappable storage, JWT/RBAC, audit logging, and metrics.
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
    AuditLogEntry, AuthMode, BuildRecord, ControlPlaneServer, ControlPlaneState, HttpRequest,
    HttpResponse, JwtVerifier, PostgresStore, ProjectRecord, Role, StoreBackend,
    VerificationRecord,
};
