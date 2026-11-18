//! Project scaffolding, declarative code/manifest synchronization (`quest sync`),
//! and source-level XR linting (`quest check`) for Unity and Unreal Meta Quest repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use questforge_config::{
    EngineKind, FoveationLevel, InputMode, MetaVrCliConfig, ProjectConfig, QuestDevice,
    QuestforgeConfig, SdkConfig, SigningConfig, TargetsConfig, ToolchainConfig, XrFeaturesConfig,
};
use serde::{Deserialize, Serialize};

use crate::InitError;

const DEFAULT_UNITY_VERSION: &str = "2022.3.42f1";
const DEFAULT_UNREAL_VERSION: &str = "5.4";

/// Options controlling `quest init`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InitOptions {
    /// Target directory to scaffold into.
    pub target_dir: PathBuf,
    /// Project name (defaults to target directory name).
    pub name: String,
    /// Android package ID (e.g., `com.example.myquestapp`).
    pub package_id: String,
    /// Target game engine.
    pub engine: EngineKind,
    /// Optional pinned engine version.
    pub engine_version: Option<String>,
    /// Optional external Git template URL (`--template`).
    pub template_url: Option<String>,
    /// Allow writing into a non-empty directory.
    pub force: bool,
}

/// Summary of files created by `quest init`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitReport {
    /// Target directory.
    pub target_dir: PathBuf,
    /// Engine selected.
    pub engine: EngineKind,
    /// Relative paths of generated files.
    pub created_files: Vec<PathBuf>,
}

/// Summary of files synchronized by `quest sync`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncReport {
    /// Target project directory.
    pub project_dir: PathBuf,
    /// Engine synchronized.
    pub engine: EngineKind,
    /// Relative paths of generated/updated files.
    pub synced_files: Vec<PathBuf>,
}

/// Single diagnostic finding from `quest check`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFinding {
    /// Check identifier (`QF-SYNC-001`, `QF-VRC-001`, `QF-PERF-001`).
    pub code: String,
    /// Short human-readable check title.
    pub name: String,
    /// True if the check passed with zero offences.
    pub passed: bool,
    /// Detailed status or offence description.
    pub message: String,
    /// Actionable remediation command or guidance when `passed` is false.
    pub remediation: Option<String>,
}

/// Aggregate report emitted by `quest check`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceCheckReport {
    /// Checked project directory.
    pub project_dir: PathBuf,
    /// True if all checks passed.
    pub passed: bool,
    /// Individual check findings.
    pub findings: Vec<SourceFinding>,
}

/// Scaffolds, synchronizes, and statically checks Quest-ready repositories.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProjectScaffolder;

impl ProjectScaffolder {
    /// Scaffolds a QuestForge project into `opts.target_dir`.
    pub fn scaffold(opts: &InitOptions) -> Result<InitReport, InitError> {
        ensure_target_writable(&opts.target_dir, opts.force)?;

        if let Some(url) = &opts.template_url {
            clone_template(url, &opts.target_dir)?;
        }

        let engine_ver = opts
            .engine_version
            .clone()
            .unwrap_or_else(|| match opts.engine {
                EngineKind::Unity => DEFAULT_UNITY_VERSION.to_string(),
                EngineKind::Unreal => DEFAULT_UNREAL_VERSION.to_string(),
            });

        let cfg = QuestforgeConfig {
            version: "1".to_string(),
            project: ProjectConfig {
                name: opts.name.clone(),
                package_id: opts.package_id.clone(),
                engine: opts.engine,
                engine_version: Some(engine_ver),
            },
            targets: TargetsConfig::default(),
            toolchain: ToolchainConfig {
                metavr_cli: MetaVrCliConfig::default(),
                ..ToolchainConfig::default()
            },
            sdk: SdkConfig {
                openxr: true,
                meta_xr_sdk: Some("68.0.0".to_string()),
            },
            xr_features: XrFeaturesConfig::default(),
            signing: SigningConfig::default(),
        };

        let yaml = serde_yaml::to_string(&cfg)?;
        let mut files: Vec<(String, String)> = vec![
            ("quest_forge.yaml".to_string(), yaml),
            (
                "config/vocabulary.toml".to_string(),
                minimal_vocabulary_toml(),
            ),
            ("scripts/ci.sh".to_string(), starter_ci_sh()),
            ("configs/app.toml".to_string(), starter_app_toml(&opts.name)),
            (".gitignore".to_string(), gitignore_for(opts.engine)),
            (".gitattributes".to_string(), gitattributes_for_xr()),
            ("CODEOWNERS".to_string(), "* @platform-team\n".to_string()),
            ("SECURITY.md".to_string(), security_md()),
            ("docs/onboarding.md".to_string(), onboarding_md(&opts.name)),
            ("README.md".to_string(), readme_md(&opts.name, opts.engine)),
            ("src/.gitkeep".to_string(), String::new()),
        ];

        for (rel, content) in managed_sync_files(&cfg) {
            files.push((rel, content));
        }

        let mut created_files = Vec::with_capacity(files.len());
        for (rel, content) in files {
            let dest = opts.target_dir.join(&rel);
            write_file_checked(&dest, &content, opts.force)?;
            created_files.push(PathBuf::from(rel));
        }

        Ok(InitReport {
            target_dir: opts.target_dir.clone(),
            engine: opts.engine,
            created_files,
        })
    }

    /// Synchronizes declarative `quest_forge.yaml` settings into engine manifests,
    /// `AndroidManifest.xml`, and generated runtime bootstrap files (`quest sync`).
    pub fn sync_project(project_dir: &Path) -> Result<SyncReport, InitError> {
        let cfg = QuestforgeConfig::load_from_dir(project_dir)?;
        let managed = managed_sync_files(&cfg);
        let mut synced_files = Vec::with_capacity(managed.len());

        for (rel, content) in managed {
            let dest = project_dir.join(&rel);
            write_file_checked(&dest, &content, true)?;
            synced_files.push(PathBuf::from(rel));
        }

        Ok(SyncReport {
            project_dir: project_dir.to_path_buf(),
            engine: cfg.project.engine,
            synced_files,
        })
    }

    /// Runs sub-second static source checks (`QF-SYNC-001`, `QF-VRC-001`, `QF-PERF-001`)
    /// on `project_dir` (`quest check`).
    pub fn check_project(project_dir: &Path) -> Result<SourceCheckReport, InitError> {
        let cfg = QuestforgeConfig::load_from_dir(project_dir)?;
        let findings = vec![
            check_sync_drift(project_dir, &cfg),
            check_source_vrc(project_dir, &cfg),
            check_xr_perf_patterns(project_dir),
        ];
        let passed = findings.iter().all(|f| f.passed);

        Ok(SourceCheckReport {
            project_dir: project_dir.to_path_buf(),
            passed,
            findings,
        })
    }
}

fn managed_sync_files(cfg: &QuestforgeConfig) -> Vec<(String, String)> {
    let engine_ver =
        cfg.project
            .engine_version
            .clone()
            .unwrap_or_else(|| match cfg.project.engine {
                EngineKind::Unity => DEFAULT_UNITY_VERSION.to_string(),
                EngineKind::Unreal => DEFAULT_UNREAL_VERSION.to_string(),
            });

    match cfg.project.engine {
        EngineKind::Unity => vec![
            (
                "ProjectSettings/ProjectVersion.txt".to_string(),
                format!("m_EditorVersion: {engine_ver}\n"),
            ),
            (
                "Packages/manifest.json".to_string(),
                unity_packages_manifest(&cfg.sdk),
            ),
            (
                "Assets/Editor/QuestForgeBuild.cs".to_string(),
                unity_build_script_cs(),
            ),
            (
                "Assets/Plugins/Android/AndroidManifest.xml".to_string(),
                generate_android_manifest_xml(cfg),
            ),
            (
                "Assets/Generated/QuestForgeBootstrap.gen.cs".to_string(),
                generate_unity_bootstrap_cs(cfg),
            ),
        ],
        EngineKind::Unreal => vec![
            (
                "QuestApp.uproject".to_string(),
                unreal_uproject_json(&engine_ver, &cfg.sdk),
            ),
            (
                "Config/DefaultEngine.ini".to_string(),
                generate_unreal_default_engine_ini(cfg),
            ),
            (
                "Build/Android/AndroidManifest.xml".to_string(),
                generate_android_manifest_xml(cfg),
            ),
            (
                "Source/QuestForge/QuestForgeBootstrap.gen.h".to_string(),
                generate_unreal_bootstrap_h(cfg),
            ),
        ],
    }
}

fn check_sync_drift(project_dir: &Path, cfg: &QuestforgeConfig) -> SourceFinding {
    let expected = managed_sync_files(cfg);
    let mut drifted = Vec::new();
    for (rel, expected_content) in expected {
        let path = project_dir.join(&rel);
        match fs::read_to_string(&path) {
            Ok(actual) if normalize_newlines(&actual) == normalize_newlines(&expected_content) => {}
            _ => drifted.push(rel),
        }
    }

    if drifted.is_empty() {
        SourceFinding {
            code: "QF-SYNC-001".to_string(),
            name: "Declarative Manifest & Codegen Sync".to_string(),
            passed: true,
            message: "All generated XR manifests, engine configs, and bootstrap files match quest_forge.yaml".to_string(),
            remediation: None,
        }
    } else {
        SourceFinding {
            code: "QF-SYNC-001".to_string(),
            name: "Declarative Manifest & Codegen Sync".to_string(),
            passed: false,
            message: format!(
                "Out-of-sync or missing generated files: {}",
                drifted.join(", ")
            ),
            remediation: Some(
                "Run `quest sync` or `quest doctor --fix` to synchronize generated files with quest_forge.yaml."
                    .to_string(),
            ),
        }
    }
}

fn check_source_vrc(project_dir: &Path, cfg: &QuestforgeConfig) -> SourceFinding {
    let manifest_rel = match cfg.project.engine {
        EngineKind::Unity => "Assets/Plugins/Android/AndroidManifest.xml",
        EngineKind::Unreal => "Build/Android/AndroidManifest.xml",
    };
    let manifest_path = project_dir.join(manifest_rel);
    let Ok(xml) = fs::read_to_string(&manifest_path) else {
        return SourceFinding {
            code: "QF-VRC-001".to_string(),
            name: "Pre-Build Source VRC & ABI Gate".to_string(),
            passed: false,
            message: format!("Missing `{manifest_rel}`"),
            remediation: Some(
                "Run `quest sync` to generate a VRC-compliant AndroidManifest.xml.".to_string(),
            ),
        };
    };

    let mut issues = Vec::new();
    let expected_pkg = format!("package=\"{}\"", cfg.project.package_id);
    if !xml.contains(&expected_pkg) {
        issues.push(format!("manifest missing `{expected_pkg}`"));
    }
    if !xml.contains("android.hardware.vr.headtracking") {
        issues.push("missing `android.hardware.vr.headtracking` feature".to_string());
    }
    if !xml.contains("com.oculus.intent.category.VR") {
        issues.push("missing `com.oculus.intent.category.VR` launch category".to_string());
    }
    if xml.contains("android:debuggable=\"true\"") {
        issues.push(
            "`android:debuggable=\"true\"` is forbidden in VRC release manifests".to_string(),
        );
    }

    for forbidden_abi in ["armeabi-v7a", "x86"] {
        let abi_dir = project_dir
            .join("Assets")
            .join("Plugins")
            .join("Android")
            .join("libs")
            .join(forbidden_abi);
        if abi_dir.exists() {
            issues.push(format!(
                "32-bit plugin directory `{}` is forbidden (Quest requires arm64-v8a)",
                abi_dir.display()
            ));
        }
    }

    if issues.is_empty() {
        SourceFinding {
            code: "QF-VRC-001".to_string(),
            name: "Pre-Build Source VRC & ABI Gate".to_string(),
            passed: true,
            message: format!(
                "`{manifest_rel}` satisfies Meta VRC package, headtracking, VR intent, and 64-bit ABI rules"
            ),
            remediation: None,
        }
    } else {
        SourceFinding {
            code: "QF-VRC-001".to_string(),
            name: "Pre-Build Source VRC & ABI Gate".to_string(),
            passed: false,
            message: issues.join("; "),
            remediation: Some("Run `quest sync` to regenerate VRC-compliant manifest files and remove 32-bit native plugins.".to_string()),
        }
    }
}

fn check_xr_perf_patterns(project_dir: &Path) -> SourceFinding {
    let mut source_files = Vec::new();
    for root in ["Assets/Scripts", "Source", "src"] {
        collect_source_files(&project_dir.join(root), &mut source_files);
    }

    let mut violations = Vec::new();
    for file in source_files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };
        let in_frame_loop = content.contains("void Update(")
            || content.contains("void LateUpdate(")
            || content.contains("TickComponent(");
        if !in_frame_loop {
            continue;
        }
        for pattern in [
            "Camera.main",
            "FindObjectOfType",
            "FindObjectsOfType",
            "GameObject.Find(",
            "Resources.Load(",
        ] {
            if content.contains(pattern) {
                let rel = file.strip_prefix(project_dir).unwrap_or(&file);
                violations.push(format!(
                    "{} uses `{pattern}` inside a per-frame loop",
                    rel.display()
                ));
            }
        }
    }

    if violations.is_empty() {
        SourceFinding {
            code: "QF-PERF-001".to_string(),
            name: "XR Frame-Budget Static Linter".to_string(),
            passed: true,
            message: "No per-frame allocation or scene-lookup anti-patterns detected in XR scripts"
                .to_string(),
            remediation: None,
        }
    } else {
        SourceFinding {
            code: "QF-PERF-001".to_string(),
            name: "XR Frame-Budget Static Linter".to_string(),
            passed: false,
            message: violations.join("; "),
            remediation: Some(
                "Cache component and camera references in `Awake()` / `Start()` / `BeginPlay()` instead of querying inside `Update()` or `TickComponent()`."
                    .to_string(),
            ),
        }
    }
}

fn collect_source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, out);
        } else if matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("cs" | "cpp" | "h")
        ) {
            out.push(path);
        }
    }
}

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n")
}

fn ensure_target_writable(target_dir: &Path, force: bool) -> Result<(), InitError> {
    if !target_dir.exists() {
        fs::create_dir_all(target_dir).map_err(|source| InitError::Io {
            path: target_dir.to_path_buf(),
            source,
        })?;
        return Ok(());
    }
    if force {
        return Ok(());
    }
    let mut entries = fs::read_dir(target_dir).map_err(|source| InitError::Io {
        path: target_dir.to_path_buf(),
        source,
    })?;
    if entries.next().is_some() {
        return Err(InitError::TargetNotEmpty {
            path: target_dir.to_path_buf(),
        });
    }
    Ok(())
}

fn clone_template(url: &str, target_dir: &Path) -> Result<(), InitError> {
    let out = Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(target_dir)
        .output()
        .map_err(|err| InitError::TemplateClone {
            url: url.to_string(),
            message: err.to_string(),
        })?;
    if !out.status.success() {
        return Err(InitError::TemplateClone {
            url: url.to_string(),
            message: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }
    Ok(())
}

fn write_file_checked(path: &Path, content: &str, force: bool) -> Result<(), InitError> {
    if path.exists() && !force {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| InitError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, content).map_err(|source| InitError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn gitignore_for(engine: EngineKind) -> String {
    match engine {
        EngineKind::Unity => "\
[Ll]ibrary/
[Tt]emp/
[Oo]bj/
[Bb]uild/
[Bb]uilds/
[Ll]ogs/
[Uu]serSettings/
*.apk
*.aab
*.csproj
*.sln
"
        .to_string(),
        EngineKind::Unreal => "\
Binaries/
DerivedDataCache/
Intermediate/
Saved/
*.apk
*.aab
*.sln
*.VC.db
"
        .to_string(),
    }
}

fn gitattributes_for_xr() -> String {
    "\
* text=auto eol=lf
*.fbx filter=lfs diff=lfs merge=lfs -text
*.png filter=lfs diff=lfs merge=lfs -text
*.wav filter=lfs diff=lfs merge=lfs -text
*.apk filter=lfs diff=lfs merge=lfs -text
*.asset filter=lfs diff=lfs merge=lfs -text
*.uasset filter=lfs diff=lfs merge=lfs -text
*.umap filter=lfs diff=lfs merge=lfs -text
"
    .to_string()
}

fn unity_packages_manifest(sdk: &SdkConfig) -> String {
    let meta_ver = sdk.meta_xr_sdk.as_deref().unwrap_or("68.0.0");
    if sdk.openxr {
        format!(
            "{{\n  \"dependencies\": {{\n    \"com.meta.xr.sdk.all\": \"{meta_ver}\",\n    \"com.unity.xr.openxr\": \"1.10.0\"\n  }}\n}}\n"
        )
    } else {
        format!(
            "{{\n  \"dependencies\": {{\n    \"com.meta.xr.sdk.all\": \"{meta_ver}\"\n  }}\n}}\n"
        )
    }
}

fn unreal_uproject_json(engine_ver: &str, sdk: &SdkConfig) -> String {
    let openxr_enabled = if sdk.openxr { "true" } else { "false" };
    format!(
        "{{\n  \"FileVersion\": 3,\n  \"EngineAssociation\": \"{engine_ver}\",\n  \"Plugins\": [\n    {{ \"Name\": \"OpenXR\", \"Enabled\": {openxr_enabled} }},\n    {{ \"Name\": \"MetaXR\", \"Enabled\": true }}\n  ]\n}}\n"
    )
}

fn generate_android_manifest_xml(cfg: &QuestforgeConfig) -> String {
    let supported_devices = cfg
        .targets
        .devices
        .iter()
        .map(|d| match d {
            QuestDevice::Quest2 => "quest2",
            QuestDevice::Quest3 => "quest3",
            QuestDevice::Quest3s => "quest3s",
            QuestDevice::QuestPro => "questpro",
        })
        .collect::<Vec<_>>()
        .join("|");

    let mut extra_tags = String::new();
    if cfg.xr_features.input_modes.contains(&InputMode::Hands) {
        extra_tags.push_str(
            "    <uses-permission android:name=\"com.oculus.permission.HAND_TRACKING\" />\n    <uses-feature android:name=\"oculus.software.handtracking\" android:required=\"false\" />\n",
        );
    }
    if cfg.xr_features.passthrough {
        extra_tags.push_str(
            "    <uses-feature android:name=\"com.oculus.feature.PASSTHROUGH\" android:required=\"true\" />\n",
        );
    }
    if cfg.xr_features.spatial_anchors {
        extra_tags.push_str(
            "    <uses-permission android:name=\"com.oculus.permission.USE_ANCHOR_API\" />\n",
        );
    }

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!-- AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND. -->\n<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"\n    package=\"{pkg}\">\n    <uses-sdk android:minSdkVersion=\"{min_sdk}\" android:targetSdkVersion=\"{target_sdk}\" />\n    <uses-feature android:name=\"android.hardware.vr.headtracking\" android:required=\"true\" android:version=\"1\" />\n{extra_tags}    <application android:label=\"{name}\" android:debuggable=\"false\">\n        <meta-data android:name=\"com.oculus.supportedDevices\" android:value=\"{supported_devices}\" />\n        <activity android:name=\"com.unity3d.player.UnityPlayerActivity\"\n            android:launchMode=\"singleTask\"\n            android:exported=\"true\">\n            <intent-filter>\n                <action android:name=\"android.intent.action.MAIN\" />\n                <category android:name=\"android.intent.category.LAUNCHER\" />\n                <category android:name=\"com.oculus.intent.category.VR\" />\n            </intent-filter>\n        </activity>\n    </application>\n</manifest>\n",
        pkg = cfg.project.package_id,
        min_sdk = cfg.targets.min_sdk,
        target_sdk = cfg.targets.target_sdk,
        name = cfg.project.name,
    )
}

fn foveation_label(level: FoveationLevel) -> &'static str {
    match level {
        FoveationLevel::Off => "off",
        FoveationLevel::Low => "low",
        FoveationLevel::Medium => "medium",
        FoveationLevel::High => "high",
        FoveationLevel::Dynamic => "dynamic",
    }
}

fn generate_unity_bootstrap_cs(cfg: &QuestforgeConfig) -> String {
    let hands_enabled = cfg.xr_features.input_modes.contains(&InputMode::Hands);
    let controllers_enabled = cfg
        .xr_features
        .input_modes
        .contains(&InputMode::Controllers);
    let fov = foveation_label(cfg.xr_features.foveated_rendering);
    format!(
        "// AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND.\nusing UnityEngine;\n\nnamespace QuestForge.Generated\n{{\n    public static class QuestForgeBootstrap\n    {{\n        public const int TargetRefreshRateHz = {hz};\n        public const bool PassthroughEnabled = {passthrough};\n        public const bool SpatialAnchorsEnabled = {anchors};\n        public const bool HandTrackingEnabled = {hands};\n        public const bool ControllersEnabled = {controllers};\n        public const string FoveationLevel = \"{fov}\";\n\n        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]\n        private static void ApplyQuestForgeProfile()\n        {{\n            Application.targetFrameRate = TargetRefreshRateHz;\n            Debug.Log($\"[QuestForge] Profile initialized: {{TargetRefreshRateHz}}Hz, foveation={{FoveationLevel}}, passthrough={{PassthroughEnabled}}\");\n        }}\n    }}\n}}\n",
        hz = cfg.xr_features.refresh_rate_hz,
        passthrough = cfg.xr_features.passthrough,
        anchors = cfg.xr_features.spatial_anchors,
        hands = hands_enabled,
        controllers = controllers_enabled,
    )
}

fn generate_unreal_default_engine_ini(cfg: &QuestforgeConfig) -> String {
    let fov = foveation_label(cfg.xr_features.foveated_rendering);
    format!(
        "; AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND.\n[/Script/Engine.Engine]\n+ActiveGameNameRedirects=(OldGameName=\"TP_VirtualRealityBP\",NewGameName=\"/Script/{name}\")\n\n[/Script/AndroidRuntimeSettings.AndroidRuntimeSettings]\nbBuildForArm64=True\nbSupportsVulkan=True\nMinSDKVersion={min_sdk}\nTargetSDKVersion={target_sdk}\nPackageName={pkg}\n\n[SystemSettings]\nr.Mobile.MultiView=1\nt.MaxFPS={hz}\nr.VRS.FoveationLevel={fov}\nqf.Passthrough={passthrough}\nqf.SpatialAnchors={anchors}\n",
        name = cfg.project.name,
        min_sdk = cfg.targets.min_sdk,
        target_sdk = cfg.targets.target_sdk,
        pkg = cfg.project.package_id,
        hz = cfg.xr_features.refresh_rate_hz,
        passthrough = cfg.xr_features.passthrough,
        anchors = cfg.xr_features.spatial_anchors,
    )
}

fn generate_unreal_bootstrap_h(cfg: &QuestforgeConfig) -> String {
    let fov = foveation_label(cfg.xr_features.foveated_rendering);
    format!(
        "// AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND.\n#pragma once\n\nnamespace QuestForgeGenerated\n{{\n    constexpr int kTargetRefreshRateHz = {hz};\n    constexpr bool kPassthroughEnabled = {passthrough};\n    constexpr bool kSpatialAnchorsEnabled = {anchors};\n    constexpr const char* kFoveationLevel = \"{fov}\";\n}}\n",
        hz = cfg.xr_features.refresh_rate_hz,
        passthrough = cfg.xr_features.passthrough,
        anchors = cfg.xr_features.spatial_anchors,
    )
}

fn unity_build_script_cs() -> String {
    r#"using System;
using System.Linq;
using UnityEditor;
using UnityEditor.Build.Reporting;

namespace QuestForge
{
    public static class Build
    {
        public static void BuildQuestApk()
        {
            var args = Environment.GetCommandLineArgs();
            var outPath = "Build/quest-app.apk";
            for (var i = 0; i < args.Length - 1; i++)
            {
                if (args[i] == "-questOutputApk")
                {
                    outPath = args[i + 1];
                    break;
                }
            }

            EditorUserBuildSettings.SwitchActiveBuildTarget(BuildTargetGroup.Android, BuildTarget.Android);
            PlayerSettings.SetScriptingBackend(BuildTargetGroup.Android, ScriptingImplementation.IL2CPP);
            PlayerSettings.Android.targetArchitectures = AndroidArchitecture.ARM64;

            var scenes = EditorBuildSettings.scenes.Where(s => s.enabled).Select(s => s.path).ToArray();
            var report = BuildPipeline.BuildPlayer(scenes, outPath, BuildTarget.Android, BuildOptions.None);
            if (report.summary.result != BuildResult.Succeeded)
            {
                throw new Exception($"QuestForge Android build failed: {report.summary.result}");
            }
        }
    }
}
"#
    .to_string()
}

fn minimal_vocabulary_toml() -> String {
    "\
[folders]
source = \"src\"
tests  = \"tests\"
companions = [\"docs\", \"tests\", \"scripts\", \"configs\"]
"
    .to_string()
}

fn starter_ci_sh() -> String {
    "\
#!/usr/bin/env bash
set -euo pipefail

quest setup --project-dir .
quest sync --project-dir .
quest check --project-dir .
quest doctor --ci --project-dir .
"
    .to_string()
}

fn starter_app_toml(name: &str) -> String {
    format!("name = \"{name}\"\n")
}

fn security_md() -> String {
    "\
# Security Policy

Report security vulnerabilities privately to the platform security team.
"
    .to_string()
}

fn onboarding_md(name: &str) -> String {
    format!(
        "\
# {name} Onboarding

1. Install Quest toolchain binaries: `quest setup`
2. Synchronize declarative XR manifests & codegen: `quest sync`
3. Run fast static XR & VRC source checks: `quest check`
4. Validate your workstation: `quest doctor`
5. Run the local CI gate: `bash scripts/ci.sh`
"
    )
}

fn readme_md(name: &str, engine: EngineKind) -> String {
    let engine_label = match engine {
        EngineKind::Unity => "Unity",
        EngineKind::Unreal => "Unreal Engine",
    };
    format!(
        "\
# {name}

Meta Quest / Horizon OS project ({engine_label}), configured declaratively via `quest_forge.yaml`.

- Run `quest sync` after editing `quest_forge.yaml` to update `AndroidManifest.xml` and XR bootstrap code.
- Run `quest check` for sub-second static VRC and frame-budget verification.
- Run `quest doctor` to verify your local environment.
"
    )
}
