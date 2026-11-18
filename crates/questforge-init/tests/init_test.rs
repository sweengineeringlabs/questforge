//! Integration tests for `questforge-init`.

use std::fs;

use questforge_config::{EngineKind, QuestforgeConfig};
use questforge_init::{InitError, InitOptions, ProjectScaffolder};
use tempfile::TempDir;

#[test]
fn test_scaffold_unity_project_creates_valid_manifest_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let target = dir.path().join("my-unity-quest");
    let opts = InitOptions {
        target_dir: target.clone(),
        name: "my-unity-quest".to_string(),
        package_id: "com.acme.unityquest".to_string(),
        engine: EngineKind::Unity,
        engine_version: None,
        template_url: None,
        force: false,
    };

    let report = ProjectScaffolder::scaffold(&opts);
    assert!(report.is_ok());
    assert!(target.join("quest_forge.yaml").exists());
    assert!(target.join("config/vocabulary.toml").exists());
    assert!(target.join("scripts/ci.sh").exists());
    assert!(target.join("ProjectSettings/ProjectVersion.txt").exists());
    assert!(target.join("Packages/manifest.json").exists());
    assert!(target.join("Assets/Editor/QuestForgeBuild.cs").exists());

    let loaded = QuestforgeConfig::load_from_dir(&target);
    assert!(matches!(
        loaded,
        Ok(ref cfg) if cfg.project.name == "my-unity-quest" && cfg.project.engine == EngineKind::Unity
    ));
}

#[test]
fn test_scaffold_unreal_project_creates_uproject_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let target = dir.path().join("my-unreal-quest");
    let opts = InitOptions {
        target_dir: target.clone(),
        name: "my-unreal-quest".to_string(),
        package_id: "com.acme.unrealquest".to_string(),
        engine: EngineKind::Unreal,
        engine_version: Some("5.4".to_string()),
        template_url: None,
        force: false,
    };

    assert!(ProjectScaffolder::scaffold(&opts).is_ok());
    assert!(target.join("QuestApp.uproject").exists());
    assert!(target.join(".gitattributes").exists());

    let loaded = QuestforgeConfig::load_from_dir(&target);
    assert!(matches!(
        loaded,
        Ok(ref cfg) if cfg.project.engine == EngineKind::Unreal
    ));
}

#[test]
fn test_scaffold_refuses_non_empty_target_without_force_error() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let marker = dir.path().join("existing.txt");
    assert!(fs::write(&marker, "keep me").is_ok());

    let opts = InitOptions {
        target_dir: dir.path().to_path_buf(),
        name: "conflict".to_string(),
        package_id: "com.acme.conflict".to_string(),
        engine: EngineKind::Unity,
        engine_version: None,
        template_url: None,
        force: false,
    };

    assert!(matches!(
        ProjectScaffolder::scaffold(&opts),
        Err(InitError::TargetNotEmpty { .. })
    ));
    assert!(!dir.path().join("quest_forge.yaml").exists());
}

#[test]
fn test_sync_and_check_project_detects_drift_and_perf_violations_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let target = dir.path().join("mr-quest");
    let opts = InitOptions {
        target_dir: target.clone(),
        name: "mr-quest".to_string(),
        package_id: "com.acme.mrquest".to_string(),
        engine: EngineKind::Unity,
        engine_version: None,
        template_url: None,
        force: false,
    };
    assert!(ProjectScaffolder::scaffold(&opts).is_ok());
    assert!(target.join("specs/main_arena_xr.yaml").exists());
    assert!(target
        .join("Assets/Generated/MainArenaXrScene.gen.cs")
        .exists());
    assert!(target.join("Assets/Scripts/MainArenaActions.cs").exists());

    let initial_check = ProjectScaffolder::check_project(&target);
    assert!(matches!(initial_check, Ok(ref r) if r.passed && r.findings.len() == 5));

    assert!(fs::write(
        target.join("quest_forge.yaml"),
        r#"version: "1"
project:
  name: mr-quest
  package_id: com.acme.mrquest
  engine: unity
  engine_version: "2022.3.42f1"
xr_features:
  refresh_rate_hz: 120
  input_modes: [controllers, hands]
  passthrough: true
  spatial_anchors: true
  foveated_rendering: high
"#,
    )
    .is_ok());

    let drifted_check = ProjectScaffolder::check_project(&target);
    assert!(matches!(
        drifted_check,
        Ok(ref r)
            if !r.passed
                && r.findings.iter().any(|f| f.code == "QF-SYNC-001" && !f.passed)
    ));

    let sync_res = ProjectScaffolder::sync_project(&target);
    assert!(sync_res.is_ok());

    let manifest_xml =
        fs::read_to_string(target.join("Assets/Plugins/Android/AndroidManifest.xml"));
    assert!(matches!(
        manifest_xml,
        Ok(ref xml)
            if xml.contains("com.oculus.permission.HAND_TRACKING")
                && xml.contains("com.oculus.feature.PASSTHROUGH")
                && xml.contains("com.oculus.permission.USE_ANCHOR_API")
    ));

    let post_sync_check = ProjectScaffolder::check_project(&target);
    assert!(matches!(post_sync_check, Ok(ref r) if r.passed));

    let scripts_dir = target.join("Assets/Scripts");
    assert!(fs::create_dir_all(&scripts_dir).is_ok());
    assert!(fs::write(
        scripts_dir.join("BadLoop.cs"),
        "using UnityEngine;\npublic class BadLoop : MonoBehaviour {\n  void Update() { var cam = Camera.main; }\n}\n",
    )
    .is_ok());

    let perf_check = ProjectScaffolder::check_project(&target);
    assert!(matches!(
        perf_check,
        Ok(ref r)
            if !r.passed
                && r.findings.iter().any(|f| f.code == "QF-PERF-001" && !f.passed)
    ));
}

#[test]
fn test_scaffold_rust_openxr_project_sync_and_check_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let target = dir.path().join("my-rust-quest");
    let opts = InitOptions {
        target_dir: target.clone(),
        name: "my-rust-quest".to_string(),
        package_id: "com.acme.rustquest".to_string(),
        engine: EngineKind::Rust,
        engine_version: None,
        template_url: None,
        force: false,
    };

    assert!(ProjectScaffolder::scaffold(&opts).is_ok());
    assert!(target.join("Cargo.toml").exists());
    assert!(target.join("rust-toolchain.toml").exists());
    assert!(target.join("AndroidManifest.xml").exists());
    assert!(target.join("src/questforge_bootstrap_gen.rs").exists());
    assert!(target.join("src/main_arena_xr_scene_gen.rs").exists());
    assert!(target.join("src/main_arena_actions.rs").exists());

    let check = ProjectScaffolder::check_project(&target);
    assert!(matches!(check, Ok(ref r) if r.passed && r.findings.len() == 5));

    let bootstrap_rs = fs::read_to_string(target.join("src/questforge_bootstrap_gen.rs"));
    assert!(matches!(
        bootstrap_rs,
        Ok(ref code)
            if code.contains("XR_FB_display_refresh_rate")
                && code.contains("build_openxr_session_config")
                && code.contains("run_frame_step")
    ));
}
