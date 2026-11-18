//! Integration tests for `questforge-ci`.

use std::collections::BTreeMap;
use std::fs;

use questforge_ci::{
    CiCmdOutput, CiEnv, CiError, CiOptions, CiOrchestrator, CiProvider, MockCiEnv,
};
use questforge_config::EngineKind;
use tempfile::TempDir;

#[test]
fn test_ci_unity_dry_run_emits_metadata_and_sbom_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let proj = dir.path().join("app");
    let out = dir.path().join("dist");
    assert!(fs::create_dir_all(proj.join("Packages")).is_ok());
    assert!(fs::write(
        proj.join("quest_forge.yaml"),
        r#"
version: "1"
project:
  name: quest-ci-demo
  package_id: com.acme.cidemo
  engine: unity
  engine_version: "2022.3.42f1"
sdk:
  openxr: true
  meta_xr_sdk: "68.0.0"
"#,
    )
    .is_ok());
    assert!(fs::write(
        proj.join("Packages").join("manifest.json"),
        r#"{"dependencies":{"com.meta.xr.sdk.all":"68.0.0","com.oculus.xr":"1.0.0"}}"#,
    )
    .is_ok());

    let opts = CiOptions {
        project_dir: proj,
        output_dir: out,
        release_version: "1.2.3".to_string(),
        upload_channel: Some("ALPHA".to_string()),
        app_id: Some("123456".to_string()),
        dry_run: true,
    };

    let report = CiOrchestrator::run_with_env(&opts, &CiEnv::Mock(MockCiEnv::default()));
    assert!(matches!(
        report,
        Ok(ref r)
            if r.metadata.engine == EngineKind::Unity
                && r.metadata_path.exists()
                && r.sbom_path.exists()
                && r.upload_command.is_some()
                && r.vulnerabilities.len() == 1
    ));
}

#[test]
fn test_ci_unreal_build_failure_surfaces_diagnostics_error() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let proj = dir.path().join("unreal");
    let out = dir.path().join("dist");
    assert!(fs::create_dir_all(&proj).is_ok());
    assert!(fs::write(
        proj.join("quest_forge.yaml"),
        r#"
version: "1"
project:
  name: unreal-demo
  package_id: com.acme.unreal
  engine: unreal
  engine_version: "5.4"
"#,
    )
    .is_ok());

    let mut commands = BTreeMap::new();
    commands.insert(
        "RunUAT".to_string(),
        CiCmdOutput {
            exit_code: Some(2),
            stdout: String::new(),
            stderr: "ERROR: Android NDK r23b missing".to_string(),
        },
    );

    let opts = CiOptions {
        project_dir: proj,
        output_dir: out,
        release_version: "0.1.0".to_string(),
        upload_channel: None,
        app_id: None,
        dry_run: false,
    };

    let res = CiOrchestrator::run_with_env(
        &opts,
        &CiEnv::Mock(MockCiEnv {
            vars: BTreeMap::new(),
            commands,
        }),
    );
    assert!(matches!(
        res,
        Err(CiError::StepFailed {
            ref step,
            exit_code: Some(2),
            ref summary,
            ..
        }) if step == "compile" && summary.contains("Android NDK")
    ));
}

#[test]
fn test_ci_render_pipeline_templates_edge() {
    assert!(CiOrchestrator::render_pipeline_template(CiProvider::Github).contains("quest ci"));
    assert!(CiOrchestrator::render_pipeline_template(CiProvider::Azure).contains("quest doctor"));
    assert!(CiOrchestrator::render_pipeline_template(CiProvider::Jenkins).contains("pipeline"));
}
