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
