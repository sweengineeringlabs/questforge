//! Typed `quest_forge.yaml` manifest model, defaults, validation, and engine auto-detection.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::ConfigError;

const SUPPORTED_VERSION: &str = "1";
const DEFAULT_MIN_SDK: u32 = 29;
const DEFAULT_TARGET_SDK: u32 = 32;
const DEFAULT_ARCH: &str = "arm64-v8a";
const DEFAULT_JDK: &str = "17";
const DEFAULT_NDK: &str = "23.1.7779620";
const DEFAULT_BUILD_TOOLS: &str = "34.0.0";
const DEFAULT_ADB: &str = "35.0.2";
const DEFAULT_KEYSTORE_ENV: &str = "QUEST_KEYSTORE_PATH";
const DEFAULT_ALIAS_ENV: &str = "QUEST_KEY_ALIAS";
const DEFAULT_REFRESH_RATE_HZ: u32 = 90;

/// Root `quest_forge.yaml` manifest.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestforgeConfig {
    /// Manifest schema version (currently `"1"`).
    pub version: String,
    /// Project identity and engine settings.
    pub project: ProjectConfig,
    /// Target Quest devices and Android SDK settings.
    #[serde(default)]
    pub targets: TargetsConfig,
    /// Required Android/JDK/Meta CLI toolchain versions.
    #[serde(default)]
    pub toolchain: ToolchainConfig,
    /// XR SDK settings.
    #[serde(default)]
    pub sdk: SdkConfig,
    /// Declarative XR runtime and manifest capabilities managed by `quest sync`.
    #[serde(default)]
    pub xr_features: XrFeaturesConfig,
    /// Release signing environment variable names.
    #[serde(default)]
    pub signing: SigningConfig,
}

/// Project identity and game engine metadata.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Human-readable project name.
    pub name: String,
    /// Android application package identifier (e.g., `com.example.myquestapp`).
    pub package_id: String,
    /// Target game engine (`unity` or `unreal`).
    pub engine: EngineKind,
    /// Pinned engine version; auto-detected from project files if omitted.
    #[serde(default)]
    pub engine_version: Option<String>,
}

/// Supported XR game engines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineKind {
    /// Unity Editor.
    Unity,
    /// Unreal Engine.
    Unreal,
}

/// Target hardware and Android API configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TargetsConfig {
    /// Target Quest headset models.
    pub devices: Vec<QuestDevice>,
    /// Minimum Android API level.
    pub min_sdk: u32,
    /// Target Android API level.
    pub target_sdk: u32,
    /// Target Android ABI.
    pub architecture: String,
}

impl Default for TargetsConfig {
    fn default() -> Self {
        Self {
            devices: vec![
                QuestDevice::Quest2,
                QuestDevice::Quest3,
                QuestDevice::Quest3s,
                QuestDevice::QuestPro,
            ],
            min_sdk: DEFAULT_MIN_SDK,
            target_sdk: DEFAULT_TARGET_SDK,
            architecture: DEFAULT_ARCH.to_string(),
        }
    }
}

/// Supported Meta Quest hardware targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestDevice {
    /// Meta Quest 2.
    Quest2,
    /// Meta Quest 3.
    Quest3,
    /// Meta Quest 3S.
    Quest3s,
    /// Meta Quest Pro.
    QuestPro,
}

/// Required local and CI toolchain versions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolchainConfig {
    /// Required JDK major version.
    pub jdk: String,
    /// Required Android NDK version.
    pub android_ndk: String,
    /// Required Android SDK build-tools version.
    pub android_build_tools: String,
    /// Quest-specific CLI tool pins.
    pub metavr_cli: MetaVrCliConfig,
}

impl Default for ToolchainConfig {
    fn default() -> Self {
        Self {
            jdk: DEFAULT_JDK.to_string(),
            android_ndk: DEFAULT_NDK.to_string(),
            android_build_tools: DEFAULT_BUILD_TOOLS.to_string(),
            metavr_cli: MetaVrCliConfig::default(),
        }
    }
}

/// Pinned CLI utilities managed by `quest setup`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MetaVrCliConfig {
    /// Android `platform-tools` (`adb`) version.
    pub adb: String,
    /// Expected SHA-256 hex digest for the `adb` archive.
    pub adb_sha256: Option<String>,
    /// Expected SHA-256 hex digest for Android `cmdline-tools` archive.
    pub cmdline_tools_sha256: Option<String>,
    /// Optional Meta `ovr-platform-util` version.
    pub ovr_platform_util: Option<String>,
    /// Expected SHA-256 hex digest for `ovr-platform-util`.
    pub ovr_sha256: Option<String>,
}

impl Default for MetaVrCliConfig {
    fn default() -> Self {
        Self {
            adb: DEFAULT_ADB.to_string(),
            adb_sha256: None,
            cmdline_tools_sha256: None,
            ovr_platform_util: None,
            ovr_sha256: None,
        }
    }
}

/// XR SDK requirements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SdkConfig {
    /// Whether OpenXR support is required.
    pub openxr: bool,
    /// Minimum/expected Meta XR SDK version.
    pub meta_xr_sdk: Option<String>,
}

impl Default for SdkConfig {
    fn default() -> Self {
        Self {
            openxr: true,
            meta_xr_sdk: None,
        }
    }
}

/// Supported Quest input modes declared in `quest_forge.yaml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InputMode {
    /// Touch / Touch Plus controllers.
    Controllers,
    /// Optical hand tracking (`com.oculus.permission.HAND_TRACKING`).
    Hands,
}

/// Fixed or dynamic foveated rendering level.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FoveationLevel {
    /// Foveated rendering disabled.
    Off,
    /// Low peripheral resolution reduction.
    Low,
    /// Balanced peripheral resolution reduction.
    #[default]
    Medium,
    /// Aggressive peripheral resolution reduction.
    High,
    /// Dynamic GPU-load-driven foveation.
    Dynamic,
}

/// Declarative XR runtime and manifest features synchronized by `quest sync`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct XrFeaturesConfig {
    /// Target headset display refresh rate in Hz (`72`, `90`, or `120`).
    pub refresh_rate_hz: u32,
    /// Enabled user interaction modes (`controllers`, `hands`).
    pub input_modes: Vec<InputMode>,
    /// Enable Mixed Reality passthrough camera support (`com.oculus.feature.PASSTHROUGH`).
    pub passthrough: bool,
    /// Enable persistent spatial anchors (`com.oculus.permission.USE_ANCHOR_API`).
    pub spatial_anchors: bool,
    /// Foveated rendering level (`off`, `low`, `medium`, `high`, `dynamic`).
    pub foveated_rendering: FoveationLevel,
}

impl Default for XrFeaturesConfig {
    fn default() -> Self {
        Self {
            refresh_rate_hz: DEFAULT_REFRESH_RATE_HZ,
            input_modes: vec![InputMode::Controllers],
            passthrough: false,
            spatial_anchors: false,
            foveated_rendering: FoveationLevel::Medium,
        }
    }
}

/// Keystore signing environment variable references.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SigningConfig {
    /// Environment variable holding the keystore file path.
    pub keystore_env: String,
    /// Environment variable holding the key alias.
    pub alias_env: String,
}

impl Default for SigningConfig {
    fn default() -> Self {
        Self {
            keystore_env: DEFAULT_KEYSTORE_ENV.to_string(),
            alias_env: DEFAULT_ALIAS_ENV.to_string(),
        }
    }
}

impl QuestforgeConfig {
    /// Parses and validates a `quest_forge.yaml` string.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, ConfigError> {
        let cfg: Self = serde_yaml::from_str(yaml)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Loads `quest_forge.yaml` from `project_dir`, applying engine-version auto-detection if omitted.
    pub fn load_from_dir(project_dir: &Path) -> Result<Self, ConfigError> {
        let manifest_path = project_dir.join("quest_forge.yaml");
        let raw = fs::read_to_string(&manifest_path).map_err(|source| ConfigError::Io {
            path: manifest_path,
            source,
        })?;
        let mut cfg: Self = serde_yaml::from_str(&raw)?;
        if cfg
            .project
            .engine_version
            .as_deref()
            .is_none_or(|v| v.trim().is_empty())
        {
            cfg.project.engine_version = detect_engine_version(project_dir, cfg.project.engine)?;
        }
        cfg.validate()?;
        Ok(cfg)
    }

    /// Validates manifest invariants and field constraints.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.version.trim() != SUPPORTED_VERSION {
            return Err(ConfigError::UnsupportedVersion {
                version: self.version.clone(),
            });
        }
        if self.project.name.trim().is_empty() {
            return Err(ConfigError::Validation {
                field: "project.name",
                message: "must not be empty".to_string(),
            });
        }
        if !is_valid_package_id(&self.project.package_id) {
            return Err(ConfigError::Validation {
                field: "project.package_id",
                message: "must be a reverse-domain identifier (e.g. com.example.app)".to_string(),
            });
        }
        if self.targets.devices.is_empty() {
            return Err(ConfigError::Validation {
                field: "targets.devices",
                message: "must specify at least one Quest device".to_string(),
            });
        }
        if self.targets.min_sdk > self.targets.target_sdk {
            return Err(ConfigError::Validation {
                field: "targets.min_sdk",
                message: "must be less than or equal to targets.target_sdk".to_string(),
            });
        }
        if !matches!(self.xr_features.refresh_rate_hz, 72 | 90 | 120) {
            return Err(ConfigError::Validation {
                field: "xr_features.refresh_rate_hz",
                message: "must be one of 72, 90, or 120".to_string(),
            });
        }
        if self.xr_features.input_modes.is_empty() {
            return Err(ConfigError::Validation {
                field: "xr_features.input_modes",
                message: "must specify at least one input mode (controllers or hands)".to_string(),
            });
        }
        Ok(())
    }
}

fn is_valid_package_id(package_id: &str) -> bool {
    let parts: Vec<&str> = package_id.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|seg| {
            !seg.is_empty() && seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

fn detect_engine_version(
    project_dir: &Path,
    engine: EngineKind,
) -> Result<Option<String>, ConfigError> {
    match engine {
        EngineKind::Unity => detect_unity_version(project_dir),
        EngineKind::Unreal => detect_unreal_version(project_dir),
    }
}

fn detect_unity_version(project_dir: &Path) -> Result<Option<String>, ConfigError> {
    let version_file = project_dir
        .join("ProjectSettings")
        .join("ProjectVersion.txt");
    if !version_file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&version_file).map_err(|source| ConfigError::Io {
        path: version_file,
        source,
    })?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("m_EditorVersion:") {
            let trimmed = rest.trim();
            if !trimmed.is_empty() {
                return Ok(Some(trimmed.to_string()));
            }
        }
    }
    Ok(None)
}

fn detect_unreal_version(project_dir: &Path) -> Result<Option<String>, ConfigError> {
    let entries = fs::read_dir(project_dir).map_err(|source| ConfigError::Io {
        path: project_dir.to_path_buf(),
        source,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("uproject") {
            let raw = fs::read_to_string(&path).map_err(|source| ConfigError::Io {
                path: path.clone(),
                source,
            })?;
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(ver) = json.get("EngineAssociation").and_then(|v| v.as_str()) {
                    let trimmed = ver.trim();
                    if !trimmed.is_empty() {
                        return Ok(Some(trimmed.to_string()));
                    }
                }
            }
        }
    }
    Ok(None)
}
