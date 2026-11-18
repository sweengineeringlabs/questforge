//! Integration tests for `questforge-ops`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use questforge_ops::{
    AdbOutput, DeploymentRecord, FleetInventory, FleetManager, HmsReleaseRequest, MdmProvider,
    MockOpsEnv, OpsEnv, OpsError,
};
use tempfile::TempDir;

#[test]
fn test_ops_discover_telemetry_deploy_rollback_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let registry_path = dir.path().join("fleet.json");

    let mut responses = BTreeMap::new();
    responses.insert(
        "devices".to_string(),
        AdbOutput {
            success: true,
            stdout: b"List of devices attached\nemulator-5554\tdevice\n".to_vec(),
            stderr: String::new(),
        },
    );
    responses.insert(
        "-s emulator-5554 shell getprop ro.build.display.id".to_string(),
        AdbOutput {
            success: true,
            stdout: b"SQ3A.220605.009.A1\n".to_vec(),
            stderr: String::new(),
        },
    );
    responses.insert(
        "-s emulator-5554 shell dumpsys battery".to_string(),
        AdbOutput {
            success: true,
            stdout: b"Current Battery Service state:\n  level: 87\n  health: 2\n".to_vec(),
            stderr: String::new(),
        },
    );

    let env = OpsEnv::Mock(MockOpsEnv {
        responses,
        ..Default::default()
    });
    let mut inv = FleetInventory::default();

    let found = FleetManager::discover(&mut inv, "qa-lab", &env);
    assert!(matches!(found, Ok(ref devs) if devs.len() == 1 && devs[0].serial == "emulator-5554"));

    let updated = FleetManager::refresh_telemetry(&mut inv, "emulator-5554", &env);
    assert!(matches!(
        updated,
        Ok(ref d)
            if d.firmware_version.as_deref() == Some("SQ3A.220605.009.A1")
                && d.battery_level == Some(87)
                && d.battery_health.as_deref() == Some("good")
    ));

    assert!(FleetManager::deploy_apk(&mut inv, "qa-lab", Path::new("v1.apk"), None, &env).is_ok());
    assert!(FleetManager::deploy_apk(&mut inv, "qa-lab", Path::new("v2.apk"), None, &env).is_ok());
    let rolled = FleetManager::rollback(&mut inv, "qa-lab", &env);
    assert!(matches!(
        rolled,
        Ok(ref r) if r.status == "rolled_back" && r.apk_path == Path::new("v1.apk")
    ));

    assert!(inv.save(&registry_path).is_ok());
    let reloaded = FleetInventory::load(&registry_path);
    assert!(matches!(
        reloaded,
        Ok(ref loaded) if loaded.devices.contains_key("emulator-5554") && loaded.deployments.len() == 3
    ));
}

#[test]
fn test_ops_logcat_and_screenshot_capture_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let mut responses = BTreeMap::new();
    responses.insert(
        "-s emulator-5554 logcat -d".to_string(),
        AdbOutput {
            success: true,
            stdout: b"I/Unity: XR initialized\n".to_vec(),
            stderr: String::new(),
        },
    );
    responses.insert(
        "-s emulator-5554 exec-out screencap -p".to_string(),
        AdbOutput {
            success: true,
            stdout: b"\x89PNG\r\n\x1a\n".to_vec(),
            stderr: String::new(),
        },
    );
    let env = OpsEnv::Mock(MockOpsEnv {
        responses,
        ..Default::default()
    });

    let log_path = dir.path().join("device.log");
    let shot_path = dir.path().join("screen.png");
    assert!(FleetManager::capture_logcat("emulator-5554", &log_path, &env).is_ok());
    assert!(FleetManager::capture_screenshot("emulator-5554", &shot_path, &env).is_ok());
    assert!(log_path.exists());
    assert!(shot_path.exists());
}

#[test]
fn test_ops_rollback_without_previous_apk_error() {
    let mut inv = FleetInventory::default();
    FleetManager::register_device(&mut inv, "quest-01", "canary");
    let res = FleetManager::rollback(&mut inv, "quest-01", &OpsEnv::Mock(MockOpsEnv::default()));
    assert!(matches!(
        res,
        Err(OpsError::NoRollbackTarget { ref serial }) if serial == "quest-01"
    ));
}

#[test]
fn test_ops_agent_executes_matching_scheduled_deployments_happy() {
    let mut responses = BTreeMap::new();
    responses.insert(
        "devices".to_string(),
        AdbOutput {
            success: true,
            stdout: b"List of devices attached\nemulator-5554\tdevice\n".to_vec(),
            stderr: String::new(),
        },
    );
    let env = OpsEnv::Mock(MockOpsEnv {
        responses,
        ..Default::default()
    });
    let mut inv = FleetInventory::default();
    let pending = vec![DeploymentRecord {
        target: "qa-lab".to_string(),
        apk_path: Path::new("dist/quest-app.apk").to_path_buf(),
        scheduled_at: None,
        status: "scheduled".to_string(),
        devices: Vec::new(),
    }];

    let report = FleetManager::sync_agent_with_deployments(&mut inv, "qa-lab", &pending, &env);
    assert!(matches!(
        report,
        Ok(ref r)
            if r.discovered == 1
                && r.executed_deployments.len() == 1
                && r.executed_deployments[0].status == "deployed"
    ));
}

#[test]
fn test_ops_publish_to_mdm_hms_and_missing_token_error() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let apk_path = dir.path().join("orbit.apk");
    assert!(fs::write(&apk_path, b"PK\x03\x04dummy-apk").is_ok());

    let mut http_responses = BTreeMap::new();
    http_responses.insert(
        MdmProvider::MetaHms.default_endpoint_for("app-123"),
        r#"{"id":"hms-rel-42","status":"published"}"#.to_string(),
    );
    let env = OpsEnv::Mock(MockOpsEnv {
        http_responses,
        ..Default::default()
    });

    let receipt = FleetManager::publish_to_mdm(
        &HmsReleaseRequest {
            provider: MdmProvider::MetaHms,
            app_id: "app-123".to_string(),
            apk_path: apk_path.clone(),
            channel: "PRODUCTION".to_string(),
            device_group: "training-floor".to_string(),
            endpoint: None,
            access_token: "hms-secret-token".to_string(),
        },
        &env,
    );
    assert!(matches!(
        receipt,
        Ok(ref r)
            if r.provider == "meta-hms"
                && r.release_id == "hms-rel-42"
                && r.status == "published"
                && r.device_group == "training-floor"
                && !r.apk_sha256.is_empty()
    ));

    let missing_token = FleetManager::publish_to_mdm(
        &HmsReleaseRequest {
            provider: MdmProvider::ManageXr,
            app_id: "app-123".to_string(),
            apk_path,
            channel: "BETA".to_string(),
            device_group: "qa-lab".to_string(),
            endpoint: None,
            access_token: String::new(),
        },
        &env,
    );
    assert!(matches!(missing_token, Err(OpsError::MdmFailed { .. })));
}
