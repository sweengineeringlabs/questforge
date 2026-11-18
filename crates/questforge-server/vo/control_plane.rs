//! QuestForge control plane REST API, swappable storage (`InMemory` / `Postgres`), JWT/RBAC, and audit logging.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use postgres::{Client, NoTls};
use questforge_ops::{DeploymentRecord, FleetInventory};
use questforge_verify::VerifyReport;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::ServerError;

type HmacSha256 = Hmac<Sha256>;

/// RBAC role for control plane API access (`SEC-005`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Full administrative privileges.
    Administrator,
    /// Platform & CI/CD management.
    PlatformEngineer,
    /// Project & build creation.
    Developer,
    /// Release verification & QA.
    QaEngineer,
    /// Fleet & deployment operations.
    OperationsEngineer,
    /// Read-only audit & compliance access.
    Auditor,
}

impl Role {
    /// Parses a role from header or JWT claim value.
    #[must_use]
    pub fn parse_header(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "administrator" | "admin" => Some(Self::Administrator),
            "platform_engineer" | "platform" => Some(Self::PlatformEngineer),
            "developer" | "dev" => Some(Self::Developer),
            "qa_engineer" | "qa" => Some(Self::QaEngineer),
            "operations_engineer" | "ops" => Some(Self::OperationsEngineer),
            "auditor" => Some(Self::Auditor),
            _ => None,
        }
    }

    /// Returns true if this role may mutate projects/builds.
    #[must_use]
    pub const fn can_write_projects(self) -> bool {
        matches!(
            self,
            Self::Administrator | Self::PlatformEngineer | Self::Developer
        )
    }

    /// Returns true if this role may trigger deployments.
    #[must_use]
    pub const fn can_deploy(self) -> bool {
        matches!(
            self,
            Self::Administrator | Self::PlatformEngineer | Self::OperationsEngineer
        )
    }

    /// Returns the canonical snake_case role string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Administrator => "administrator",
            Self::PlatformEngineer => "platform_engineer",
            Self::Developer => "developer",
            Self::QaEngineer => "qa_engineer",
            Self::OperationsEngineer => "operations_engineer",
            Self::Auditor => "auditor",
        }
    }
}

/// HS256 JWT verifier for OIDC/OAuth bearer tokens (`SEC-005`).
#[derive(Clone, Debug)]
pub struct JwtVerifier {
    secret: Vec<u8>,
}

impl JwtVerifier {
    /// Creates a new HS256 JWT verifier with the given shared secret.
    #[must_use]
    pub fn new(secret: impl Into<Vec<u8>>) -> Self {
        Self {
            secret: secret.into(),
        }
    }

    /// Signs a compact HS256 JWT containing `sub`, `role`, and optional `exp` (Unix seconds).
    pub fn sign_token(
        &self,
        sub: &str,
        role: Role,
        exp: Option<u64>,
    ) -> Result<String, ServerError> {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let mut claims = json!({
            "sub": sub,
            "role": role.as_str(),
        });
        if let Some(exp_ts) = exp {
            claims["exp"] = json!(exp_ts);
        }
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims)?);
        let signing_input = format!("{header}.{payload}");
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|err| ServerError::Auth(err.to_string()))?;
        mac.update(signing_input.as_bytes());
        let sig = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        Ok(format!("{signing_input}.{sig}"))
    }

    /// Verifies a `Bearer <token>` header value and extracts its `Role`.
    pub fn verify_bearer(&self, auth_header: &str) -> Result<Role, ServerError> {
        let token = auth_header
            .strip_prefix("Bearer ")
            .or_else(|| auth_header.strip_prefix("bearer "))
            .map(str::trim)
            .ok_or_else(|| ServerError::Auth("missing Bearer prefix".to_string()))?;

        let mut parts = token.split('.');
        let (Some(h), Some(p), Some(s), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ServerError::Auth("malformed JWT".to_string()));
        };

        let signing_input = format!("{h}.{p}");
        let sig_bytes = URL_SAFE_NO_PAD
            .decode(s)
            .map_err(|_| ServerError::Auth("invalid base64 signature".to_string()))?;

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .map_err(|err| ServerError::Auth(err.to_string()))?;
        mac.update(signing_input.as_bytes());
        mac.verify_slice(&sig_bytes)
            .map_err(|_| ServerError::Auth("invalid JWT signature".to_string()))?;

        let payload_bytes = URL_SAFE_NO_PAD
            .decode(p)
            .map_err(|_| ServerError::Auth("invalid base64 payload".to_string()))?;
        let claims: Value = serde_json::from_slice(&payload_bytes)?;

        if let Some(exp) = claims.get("exp").and_then(|v| v.as_u64()) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if now > exp {
                return Err(ServerError::Auth("JWT expired".to_string()));
            }
        }

        let role_str = claims
            .get("role")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServerError::Auth("missing 'role' claim".to_string()))?;
        Role::parse_header(role_str)
            .ok_or_else(|| ServerError::Auth(format!("unknown role '{role_str}'")))
    }
}

/// Authentication enforcement mode for the control plane.
#[derive(Clone, Debug, Default)]
pub enum AuthMode {
    /// Local development mode accepting `X-Quest-Role` header.
    #[default]
    DevHeader,
    /// Production mode requiring valid HS256 JWT bearer tokens.
    Jwt(JwtVerifier),
}

/// Registered QuestForge project in the control plane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectRecord {
    /// Unique project ID.
    pub id: String,
    /// Project name.
    pub name: String,
    /// Repository URL.
    pub repository: String,
    /// Owning team or user.
    pub owner: String,
}

/// Recorded CI build in the control plane.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRecord {
    /// Unique build ID.
    pub id: String,
    /// Associated project ID.
    pub project_id: String,
    /// Build status (`queued`, `succeeded`, `failed`).
    pub status: String,
    /// Release version.
    pub version: String,
}

/// Stored verification scorecard record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRecord {
    /// Unique report ID.
    pub id: String,
    /// Score (`0`..=`100`).
    pub score: u8,
    /// Readiness flag.
    pub ready: bool,
    /// Full verification report if provided.
    pub report: Option<VerifyReport>,
}

/// Immutable audit log entry (`SEC-006`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Monotonic event sequence number.
    pub seq: u64,
    /// Actor role.
    pub role: Role,
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Outcome status code.
    pub status_code: u16,
}

/// Thread-safe in-memory control plane state.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ControlPlaneState {
    /// Projects keyed by ID.
    pub projects: BTreeMap<String, ProjectRecord>,
    /// Builds keyed by ID.
    pub builds: BTreeMap<String, BuildRecord>,
    /// Verification reports keyed by ID.
    pub reports: BTreeMap<String, VerificationRecord>,
    /// Fleet inventory & deployments.
    pub fleet: FleetInventory,
    /// Structured audit log.
    pub audit_log: Vec<AuditLogEntry>,
}

/// PostgreSQL outbound `spi` storage adapter.
pub struct PostgresStore {
    client: Mutex<Client>,
}

impl std::fmt::Debug for PostgresStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresStore").finish_non_exhaustive()
    }
}

impl PostgresStore {
    /// Connects to PostgreSQL at `database_url` and ensures required tables exist.
    pub fn connect(database_url: &str) -> Result<Self, ServerError> {
        let mut client = Client::connect(database_url, NoTls)?;
        client.batch_execute(
            "CREATE TABLE IF NOT EXISTS questforge_projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                repository TEXT NOT NULL,
                owner TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS questforge_builds (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                status TEXT NOT NULL,
                version TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS questforge_reports (
                id TEXT PRIMARY KEY,
                score SMALLINT NOT NULL,
                ready BOOLEAN NOT NULL
            );
            CREATE TABLE IF NOT EXISTS questforge_deployments (
                id TEXT PRIMARY KEY,
                target TEXT NOT NULL,
                apk_path TEXT NOT NULL,
                status TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS questforge_audit_log (
                seq BIGSERIAL PRIMARY KEY,
                role TEXT NOT NULL,
                method TEXT NOT NULL,
                path TEXT NOT NULL,
                status_code INTEGER NOT NULL
            );",
        )?;
        Ok(Self {
            client: Mutex::new(client),
        })
    }
}

/// Swappable storage backend (`InMemory` for tests/dev, `Postgres` for production).
#[derive(Clone, Debug)]
pub enum StoreBackend {
    /// In-memory store.
    InMemory(Arc<Mutex<ControlPlaneState>>),
    /// PostgreSQL `spi` adapter.
    Postgres(Arc<PostgresStore>),
}

impl Default for StoreBackend {
    fn default() -> Self {
        Self::InMemory(Arc::new(Mutex::new(ControlPlaneState::default())))
    }
}

/// Incoming HTTP request representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    /// HTTP method (`GET`, `POST`).
    pub method: String,
    /// Request path (without query string).
    pub path: String,
    /// Lowercased header map.
    pub headers: BTreeMap<String, String>,
    /// UTF-8 body.
    pub body: String,
}

/// Outgoing HTTP response representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Content-Type header value.
    pub content_type: &'static str,
    /// Response body bytes.
    pub body: Vec<u8>,
}

/// QuestForge control plane server.
#[derive(Clone, Debug)]
pub struct ControlPlaneServer {
    /// Swappable storage backend.
    pub store: StoreBackend,
    /// Authentication mode (`DevHeader` or `Jwt`).
    pub auth: AuthMode,
    /// Optional web root serving `machineweb` (`mweb`) generated assets.
    pub web_root: Option<PathBuf>,
}

impl Default for ControlPlaneServer {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ControlPlaneServer {
    /// Creates a new control plane server with in-memory storage and dev-header auth.
    #[must_use]
    pub fn new(web_root: Option<PathBuf>) -> Self {
        Self {
            store: StoreBackend::default(),
            auth: AuthMode::default(),
            web_root,
        }
    }

    /// Creates a control plane server with an explicit storage backend and auth mode.
    #[must_use]
    pub fn with_backend(store: StoreBackend, auth: AuthMode, web_root: Option<PathBuf>) -> Self {
        Self {
            store,
            auth,
            web_root,
        }
    }

    /// Binds to `addr` and serves HTTP/1.1 requests in a blocking loop.
    pub fn serve(&self, addr: &str) -> Result<(), ServerError> {
        let listener = TcpListener::bind(addr)?;
        for stream in listener.incoming() {
            let Ok(mut conn) = stream else {
                continue;
            };
            if let Ok(req) = read_http_request(&mut conn) {
                let resp = self.handle_request(&req);
                let _ = write_http_response(&mut conn, &resp);
            }
        }
        Ok(())
    }

    /// Routes and executes a single HTTP request.
    #[must_use]
    pub fn handle_request(&self, req: &HttpRequest) -> HttpResponse {
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/health") => json_response(200, &json!({ "status": "ok" })),
            ("GET", "/") => self.serve_dashboard_index(),
            ("GET", "/metrics") => self.serve_metrics(),
            ("GET", "/api/projects") => self.list_projects(),
            ("POST", "/api/projects") => self.create_project(req),
            ("GET", "/api/builds") => self.list_builds(),
            ("POST", "/api/builds") => self.create_build(req),
            ("GET", "/api/reports") => self.list_reports(),
            ("POST", "/api/verify") => self.create_verify_record(req),
            ("GET", "/api/deployments") => self.list_deployments(),
            ("POST", "/api/deployments") => self.create_deployment(req),
            ("GET", "/api/audit") => self.list_audit(),
            ("GET", path) if path.starts_with("/api/projects/") => {
                let id = path.trim_start_matches("/api/projects/");
                self.get_project(id)
            }
            ("GET", path) if path.starts_with("/api/builds/") => {
                let id = path.trim_start_matches("/api/builds/");
                self.get_build(id)
            }
            ("GET", path) if path.starts_with("/api/reports/") => {
                let id = path.trim_start_matches("/api/reports/");
                self.get_report(id)
            }
            ("GET", path) if path.starts_with("/api/deployments/") => {
                let id = path.trim_start_matches("/api/deployments/");
                self.get_deployment(id)
            }
            _ => json_response(404, &json!({ "error": "not found" })),
        }
    }

    fn authenticate(&self, req: &HttpRequest) -> Result<Role, HttpResponse> {
        match &self.auth {
            AuthMode::DevHeader => req
                .headers
                .get("x-quest-role")
                .and_then(|raw| Role::parse_header(raw))
                .ok_or_else(|| json_response(401, &json!({ "error": "missing X-Quest-Role" }))),
            AuthMode::Jwt(verifier) => {
                let auth_val = req.headers.get("authorization").ok_or_else(|| {
                    json_response(401, &json!({ "error": "missing Authorization header" }))
                })?;
                verifier
                    .verify_bearer(auth_val)
                    .map_err(|err| json_response(401, &json!({ "error": err.to_string() })))
            }
        }
    }

    fn serve_dashboard_index(&self) -> HttpResponse {
        if let Some(root) = &self.web_root {
            let index = root.join("index.html");
            if let Ok(html) = std::fs::read(&index) {
                return HttpResponse {
                    status: 200,
                    content_type: "text/html; charset=utf-8",
                    body: html,
                };
            }
        }
        HttpResponse {
            status: 200,
            content_type: "text/html; charset=utf-8",
            body: FALLBACK_DASHBOARD_HTML.as_bytes().to_vec(),
        }
    }

    fn serve_metrics(&self) -> HttpResponse {
        let (p, b, r, d, a) = match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                (
                    guard.projects.len(),
                    guard.builds.len(),
                    guard.reports.len(),
                    guard.fleet.deployments.len(),
                    guard.audit_log.len(),
                )
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                let count = |c: &mut Client, tbl: &str| -> usize {
                    c.query_one(&format!("SELECT COUNT(*) FROM {tbl}"), &[])
                        .ok()
                        .and_then(|row| row.get::<_, i64>(0).try_into().ok())
                        .unwrap_or(0)
                };
                (
                    count(&mut client, "questforge_projects"),
                    count(&mut client, "questforge_builds"),
                    count(&mut client, "questforge_reports"),
                    count(&mut client, "questforge_deployments"),
                    count(&mut client, "questforge_audit_log"),
                )
            }
        };
        let body = format!(
            "questforge_projects_total {p}\nquestforge_builds_total {b}\nquestforge_reports_total {r}\nquestforge_deployments_total {d}\nquestforge_audit_events_total {a}\n"
        );
        HttpResponse {
            status: 200,
            content_type: "text/plain; version=0.0.4",
            body: body.into_bytes(),
        }
    }

    fn list_projects(&self) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let list: Vec<&ProjectRecord> = guard.projects.values().collect();
                json_response(200, &json!({ "projects": list }))
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query(
                    "SELECT id, name, repository, owner FROM questforge_projects ORDER BY id",
                    &[],
                ) {
                    Ok(rows) => {
                        let list: Vec<ProjectRecord> = rows
                            .into_iter()
                            .map(|row| ProjectRecord {
                                id: row.get(0),
                                name: row.get(1),
                                repository: row.get(2),
                                owner: row.get(3),
                            })
                            .collect();
                        json_response(200, &json!({ "projects": list }))
                    }
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn get_project(&self, id: &str) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                match guard.projects.get(id) {
                    Some(p) => json_response(200, &json!(p)),
                    None => json_response(404, &json!({ "error": "project not found" })),
                }
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query_opt(
                    "SELECT id, name, repository, owner FROM questforge_projects WHERE id = $1",
                    &[&id],
                ) {
                    Ok(Some(row)) => json_response(
                        200,
                        &json!(ProjectRecord {
                            id: row.get(0),
                            name: row.get(1),
                            repository: row.get(2),
                            owner: row.get(3),
                        }),
                    ),
                    Ok(None) => json_response(404, &json!({ "error": "project not found" })),
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn create_project(&self, req: &HttpRequest) -> HttpResponse {
        let role = match self.authenticate(req) {
            Ok(r) => r,
            Err(resp) => return resp,
        };
        if !role.can_write_projects() {
            self.record_audit(role, "POST", &req.path, 403);
            return json_response(403, &json!({ "error": "forbidden" }));
        }
        let Ok(payload) = serde_json::from_str::<Value>(&req.body) else {
            return json_response(400, &json!({ "error": "invalid JSON body" }));
        };
        let name = payload
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("quest-app")
            .to_string();
        let repository = payload
            .get("repository")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let owner = payload
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or("platform")
            .to_string();

        let rec = match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(mut guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let id = format!("proj-{}", guard.projects.len() + 1);
                let rec = ProjectRecord {
                    id: id.clone(),
                    name,
                    repository,
                    owner,
                };
                guard.projects.insert(id, rec.clone());
                rec
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                let count: i64 = client
                    .query_one("SELECT COUNT(*) FROM questforge_projects", &[])
                    .map(|r| r.get(0))
                    .unwrap_or(0);
                let id = format!("proj-{}", count + 1);
                if let Err(err) = client.execute(
                    "INSERT INTO questforge_projects (id, name, repository, owner) VALUES ($1, $2, $3, $4)",
                    &[&id, &name, &repository, &owner],
                ) {
                    return json_response(500, &json!({ "error": err.to_string() }));
                }
                ProjectRecord {
                    id,
                    name,
                    repository,
                    owner,
                }
            }
        };
        self.record_audit(role, "POST", &req.path, 201);
        json_response(201, &json!(rec))
    }

    fn list_builds(&self) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let list: Vec<&BuildRecord> = guard.builds.values().collect();
                json_response(200, &json!({ "builds": list }))
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query(
                    "SELECT id, project_id, status, version FROM questforge_builds ORDER BY id",
                    &[],
                ) {
                    Ok(rows) => {
                        let list: Vec<BuildRecord> = rows
                            .into_iter()
                            .map(|row| BuildRecord {
                                id: row.get(0),
                                project_id: row.get(1),
                                status: row.get(2),
                                version: row.get(3),
                            })
                            .collect();
                        json_response(200, &json!({ "builds": list }))
                    }
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn create_build(&self, req: &HttpRequest) -> HttpResponse {
        let role = match self.authenticate(req) {
            Ok(r) => r,
            Err(resp) => return resp,
        };
        if !role.can_write_projects() {
            self.record_audit(role, "POST", &req.path, 403);
            return json_response(403, &json!({ "error": "forbidden" }));
        }
        let Ok(payload) = serde_json::from_str::<Value>(&req.body) else {
            return json_response(400, &json!({ "error": "invalid JSON body" }));
        };
        let project_id = payload
            .get("project_id")
            .and_then(|v| v.as_str())
            .unwrap_or("proj-1")
            .to_string();
        let version = payload
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.1.0")
            .to_string();

        let rec = match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(mut guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let id = format!("build-{}", guard.builds.len() + 1);
                let rec = BuildRecord {
                    id: id.clone(),
                    project_id,
                    status: "queued".to_string(),
                    version,
                };
                guard.builds.insert(id, rec.clone());
                rec
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                let count: i64 = client
                    .query_one("SELECT COUNT(*) FROM questforge_builds", &[])
                    .map(|r| r.get(0))
                    .unwrap_or(0);
                let id = format!("build-{}", count + 1);
                if let Err(err) = client.execute(
                    "INSERT INTO questforge_builds (id, project_id, status, version) VALUES ($1, $2, 'queued', $3)",
                    &[&id, &project_id, &version],
                ) {
                    return json_response(500, &json!({ "error": err.to_string() }));
                }
                BuildRecord {
                    id,
                    project_id,
                    status: "queued".to_string(),
                    version,
                }
            }
        };
        self.record_audit(role, "POST", &req.path, 201);
        json_response(201, &json!(rec))
    }

    fn get_build(&self, id: &str) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                match guard.builds.get(id) {
                    Some(b) => json_response(200, &json!(b)),
                    None => json_response(404, &json!({ "error": "build not found" })),
                }
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query_opt(
                    "SELECT id, project_id, status, version FROM questforge_builds WHERE id = $1",
                    &[&id],
                ) {
                    Ok(Some(row)) => json_response(
                        200,
                        &json!(BuildRecord {
                            id: row.get(0),
                            project_id: row.get(1),
                            status: row.get(2),
                            version: row.get(3),
                        }),
                    ),
                    Ok(None) => json_response(404, &json!({ "error": "build not found" })),
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn list_reports(&self) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let list: Vec<&VerificationRecord> = guard.reports.values().collect();
                json_response(200, &json!({ "reports": list }))
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query(
                    "SELECT id, score, ready FROM questforge_reports ORDER BY id",
                    &[],
                ) {
                    Ok(rows) => {
                        let list: Vec<VerificationRecord> = rows
                            .into_iter()
                            .map(|row| VerificationRecord {
                                id: row.get(0),
                                score: row.get::<_, i16>(1).clamp(0, 100) as u8,
                                ready: row.get(2),
                                report: None,
                            })
                            .collect();
                        json_response(200, &json!({ "reports": list }))
                    }
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn create_verify_record(&self, req: &HttpRequest) -> HttpResponse {
        let role = match self.authenticate(req) {
            Ok(r) => r,
            Err(resp) => return resp,
        };
        let Ok(payload) = serde_json::from_str::<Value>(&req.body) else {
            return json_response(400, &json!({ "error": "invalid JSON body" }));
        };
        let score = payload.get("score").and_then(|v| v.as_u64()).unwrap_or(100) as u8;
        let ready = payload
            .get("ready")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let rec = match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(mut guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let id = format!("rep-{}", guard.reports.len() + 1);
                let rec = VerificationRecord {
                    id: id.clone(),
                    score,
                    ready,
                    report: None,
                };
                guard.reports.insert(id, rec.clone());
                rec
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                let count: i64 = client
                    .query_one("SELECT COUNT(*) FROM questforge_reports", &[])
                    .map(|r| r.get(0))
                    .unwrap_or(0);
                let id = format!("rep-{}", count + 1);
                let score_i16 = i16::from(score);
                if let Err(err) = client.execute(
                    "INSERT INTO questforge_reports (id, score, ready) VALUES ($1, $2, $3)",
                    &[&id, &score_i16, &ready],
                ) {
                    return json_response(500, &json!({ "error": err.to_string() }));
                }
                VerificationRecord {
                    id,
                    score,
                    ready,
                    report: None,
                }
            }
        };
        self.record_audit(role, "POST", &req.path, 201);
        json_response(201, &json!(rec))
    }

    fn get_report(&self, id: &str) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                match guard.reports.get(id) {
                    Some(r) => json_response(200, &json!(r)),
                    None => json_response(404, &json!({ "error": "report not found" })),
                }
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query_opt(
                    "SELECT id, score, ready FROM questforge_reports WHERE id = $1",
                    &[&id],
                ) {
                    Ok(Some(row)) => json_response(
                        200,
                        &json!(VerificationRecord {
                            id: row.get(0),
                            score: row.get::<_, i16>(1).clamp(0, 100) as u8,
                            ready: row.get(2),
                            report: None,
                        }),
                    ),
                    Ok(None) => json_response(404, &json!({ "error": "report not found" })),
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn list_deployments(&self) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                json_response(200, &json!({ "deployments": guard.fleet.deployments }))
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query(
                    "SELECT target, apk_path, status FROM questforge_deployments ORDER BY id",
                    &[],
                ) {
                    Ok(rows) => {
                        let list: Vec<DeploymentRecord> = rows
                            .into_iter()
                            .map(|row| DeploymentRecord {
                                target: row.get(0),
                                apk_path: PathBuf::from(row.get::<_, String>(1)),
                                scheduled_at: None,
                                status: row.get(2),
                                devices: Vec::new(),
                            })
                            .collect();
                        json_response(200, &json!({ "deployments": list }))
                    }
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn create_deployment(&self, req: &HttpRequest) -> HttpResponse {
        let role = match self.authenticate(req) {
            Ok(r) => r,
            Err(resp) => return resp,
        };
        if !role.can_deploy() {
            self.record_audit(role, "POST", &req.path, 403);
            return json_response(403, &json!({ "error": "forbidden" }));
        }
        let Ok(payload) = serde_json::from_str::<Value>(&req.body) else {
            return json_response(400, &json!({ "error": "invalid JSON body" }));
        };
        let target = payload
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("canary")
            .to_string();
        let apk_path = payload
            .get("apk_path")
            .and_then(|v| v.as_str())
            .unwrap_or("app.apk")
            .to_string();

        let (id, rec) = match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(mut guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let idx = guard.fleet.deployments.len() + 1;
                let rec = DeploymentRecord {
                    target,
                    apk_path: PathBuf::from(apk_path),
                    scheduled_at: None,
                    status: "scheduled".to_string(),
                    devices: Vec::new(),
                };
                guard.fleet.deployments.push(rec.clone());
                (format!("dep-{idx}"), rec)
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                let count: i64 = client
                    .query_one("SELECT COUNT(*) FROM questforge_deployments", &[])
                    .map(|r| r.get(0))
                    .unwrap_or(0);
                let id = format!("dep-{}", count + 1);
                if let Err(err) = client.execute(
                    "INSERT INTO questforge_deployments (id, target, apk_path, status) VALUES ($1, $2, $3, 'scheduled')",
                    &[&id, &target, &apk_path],
                ) {
                    return json_response(500, &json!({ "error": err.to_string() }));
                }
                (
                    id,
                    DeploymentRecord {
                        target,
                        apk_path: PathBuf::from(apk_path),
                        scheduled_at: None,
                        status: "scheduled".to_string(),
                        devices: Vec::new(),
                    },
                )
            }
        };
        self.record_audit(role, "POST", &req.path, 201);
        json_response(201, &json!({ "id": id, "deployment": rec }))
    }

    fn get_deployment(&self, id: &str) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                let parsed = id
                    .strip_prefix("dep-")
                    .and_then(|n| n.parse::<usize>().ok())
                    .and_then(|i| i.checked_sub(1))
                    .and_then(|i| guard.fleet.deployments.get(i));
                match parsed {
                    Some(d) => json_response(200, &json!(d)),
                    None => json_response(404, &json!({ "error": "deployment not found" })),
                }
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query_opt(
                    "SELECT target, apk_path, status FROM questforge_deployments WHERE id = $1",
                    &[&id],
                ) {
                    Ok(Some(row)) => json_response(
                        200,
                        &json!(DeploymentRecord {
                            target: row.get(0),
                            apk_path: PathBuf::from(row.get::<_, String>(1)),
                            scheduled_at: None,
                            status: row.get(2),
                            devices: Vec::new(),
                        }),
                    ),
                    Ok(None) => json_response(404, &json!({ "error": "deployment not found" })),
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn list_audit(&self) -> HttpResponse {
        match &self.store {
            StoreBackend::InMemory(state) => {
                let Ok(guard) = state.lock() else {
                    return json_response(500, &json!({ "error": "state lock poisoned" }));
                };
                json_response(200, &json!({ "audit_log": guard.audit_log }))
            }
            StoreBackend::Postgres(pg) => {
                let Ok(mut client) = pg.client.lock() else {
                    return json_response(500, &json!({ "error": "db lock poisoned" }));
                };
                match client.query(
                    "SELECT seq, role, method, path, status_code FROM questforge_audit_log ORDER BY seq",
                    &[],
                ) {
                    Ok(rows) => {
                        let list: Vec<AuditLogEntry> = rows
                            .into_iter()
                            .map(|row| AuditLogEntry {
                                seq: row.get::<_, i64>(0).max(0) as u64,
                                role: Role::parse_header(&row.get::<_, String>(1))
                                    .unwrap_or(Role::Auditor),
                                method: row.get(2),
                                path: row.get(3),
                                status_code: row.get::<_, i32>(4).clamp(0, 999) as u16,
                            })
                            .collect();
                        json_response(200, &json!({ "audit_log": list }))
                    }
                    Err(err) => json_response(500, &json!({ "error": err.to_string() })),
                }
            }
        }
    }

    fn record_audit(&self, role: Role, method: &str, path: &str, status_code: u16) {
        match &self.store {
            StoreBackend::InMemory(state) => {
                if let Ok(mut guard) = state.lock() {
                    let seq = guard.audit_log.len() as u64 + 1;
                    guard.audit_log.push(AuditLogEntry {
                        seq,
                        role,
                        method: method.to_string(),
                        path: path.to_string(),
                        status_code,
                    });
                }
            }
            StoreBackend::Postgres(pg) => {
                if let Ok(mut client) = pg.client.lock() {
                    let _ = client.execute(
                        "INSERT INTO questforge_audit_log (role, method, path, status_code) VALUES ($1, $2, $3, $4)",
                        &[&role.as_str(), &method, &path, &i32::from(status_code)],
                    );
                }
            }
        }
    }
}

fn json_response(status: u16, payload: &Value) -> HttpResponse {
    let body = serde_json::to_vec_pretty(payload).unwrap_or_else(|_| b"{}".to_vec());
    HttpResponse {
        status,
        content_type: "application/json",
        body,
    }
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, ServerError> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET").to_string();
    let raw_target = parts.next().unwrap_or("/");
    let path = raw_target.split('?').next().unwrap_or("/").to_string();

    let mut headers = BTreeMap::new();
    let mut content_length = 0_usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line)?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim().to_string();
            if key == "content-length" {
                content_length = val.parse::<usize>().unwrap_or(0);
            }
            headers.insert(key, val);
        }
    }

    let mut buf = vec![0_u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut buf)?;
    }

    Ok(HttpRequest {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&buf).to_string(),
    })
}

fn write_http_response(stream: &mut TcpStream, resp: &HttpResponse) -> Result<(), ServerError> {
    let reason = match resp.status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        resp.status,
        reason,
        resp.content_type,
        resp.body.len()
    )?;
    stream.write_all(&resp.body)?;
    stream.flush()?;
    Ok(())
}

const FALLBACK_DASHBOARD_HTML: &str = "\
<!doctype html>
<html lang=\"en\">
<head><meta charset=\"utf-8\"><title>QuestForge Control Plane</title></head>
<body>
<h1>QuestForge Control Plane</h1>
<p>Powered by machineweb (mweb) components and QuestForge REST API.</p>
</body>
</html>
";
