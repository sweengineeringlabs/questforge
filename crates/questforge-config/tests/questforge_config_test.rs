//! Integration tests for `questforge-config`.

use std::fs;

use questforge_config::{
    ConfigError, EngineKind, MdmTarget, QuestDevice, QuestforgeConfig, XrSceneSpec,
};
use tempfile::TempDir;

#[test]
fn test_parse_full_manifest_happy() {
    let yaml = r#"
version: "1"
project:
  name: my-quest-app
  package_id: com.example.myquestapp
  engine: unity
  engine_version: "2022.3.42f1"
targets:
  devices:
    - quest2
    - quest3
  min_sdk: 29
  target_sdk: 32
  architecture: arm64-v8a
toolchain:
  jdk: "17"
  android_ndk: "23.1.7779620"
  android_build_tools: "34.0.0"
  metavr_cli:
    adb: "35.0.2"
    ovr_platform_util: "1.99.0"
sdk:
  openxr: true
  meta_xr_sdk: "68.0.0"
signing:
  keystore_env: QUEST_KEYSTORE_PATH
  alias_env: QUEST_KEY_ALIAS
"#;
    let parsed = QuestforgeConfig::from_yaml_str(yaml);
    assert!(matches!(
        parsed,
        Ok(ref cfg)
            if cfg.project.name == "my-quest-app"
                && cfg.project.engine == EngineKind::Unity
                && cfg.project.engine_version.as_deref() == Some("2022.3.42f1")
                && cfg.targets.devices == vec![QuestDevice::Quest2, QuestDevice::Quest3]
                && cfg.toolchain.metavr_cli.ovr_platform_util.as_deref() == Some("1.99.0")
    ));
}

#[test]
fn test_parse_minimal_manifest_applies_defaults_happy() {
    let yaml = r#"
version: "1"
project:
  name: minimal-app
  package_id: com.acme.minimal
  engine: unreal
"#;
    let parsed = QuestforgeConfig::from_yaml_str(yaml);
    assert!(matches!(
        parsed,
        Ok(ref cfg)
            if cfg.targets.min_sdk == 29
                && cfg.targets.target_sdk == 32
                && cfg.toolchain.jdk == "17"
                && cfg.sdk.openxr
    ));
}

#[test]
fn test_load_from_dir_autodetects_unity_version_edge() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let settings = dir.path().join("ProjectSettings");
    assert!(fs::create_dir_all(&settings).is_ok());
    assert!(fs::write(
        settings.join("ProjectVersion.txt"),
        "m_EditorVersion: 2022.3.42f1\nm_EditorVersionWithRevision: 2022.3.42f1 (abc)\n",
    )
    .is_ok());
    assert!(fs::write(
        dir.path().join("quest_forge.yaml"),
        "version: \"1\"\nproject:\n  name: unity-auto\n  package_id: com.acme.unity\n  engine: unity\n",
    )
    .is_ok());

    let loaded = QuestforgeConfig::load_from_dir(dir.path());
    assert!(matches!(
        loaded,
        Ok(ref cfg) if cfg.project.engine_version.as_deref() == Some("2022.3.42f1")
    ));
}

#[test]
fn test_load_from_dir_autodetects_unreal_version_edge() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    assert!(fs::write(
        dir.path().join("MyGame.uproject"),
        r#"{ "FileVersion": 3, "EngineAssociation": "5.4" }"#,
    )
    .is_ok());
    assert!(fs::write(
        dir.path().join("quest_forge.yaml"),
        "version: \"1\"\nproject:\n  name: unreal-auto\n  package_id: com.acme.unreal\n  engine: unreal\n",
    )
    .is_ok());

    let loaded = QuestforgeConfig::load_from_dir(dir.path());
    assert!(matches!(
        loaded,
        Ok(ref cfg) if cfg.project.engine_version.as_deref() == Some("5.4")
    ));
}

#[test]
fn test_parse_unsupported_version_error() {
    let yaml = r#"
version: "99"
project:
  name: bad-ver
  package_id: com.acme.bad
  engine: unity
"#;
    assert!(matches!(
        QuestforgeConfig::from_yaml_str(yaml),
        Err(ConfigError::UnsupportedVersion { ref version }) if version == "99"
    ));
}

#[test]
fn test_parse_invalid_package_id_error() {
    let yaml = r#"
version: "1"
project:
  name: bad-pkg
  package_id: invalid
  engine: unity
"#;
    assert!(matches!(
        QuestforgeConfig::from_yaml_str(yaml),
        Err(ConfigError::Validation {
            field: "project.package_id",
            ..
        })
    ));
}

#[test]
fn test_parse_min_sdk_above_target_sdk_error() {
    let yaml = r#"
version: "1"
project:
  name: bad-sdk
  package_id: com.acme.sdk
  engine: unity
targets:
  min_sdk: 34
  target_sdk: 32
"#;
    assert!(matches!(
        QuestforgeConfig::from_yaml_str(yaml),
        Err(ConfigError::Validation {
            field: "targets.min_sdk",
            ..
        })
    ));
}

#[test]
fn test_parse_xr_features_and_validate_refresh_rate_edge() {
    let valid_yaml = r#"
version: "1"
project:
  name: mr-app
  package_id: com.acme.mrapp
  engine: unity
xr_features:
  refresh_rate_hz: 120
  input_modes: [controllers, hands]
  passthrough: true
  spatial_anchors: true
  foveated_rendering: dynamic
"#;
    let parsed = QuestforgeConfig::from_yaml_str(valid_yaml);
    assert!(matches!(
        parsed,
        Ok(ref cfg)
            if cfg.xr_features.refresh_rate_hz == 120
                && cfg.xr_features.passthrough
                && cfg.xr_features.spatial_anchors
    ));

    let invalid_hz = r#"
version: "1"
project:
  name: mr-app
  package_id: com.acme.mrapp
  engine: unity
xr_features:
  refresh_rate_hz: 60
"#;
    assert!(matches!(
        QuestforgeConfig::from_yaml_str(invalid_hz),
        Err(ConfigError::Validation {
            field: "xr_features.refresh_rate_hz",
            ..
        })
    ));
}

#[test]
fn test_parse_xr_scene_spec_and_validate_handlers_happy() {
    let yaml = r#"
scene: TrainingLab
locomotion:
  mode: teleport
  snap_turn_deg: 45
interactables:
  - id: scalpel_tool
    kind: grabbable
    two_handed: false
    haptic_intensity: 75
    on_activate: OnScalpelPickedUp
spatial_panels:
  - id: checklist_hud
    title: "Procedure Checklist"
    distance_cm: 120
    follow_head: true
    actions:
      - id: complete_step
        label: "Complete Step"
        handler: OnStepCompleted
"#;
    let parsed = XrSceneSpec::from_yaml_str(yaml);
    assert!(matches!(
        parsed,
        Ok(ref spec)
            if spec.scene == "TrainingLab"
                && spec.interactables.len() == 1
                && spec.spatial_panels.len() == 1
    ));

    let bad_handler = r#"
scene: TrainingLab
interactables:
  - id: scalpel_tool
    kind: grabbable
    on_activate: not_pascal_case
"#;
    assert!(matches!(
        XrSceneSpec::from_yaml_str(bad_handler),
        Err(ConfigError::Validation {
            field: "interactables.on_activate",
            ..
        })
    ));
}

#[test]
fn test_parse_mdm_config_for_all_three_providers_happy() {
    for (label, expected) in [
        ("meta-hms", MdmTarget::MetaHms),
        ("managexr", MdmTarget::ManageXr),
        ("arborxr", MdmTarget::ArborXr),
    ] {
        let yaml = format!(
            r#"
version: "1"
project:
  name: mdm-app
  package_id: com.acme.mdm
  engine: rust
mdm:
  provider: {label}
  app_id: app-999
  channel: BETA
  device_group: training-floor
  endpoint: https://custom.mdm.local/v1/releases
  token_env: MY_MDM_TOKEN
"#
        );
        let parsed = QuestforgeConfig::from_yaml_str(&yaml);
        assert!(matches!(
            parsed,
            Ok(ref cfg)
                if cfg.mdm.as_ref().is_some_and(|m| m.provider == expected
                    && m.app_id == "app-999"
                    && m.channel == "BETA"
                    && m.device_group == "training-floor"
                    && m.token_env == "MY_MDM_TOKEN")
        ));
    }
}

#[test]
fn test_targets_future_and_custom_devices_happy() {
    let yaml = r#"
version: "1"
project:
  name: future-app
  package_id: com.acme.future
  engine: rust
targets:
  devices:
    - quest2
    - quest3
    - quest3s
    - questpro
    - quest4
    - mona
"#;
    let Ok(cfg) = QuestforgeConfig::from_yaml_str(yaml) else {
        panic!("valid yaml expected");
    };
    assert_eq!(
        cfg.targets.devices,
        vec![
            QuestDevice::Quest2,
            QuestDevice::Quest3,
            QuestDevice::Quest3s,
            QuestDevice::QuestPro,
            QuestDevice::Custom("quest4".to_string()),
            QuestDevice::Custom("mona".to_string()),
        ]
    );
    assert_eq!(cfg.targets.devices[4].as_tag(), "quest4");
    assert_eq!(cfg.targets.devices[5].as_tag(), "mona");
}
