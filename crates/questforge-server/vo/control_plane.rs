//! QuestForge control plane REST API, RBAC, audit logging, and HTTP server.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use questforge_ops::{DeploymentRecord, FleetInventory};
use questforge_verify::VerifyReport;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ServerError;

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
    /// Parses a role from header value.
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

/// Thread-safe in-memory control plane state (persisted optionally to JSON).
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
    /// Shared state.
    pub state: Arc<Mutex<ControlPlaneState>>,
    /// Optional web root serving `machineweb` (`mweb`) generated assets.
    pub web_root: Option<PathBuf>,
}

impl Default for ControlPlaneServer {
    fn default() -> Self {
        Self::new(None)
    }
}

impl ControlPlaneServer {
    /// Creates a new control plane server.
    #[must_use]
    pub fn new(web_root: Option<PathBuf>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ControlPlaneState::default())),
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

    /// Routes and executes a single HTTP request (used by `serve` and integration tests).
    #[must_use]
    pub fn handle_request(&self, req: &HttpRequest) -> HttpResponse {
        match (req.method.as_str(), req.path.as_str()) {
            ("GET", "/health") => json_response(200, &json!({ "status": "ok" })),
            ("GET", "/") => self.serve_dashboard_index(),
            ("GET", "/metrics") => self.serve_metrics(),
            ("GET", "/api/projects") => self.list_projects(),
            ("POST", "/api/projects") => self.create_project(req),
            ("GET", "/api/audit") => self.list_audit(),
            ("POST", "/api/builds") => self.create_build(req),
            ("POST", "/api/verify") => self.create_verify_record(req),
            ("POST", "/api/deployments") => self.create_deployment(req),
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
                let idx = path.trim_start_matches("/api/deployments/");
                self.get_deployment(idx)
            }
            _ => json_response(404, &json!({ "error": "not found" })),
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
        let Ok(guard) = self.state.lock() else {
            return json_response(500, &json!({ "error": "state lock poisoned" }));
        };
        let body = format!(
            "questforge_projects_total {}\nquestforge_builds_total {}\nquestforge_reports_total {}\nquestforge_deployments_total {}\nquestforge_audit_events_total {}\n",
            guard.projects.len(),
            guard.builds.len(),
            guard.reports.len(),
            guard.fleet.deployments.len(),
            guard.audit_log.len()
        );
        HttpResponse {
            status: 200,
            content_type: "text/plain; version=0.0.4",
            body: body.into_bytes(),
        }
    }

    fn list_projects(&self) -> HttpResponse {
        let Ok(guard) = self.state.lock() else {
            return json_response(500, &json!({ "error": "state lock poisoned" }));
        };
        let list: Vec<&ProjectRecord> = guard.projects.values().collect();
        json_response(200, &json!({ "projects": list }))
    }

    fn get_project(&self, id: &str) -> HttpResponse {
        let Ok(guard) = self.state.lock() else {
            return json_response(500, &json!({ "error": "state lock poisoned" }));
        };
        match guard.projects.get(id) {
            Some(p) => json_response(200, &json!(p)),
            None => json_response(404, &json!({ "error": "project not found" })),
        }
    }

    fn create_project(&self, req: &HttpRequest) -> HttpResponse {
        let Some(role) = extract_role(req) else {
            return json_response(401, &json!({ "error": "missing or invalid X-Quest-Role" }));
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

        let Ok(mut guard) = self.state.lock() else {
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
        let seq = guard.audit_log.len() as u64 + 1;
        guard.audit_log.push(AuditLogEntry {
            seq,
            role,
            method: "POST".to_string(),
            path: req.path.clone(),
            status_code: 201,
        });
        json_response(201, &json!(rec))
    }

    fn create_build(&self, req: &HttpRequest) -> HttpResponse {
        let Some(role) = extract_role(req) else {
            return json_response(401, &json!({ "error": "missing or invalid X-Quest-Role" }));
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

        let Ok(mut guard) = self.state.lock() else {
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
        let seq = guard.audit_log.len() as u64 + 1;
        guard.audit_log.push(AuditLogEntry {
            seq,
            role,
            method: "POST".to_string(),
            path: req.path.clone(),
            status_code: 201,
        });
        json_response(201, &json!(rec))
    }

    fn get_build(&self, id: &str) -> HttpResponse {
        let Ok(guard) = self.state.lock() else {
            return json_response(500, &json!({ "error": "state lock poisoned" }));
        };
        match guard.builds.get(id) {
            Some(b) => json_response(200, &json!(b)),
            None => json_response(404, &json!({ "error": "build not found" })),
        }
    }

    fn create_verify_record(&self, req: &HttpRequest) -> HttpResponse {
        let Some(role) = extract_role(req) else {
            return json_response(401, &json!({ "error": "missing or invalid X-Quest-Role" }));
        };
        let Ok(payload) = serde_json::from_str::<Value>(&req.body) else {
            return json_response(400, &json!({ "error": "invalid JSON body" }));
        };
        let score = payload.get("score").and_then(|v| v.as_u64()).unwrap_or(100) as u8;
        let ready = payload
            .get("ready")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let Ok(mut guard) = self.state.lock() else {
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
        let seq = guard.audit_log.len() as u64 + 1;
        guard.audit_log.push(AuditLogEntry {
            seq,
            role,
            method: "POST".to_string(),
            path: req.path.clone(),
            status_code: 201,
        });
        json_response(201, &json!(rec))
    }

    fn get_report(&self, id: &str) -> HttpResponse {
        let Ok(guard) = self.state.lock() else {
            return json_response(500, &json!({ "error": "state lock poisoned" }));
        };
        match guard.reports.get(id) {
            Some(r) => json_response(200, &json!(r)),
            None => json_response(404, &json!({ "error": "report not found" })),
        }
    }

    fn create_deployment(&self, req: &HttpRequest) -> HttpResponse {
        let Some(role) = extract_role(req) else {
            return json_response(401, &json!({ "error": "missing or invalid X-Quest-Role" }));
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
            .unwrap_or("app.apk");

        let Ok(mut guard) = self.state.lock() else {
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
        let seq = guard.audit_log.len() as u64 + 1;
        guard.audit_log.push(AuditLogEntry {
            seq,
            role,
            method: "POST".to_string(),
            path: req.path.clone(),
            status_code: 201,
        });
        json_response(
            201,
            &json!({ "id": format!("dep-{idx}"), "deployment": rec }),
        )
    }

    fn get_deployment(&self, id: &str) -> HttpResponse {
        let Ok(guard) = self.state.lock() else {
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

    fn list_audit(&self) -> HttpResponse {
        let Ok(guard) = self.state.lock() else {
            return json_response(500, &json!({ "error": "state lock poisoned" }));
        };
        json_response(200, &json!({ "audit_log": guard.audit_log }))
    }

    fn record_audit(&self, role: Role, method: &str, path: &str, status_code: u16) {
        if let Ok(mut guard) = self.state.lock() {
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
}

fn extract_role(req: &HttpRequest) -> Option<Role> {
    req.headers
        .get("x-quest-role")
        .and_then(|raw| Role::parse_header(raw))
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
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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
