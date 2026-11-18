//! Integration tests for `questforge-server`.

use std::collections::BTreeMap;

use questforge_server::{
    AuthMode, ControlPlaneServer, HttpRequest, JwtVerifier, Role, StoreBackend,
};

fn make_req(method: &str, path: &str, role: Option<&str>, body: &str) -> HttpRequest {
    let mut headers = BTreeMap::new();
    if let Some(r) = role {
        headers.insert("x-quest-role".to_string(), r.to_string());
    }
    HttpRequest {
        method: method.to_string(),
        path: path.to_string(),
        headers,
        body: body.to_string(),
    }
}

#[test]
fn test_server_projects_builds_verify_deployments_and_audit_happy() {
    let server = ControlPlaneServer::default();

    let health = server.handle_request(&make_req("GET", "/health", None, ""));
    assert_eq!(health.status, 200);

    let create_proj = server.handle_request(&make_req(
        "POST",
        "/api/projects",
        Some("developer"),
        r#"{"name":"vr-app","repository":"https://example.invalid/vr","owner":"xr-team"}"#,
    ));
    assert_eq!(create_proj.status, 201);

    let create_build = server.handle_request(&make_req(
        "POST",
        "/api/builds",
        Some("platform_engineer"),
        r#"{"project_id":"proj-1","version":"1.0.0"}"#,
    ));
    assert_eq!(create_build.status, 201);

    let create_dep = server.handle_request(&make_req(
        "POST",
        "/api/deployments",
        Some("operations_engineer"),
        r#"{"target":"qa-lab","apk_path":"dist/vr-app.apk"}"#,
    ));
    assert_eq!(create_dep.status, 201);

    let list_deps = server.handle_request(&make_req("GET", "/api/deployments", None, ""));
    assert_eq!(list_deps.status, 200);
    assert!(String::from_utf8_lossy(&list_deps.body).contains("qa-lab"));

    let metrics = server.handle_request(&make_req("GET", "/metrics", None, ""));
    assert_eq!(metrics.status, 200);
    assert!(String::from_utf8_lossy(&metrics.body).contains("questforge_projects_total 1"));

    let audit = server.handle_request(&make_req("GET", "/api/audit", None, ""));
    assert_eq!(audit.status, 200);
    assert!(String::from_utf8_lossy(&audit.body).contains("/api/deployments"));
}

#[test]
fn test_server_rbac_rejects_unauthorized_and_unauthenticated_error() {
    let server = ControlPlaneServer::default();

    let unauth = server.handle_request(&make_req(
        "POST",
        "/api/projects",
        None,
        r#"{"name":"unauth"}"#,
    ));
    assert_eq!(unauth.status, 401);

    let forbidden = server.handle_request(&make_req(
        "POST",
        "/api/deployments",
        Some("auditor"),
        r#"{"target":"prod"}"#,
    ));
    assert_eq!(forbidden.status, 403);
}

#[test]
fn test_server_jwt_auth_enforces_signed_tokens_edge() {
    let verifier = JwtVerifier::new(b"super-secret-signing-key".to_vec());
    let server = ControlPlaneServer::with_backend(
        StoreBackend::default(),
        AuthMode::Jwt(verifier.clone()),
        None,
    );

    let token = verifier.sign_token("amu", Role::Developer, None);
    assert!(token.is_ok());
    if let Ok(tok) = token {
        let mut headers = BTreeMap::new();
        headers.insert("authorization".to_string(), format!("Bearer {tok}"));
        let resp = server.handle_request(&HttpRequest {
            method: "POST".to_string(),
            path: "/api/projects".to_string(),
            headers,
            body: r#"{"name":"jwt-app"}"#.to_string(),
        });
        assert_eq!(resp.status, 201);
    }

    let mut bad_headers = BTreeMap::new();
    bad_headers.insert(
        "authorization".to_string(),
        "Bearer invalid.token.sig".to_string(),
    );
    let bad_resp = server.handle_request(&HttpRequest {
        method: "POST".to_string(),
        path: "/api/projects".to_string(),
        headers: bad_headers,
        body: r#"{"name":"bad"}"#.to_string(),
    });
    assert_eq!(bad_resp.status, 401);
}

#[test]
fn test_server_serves_static_css_and_blocks_traversal_happy() {
    let Ok(dir) = tempfile::TempDir::new() else {
        panic!("failed to create tempdir");
    };
    assert!(std::fs::write(dir.path().join("app.css"), "body { color: #fff; }").is_ok());

    let server = ControlPlaneServer::new(Some(dir.path().to_path_buf()));
    let css = server.handle_request(&make_req("GET", "/app.css", None, ""));
    assert_eq!(css.status, 200);
    assert_eq!(css.content_type, "text/css; charset=utf-8");

    let traversal = server.handle_request(&make_req("GET", "/../secret.txt", None, ""));
    assert_eq!(traversal.status, 404);
}
