//! Integration tests for `questforge-doctor`.

use std::collections::BTreeMap;
use std::fs;

use questforge_doctor::{
    CheckStatus, CmdOutput, DoctorEnv, DoctorOptions, DoctorRunner, MockDoctorEnv,
};
use tempfile::TempDir;

fn create_full_fixture() -> (TempDir, MockDoctorEnv) {
    let Ok(root) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let proj = root.path().join("app");
    let unity = root.path().join("unity");
    let sdk = root.path().join("android-sdk");
    let ndk = sdk.join("ndk").join("23.1.7779620");
    let keystore = root.path().join("release.keystore");

    assert!(fs::create_dir_all(proj.join("ProjectSettings")).is_ok());
    assert!(fs::create_dir_all(proj.join("Packages")).is_ok());
    assert!(fs::create_dir_all(proj.join("config")).is_ok());
    assert!(fs::create_dir_all(&unity).is_ok());
    assert!(fs::create_dir_all(sdk.join("platforms").join("android-32")).is_ok());
    assert!(fs::create_dir_all(sdk.join("build-tools").join("34.0.0")).is_ok());
    assert!(fs::create_dir_all(&ndk).is_ok());
    assert!(fs::write(&keystore, b"dummy").is_ok());

    assert!(fs::write(
        proj.join("quest_forge.yaml"),
        r#"
version: "1"
project:
  name: quest-sample
  package_id: com.acme.sample
  engine: unity
  engine_version: "2022.3.42f1"
"#,
    )
    .is_ok());
    assert!(fs::write(
        proj.join("Packages").join("manifest.json"),
        r#"{"dependencies":{"com.meta.xr.sdk.all":"68.0.0","com.unity.xr.openxr":"1.10.0"}}"#,
    )
    .is_ok());
    assert!(fs::write(proj.join("config").join("vocabulary.toml"), "[folders]\n").is_ok());

    let mut vars = BTreeMap::new();
    vars.insert("UNITY_PATH".to_string(), unity.display().to_string());
    vars.insert("JAVA_HOME".to_string(), "/opt/jdk-17".to_string());
    vars.insert("ANDROID_HOME".to_string(), sdk.display().to_string());
    vars.insert(
        "QUEST_KEYSTORE_PATH".to_string(),
        keystore.display().to_string(),
    );
    vars.insert("QUEST_KEY_ALIAS".to_string(), "release".to_string());

    let mut commands = BTreeMap::new();
    commands.insert(
        "java".to_string(),
        CmdOutput {
            success: true,
            stdout: "openjdk 17".to_string(),
            stderr: String::new(),
        },
    );
    commands.insert(
        "adb".to_string(),
        CmdOutput {
            success: true,
            stdout: "List of devices attached\nemulator-5554\tdevice\n".to_string(),
            stderr: String::new(),
        },
    );
    commands.insert(
        "swartch".to_string(),
        CmdOutput {
            success: true,
            stdout: "audit passed".to_string(),
            stderr: String::new(),
        },
    );

    (root, MockDoctorEnv { vars, commands })
}

#[test]
fn test_doctor_all_checks_pass_happy() {
    let (root, mock) = create_full_fixture();
    let opts = DoctorOptions {
        project_dir: root.path().join("app"),
        ci: false,
        strict: false,
        fix: false,
    };

    let report = DoctorRunner::run_with_env(&opts, &DoctorEnv::Mock(mock));
    assert!(matches!(
        report,
        Ok(ref r)
            if r.checks.len() == 9
                && !r.has_failures()
                && r.checks.iter().all(|c| c.status == CheckStatus::Pass)
    ));
}

#[test]
fn test_doctor_missing_manifest_fails_with_remediation_error() {
    let Ok(empty) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let opts = DoctorOptions {
        project_dir: empty.path().to_path_buf(),
        ci: false,
        strict: false,
        fix: false,
    };
    let report = DoctorRunner::run_with_env(&opts, &DoctorEnv::Mock(MockDoctorEnv::default()));
    assert!(matches!(
        report,
        Ok(ref r)
            if r.has_failures()
                && r.checks[0].id == "QD-001"
                && r.checks[0].status == CheckStatus::Fail
                && r.checks[0].remediation.is_some()
    ));
}

#[test]
fn test_doctor_ci_mode_skips_headset_and_enforces_signing_and_swartch_edge() {
    let (root, mut mock) = create_full_fixture();
    mock.vars.remove("QUEST_KEYSTORE_PATH");
    mock.commands.insert(
        "adb".to_string(),
        CmdOutput {
            success: true,
            stdout: "List of devices attached\n\n".to_string(),
            stderr: String::new(),
        },
    );
    mock.commands.insert(
        "swartch".to_string(),
        CmdOutput {
            success: false,
            stdout: "[AF_ERR_004] violation".to_string(),
            stderr: String::new(),
        },
    );

    let opts = DoctorOptions {
        project_dir: root.path().join("app"),
        ci: true,
        strict: false,
        fix: false,
    };
    let report = DoctorRunner::run_with_env(&opts, &DoctorEnv::Mock(mock));
    assert!(matches!(
        report,
        Ok(ref r)
            if r.has_failures()
                && r.checks.iter().any(|c| c.id == "QD-007" && c.status == CheckStatus::Pass)
                && r.checks.iter().any(|c| c.id == "QD-008" && c.status == CheckStatus::Fail)
                && r.checks.iter().any(|c| c.id == "QD-009" && c.status == CheckStatus::Fail)
    ));
}

#[test]
fn test_doctor_fix_heals_missing_packages_and_vocabulary_happy() {
    let (root, mock) = create_full_fixture();
    let proj = root.path().join("app");
    assert!(fs::remove_file(proj.join("Packages").join("manifest.json")).is_ok());
    assert!(fs::remove_file(proj.join("config").join("vocabulary.toml")).is_ok());

    let opts = DoctorOptions {
        project_dir: proj.clone(),
        ci: false,
        strict: false,
        fix: true,
    };
    let report = DoctorRunner::run_with_env(&opts, &DoctorEnv::Mock(mock));
    assert!(matches!(
        report,
        Ok(ref r)
            if !r.has_failures()
                && r.checks.iter().all(|c| c.status == CheckStatus::Pass)
    ));
    assert!(proj.join("Packages").join("manifest.json").exists());
    assert!(proj.join("config").join("vocabulary.toml").exists());
}
