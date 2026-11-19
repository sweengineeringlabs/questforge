//! Project scaffolding, declarative code/manifest synchronization (`quest sync`),
//! and source-level XR linting (`quest check`) for Unity and Unreal Meta Quest repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use questforge_config::{
    EngineKind, FoveationLevel, InputMode, InteractableKind, LocomotionMode, MetaVrCliConfig,
    ProjectConfig, QuestDevice, QuestforgeConfig, SdkConfig, SigningConfig, TargetsConfig,
    ToolchainConfig, XrFeaturesConfig, XrSceneSpec,
};
use serde::{Deserialize, Serialize};

use crate::InitError;

const DEFAULT_UNITY_VERSION: &str = "2022.3.42f1";
const DEFAULT_UNREAL_VERSION: &str = "5.4";
const DEFAULT_RUST_VERSION: &str = "1.100.0";

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
                EngineKind::Rust => DEFAULT_RUST_VERSION.to_string(),
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
            mdm: None,
            signing: SigningConfig::default(),
            store: None,
        };

        let yaml = serde_yaml::to_string(&cfg)?;
        let starter_spec_yaml = starter_main_arena_xr_yaml();
        let starter_spec = XrSceneSpec::from_yaml_str(&starter_spec_yaml)?;
        let mut files: Vec<(String, String)> = vec![
            ("quest_forge.yaml".to_string(), yaml),
            ("specs/main_arena_xr.yaml".to_string(), starter_spec_yaml),
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

        match opts.engine {
            EngineKind::Unity => {
                files.push((
                    "Assets/Scripts/MainArenaActions.cs".to_string(),
                    starter_unity_actions_cs(),
                ));
            }
            EngineKind::Unreal => {
                files.push((
                    "Source/QuestForge/MainArenaActions.h".to_string(),
                    starter_unreal_actions_h(),
                ));
            }
            EngineKind::Rust => {
                files.push(("src/lib.rs".to_string(), starter_rust_lib_rs()));
                files.push((
                    "src/main_arena_actions.rs".to_string(),
                    starter_rust_actions_rs(),
                ));
            }
        }

        for (rel, content) in managed_sync_files(&cfg, &[starter_spec]) {
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

    /// Synchronizes declarative `quest_forge.yaml` and `specs/*_xr.yaml` settings into engine manifests,
    /// `AndroidManifest.xml`, generated runtime bootstrap files, and scene `*.gen.*` components (`quest sync`).
    pub fn sync_project(project_dir: &Path) -> Result<SyncReport, InitError> {
        let cfg = QuestforgeConfig::load_from_dir(project_dir)?;
        let specs = load_scene_specs(project_dir)?;
        let managed = managed_sync_files(&cfg, &specs);
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

    /// Runs sub-second static source checks (`QF-SPEC-001`, `QF-SYNC-001`, `QF-VRC-001`, `QF-OXR-001`, `QF-PERF-001`)
    /// on `project_dir` (`quest check`).
    pub fn check_project(project_dir: &Path) -> Result<SourceCheckReport, InitError> {
        let cfg = QuestforgeConfig::load_from_dir(project_dir)?;
        let (spec_finding, specs) = check_scene_specs(project_dir);
        let findings = vec![
            spec_finding,
            check_sync_drift(project_dir, &cfg, &specs),
            check_source_vrc(project_dir, &cfg),
            check_openxr_parity(project_dir, &cfg),
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

fn load_scene_specs(project_dir: &Path) -> Result<Vec<XrSceneSpec>, InitError> {
    let specs_dir = project_dir.join("specs");
    if !specs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    let entries = fs::read_dir(&specs_dir).map_err(|source| InitError::Io {
        path: specs_dir,
        source,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with("_xr.yaml"))
        {
            paths.push(path);
        }
    }
    paths.sort();
    let mut specs = Vec::with_capacity(paths.len());
    for path in paths {
        specs.push(XrSceneSpec::load_from_file(&path)?);
    }
    Ok(specs)
}

fn check_scene_specs(project_dir: &Path) -> (SourceFinding, Vec<XrSceneSpec>) {
    match load_scene_specs(project_dir) {
        Ok(specs) => (
            SourceFinding {
                code: "QF-SPEC-001".to_string(),
                name: "Declarative XR Scene Specs".to_string(),
                passed: true,
                message: format!(
                    "Validated {} declarative XR scene spec(s) under specs/*_xr.yaml",
                    specs.len()
                ),
                remediation: None,
            },
            specs,
        ),
        Err(err) => (
            SourceFinding {
                code: "QF-SPEC-001".to_string(),
                name: "Declarative XR Scene Specs".to_string(),
                passed: false,
                message: err.to_string(),
                remediation: Some(
                    "Fix validation errors in specs/*_xr.yaml and run `quest sync`.".to_string(),
                ),
            },
            Vec::new(),
        ),
    }
}

fn managed_sync_files(cfg: &QuestforgeConfig, specs: &[XrSceneSpec]) -> Vec<(String, String)> {
    let engine_ver =
        cfg.project
            .engine_version
            .clone()
            .unwrap_or_else(|| match cfg.project.engine {
                EngineKind::Unity => DEFAULT_UNITY_VERSION.to_string(),
                EngineKind::Unreal => DEFAULT_UNREAL_VERSION.to_string(),
                EngineKind::Rust => DEFAULT_RUST_VERSION.to_string(),
            });

    let mut files = match cfg.project.engine {
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
        EngineKind::Rust => vec![
            (
                "rust-toolchain.toml".to_string(),
                generate_rust_toolchain_toml(&engine_ver),
            ),
            (
                "Cargo.toml".to_string(),
                generate_rust_cargo_toml(cfg, &engine_ver),
            ),
            (
                "AndroidManifest.xml".to_string(),
                generate_android_manifest_xml(cfg),
            ),
            (
                "src/questforge_bootstrap_gen.rs".to_string(),
                generate_rust_bootstrap_rs(cfg),
            ),
        ],
    };

    for spec in specs {
        match cfg.project.engine {
            EngineKind::Unity => {
                files.push((
                    format!("Assets/Generated/{}XrScene.gen.cs", spec.scene),
                    generate_unity_scene_cs(spec),
                ));
            }
            EngineKind::Unreal => {
                files.push((
                    format!("Source/QuestForge/{}XrScene.gen.h", spec.scene),
                    generate_unreal_scene_h(spec),
                ));
            }
            EngineKind::Rust => {
                let snake = pascal_to_snake(&spec.scene);
                files.push((
                    format!("src/{snake}_xr_scene_gen.rs"),
                    generate_rust_scene_rs(spec),
                ));
            }
        }
    }

    files
}

fn check_sync_drift(
    project_dir: &Path,
    cfg: &QuestforgeConfig,
    specs: &[XrSceneSpec],
) -> SourceFinding {
    let expected = managed_sync_files(cfg, specs);
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
        EngineKind::Rust => "AndroidManifest.xml",
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
            || content.contains("TickComponent(")
            || content.contains("fn step_frame(")
            || content.contains("fn render_frame(");
        if !in_frame_loop {
            continue;
        }
        for pattern in [
            "Camera.main",
            "FindObjectOfType",
            "FindObjectsOfType",
            "GameObject.Find(",
            "Resources.Load(",
            "thread::sleep(",
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
            Some("cs" | "cpp" | "h" | "rs")
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
        EngineKind::Rust => "\
target/
dist/
*.apk
*.aab
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
        .map(QuestDevice::as_tag)
        .collect::<Vec<_>>()
        .join("|");

    let activity_name = match cfg.project.engine {
        EngineKind::Unity => "com.unity3d.player.UnityPlayerActivity".to_string(),
        EngineKind::Unreal => "com.epicgames.unreal.SplashActivity".to_string(),
        EngineKind::Rust => format!("{}.MainActivity", cfg.project.package_id),
    };

    let mut extra_tags = String::new();
    if cfg.xr_features.input_modes.contains(&InputMode::Hands) {
        extra_tags.push_str(
            "    <uses-permission android:name=\"com.oculus.permission.HAND_TRACKING\" />\n    <uses-feature android:name=\"oculus.software.handtracking\" android:required=\"false\" />\n    <uses-feature android:name=\"android.hardware.sensor.proximity\" android:required=\"false\" />\n",
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

    let app_id_meta = if let Some(app_id) = cfg.store.as_ref().and_then(|s| s.app_id.as_deref()) {
        format!("        <meta-data android:name=\"com.oculus.applicationId\" android:value=\"{app_id}\" />\n")
    } else {
        String::new()
    };

    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!-- AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND. -->\n<manifest xmlns:android=\"http://schemas.android.com/apk/res/android\"\n    package=\"{pkg}\"\n    android:versionCode=\"1\"\n    android:versionName=\"1.0.0\"\n    android:installLocation=\"auto\">\n    <uses-sdk android:minSdkVersion=\"{min_sdk}\" android:targetSdkVersion=\"{target_sdk}\" />\n    <uses-feature android:name=\"android.hardware.vr.headtracking\" android:required=\"true\" android:version=\"1\" />\n    <uses-feature android:glEsVersion=\"0x00030002\" android:required=\"true\" />\n{extra_tags}    <application android:label=\"{name}\" android:allowBackup=\"false\" android:debuggable=\"false\">\n{app_id_meta}        <meta-data android:name=\"com.oculus.supportedDevices\" android:value=\"{supported_devices}\" />\n        <meta-data android:name=\"com.oculus.vr.focusaware\" android:value=\"true\" />\n        <activity android:name=\"{activity_name}\"\n            android:screenOrientation=\"landscape\"\n            android:configChanges=\"density|keyboard|keyboardHidden|navigation|orientation|screenLayout|screenSize|uiMode\"\n            android:launchMode=\"singleTask\"\n            android:exported=\"true\">\n            <intent-filter>\n                <action android:name=\"android.intent.action.MAIN\" />\n                <category android:name=\"android.intent.category.LAUNCHER\" />\n                <category android:name=\"com.oculus.intent.category.VR\" />\n            </intent-filter>\n        </activity>\n    </application>\n</manifest>\n",
        pkg = cfg.project.package_id,
        min_sdk = cfg.targets.min_sdk,
        target_sdk = cfg.targets.target_sdk,
        name = cfg.project.name,
        activity_name = activity_name,
    )
}

fn required_openxr_extensions(cfg: &QuestforgeConfig) -> Vec<&'static str> {
    let mut exts = vec!["XR_FB_display_refresh_rate"];
    if cfg.xr_features.foveated_rendering != FoveationLevel::Off {
        exts.push("XR_FB_foveation");
    }
    if cfg.xr_features.passthrough {
        exts.push("XR_FB_passthrough");
    }
    if cfg.xr_features.input_modes.contains(&InputMode::Hands) {
        exts.push("XR_EXT_hand_tracking");
    }
    if cfg.xr_features.spatial_anchors {
        exts.push("XR_FB_spatial_entity");
    }
    exts
}

fn check_openxr_parity(project_dir: &Path, cfg: &QuestforgeConfig) -> SourceFinding {
    let bootstrap_rel = match cfg.project.engine {
        EngineKind::Unity => "Assets/Generated/QuestForgeBootstrap.gen.cs",
        EngineKind::Unreal => "Source/QuestForge/QuestForgeBootstrap.gen.h",
        EngineKind::Rust => "src/questforge_bootstrap_gen.rs",
    };
    let manifest_rel = match cfg.project.engine {
        EngineKind::Unity => "Assets/Plugins/Android/AndroidManifest.xml",
        EngineKind::Unreal => "Build/Android/AndroidManifest.xml",
        EngineKind::Rust => "AndroidManifest.xml",
    };

    let Ok(bootstrap) = fs::read_to_string(project_dir.join(bootstrap_rel)) else {
        return SourceFinding {
            code: "QF-OXR-001".to_string(),
            name: "OpenXR Extension & Manifest Parity".to_string(),
            passed: false,
            message: format!("Missing generated OpenXR bootstrap `{bootstrap_rel}`"),
            remediation: Some(
                "Run `quest sync` to generate OpenXR extension bindings.".to_string(),
            ),
        };
    };
    let Ok(manifest) = fs::read_to_string(project_dir.join(manifest_rel)) else {
        return SourceFinding {
            code: "QF-OXR-001".to_string(),
            name: "OpenXR Extension & Manifest Parity".to_string(),
            passed: false,
            message: format!("Missing `{manifest_rel}`"),
            remediation: Some("Run `quest sync` to generate AndroidManifest.xml.".to_string()),
        };
    };

    let mut mismatches = Vec::new();
    for ext in required_openxr_extensions(cfg) {
        if !bootstrap.contains(ext) {
            mismatches.push(format!("bootstrap missing `{ext}`"));
        }
    }
    if cfg.xr_features.passthrough && !manifest.contains("com.oculus.feature.PASSTHROUGH") {
        mismatches.push("manifest missing `com.oculus.feature.PASSTHROUGH`".to_string());
    }
    if cfg.xr_features.input_modes.contains(&InputMode::Hands)
        && !manifest.contains("com.oculus.permission.HAND_TRACKING")
    {
        mismatches.push("manifest missing `com.oculus.permission.HAND_TRACKING`".to_string());
    }
    if cfg.xr_features.spatial_anchors && !manifest.contains("com.oculus.permission.USE_ANCHOR_API")
    {
        mismatches.push("manifest missing `com.oculus.permission.USE_ANCHOR_API`".to_string());
    }

    if mismatches.is_empty() {
        SourceFinding {
            code: "QF-OXR-001".to_string(),
            name: "OpenXR Extension & Manifest Parity".to_string(),
            passed: true,
            message: format!(
                "All {} Horizon OS OpenXR extension(s) and manifest permissions are in parity",
                required_openxr_extensions(cfg).len()
            ),
            remediation: None,
        }
    } else {
        SourceFinding {
            code: "QF-OXR-001".to_string(),
            name: "OpenXR Extension & Manifest Parity".to_string(),
            passed: false,
            message: mismatches.join("; "),
            remediation: Some(
                "Run `quest sync` to align OpenXR extensions and AndroidManifest.xml.".to_string(),
            ),
        }
    }
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
    let exts = required_openxr_extensions(cfg)
        .into_iter()
        .map(|e| format!("\"{e}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "// AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND.\nusing UnityEngine;\n\nnamespace QuestForge.Generated\n{{\n    public static class QuestForgeBootstrap\n    {{\n        public const int TargetRefreshRateHz = {hz};\n        public const bool PassthroughEnabled = {passthrough};\n        public const bool SpatialAnchorsEnabled = {anchors};\n        public const bool HandTrackingEnabled = {hands};\n        public const bool ControllersEnabled = {controllers};\n        public const string FoveationLevel = \"{fov}\";\n        public static readonly string[] RequiredOpenXrExtensions = new[] {{ {exts} }};\n\n        [RuntimeInitializeOnLoadMethod(RuntimeInitializeLoadType.BeforeSceneLoad)]\n        private static void ApplyQuestForgeProfile()\n        {{\n            Application.targetFrameRate = TargetRefreshRateHz;\n            Debug.Log($\"[QuestForge] Profile initialized: {{TargetRefreshRateHz}}Hz, foveation={{FoveationLevel}}, passthrough={{PassthroughEnabled}}\");\n        }}\n    }}\n}}\n",
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
    let exts = required_openxr_extensions(cfg).join(",");
    format!(
        "// AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND.\n#pragma once\n\nnamespace QuestForgeGenerated\n{{\n    constexpr int kTargetRefreshRateHz = {hz};\n    constexpr bool kPassthroughEnabled = {passthrough};\n    constexpr bool kSpatialAnchorsEnabled = {anchors};\n    constexpr const char* kFoveationLevel = \"{fov}\";\n    constexpr const char* kRequiredOpenXrExtensions = \"{exts}\";\n}}\n",
        hz = cfg.xr_features.refresh_rate_hz,
        passthrough = cfg.xr_features.passthrough,
        anchors = cfg.xr_features.spatial_anchors,
    )
}

fn locomotion_label(mode: LocomotionMode) -> &'static str {
    match mode {
        LocomotionMode::None => "none",
        LocomotionMode::Teleport => "teleport",
        LocomotionMode::Smooth => "smooth",
    }
}

fn interactable_kind_label(kind: InteractableKind) -> &'static str {
    match kind {
        InteractableKind::Grabbable => "grabbable",
        InteractableKind::PokeButton => "poke_button",
        InteractableKind::RayPanel => "ray_panel",
        InteractableKind::SpatialAnchor => "spatial_anchor",
        InteractableKind::HandGesture => "hand_gesture",
    }
}

fn collect_scene_handlers(spec: &XrSceneSpec) -> Vec<(String, String)> {
    let mut actions = Vec::new();
    for item in &spec.interactables {
        actions.push((item.id.clone(), item.on_activate.clone()));
    }
    for panel in &spec.spatial_panels {
        for act in &panel.actions {
            actions.push((act.id.clone(), act.handler.clone()));
        }
    }
    actions
}

fn generate_unity_scene_cs(spec: &XrSceneSpec) -> String {
    let scene = &spec.scene;
    let loco = locomotion_label(spec.locomotion.mode);
    let snap = spec.locomotion.snap_turn_deg;
    let handlers = collect_scene_handlers(spec);

    let mut iface_methods = String::new();
    let mut dispatch_cases = String::new();
    for (id, method) in &handlers {
        iface_methods.push_str(&format!("        void {method}();\n"));
        dispatch_cases.push_str(&format!(
            "                case \"{id}\":\n                    _handler?.{method}();\n                    return true;\n"
        ));
    }

    let mut interactable_rows = String::new();
    for item in &spec.interactables {
        let kind = interactable_kind_label(item.kind);
        interactable_rows.push_str(&format!(
            "            new InteractableDescriptor(\"{id}\", \"{kind}\", {two_handed}, {haptic}, \"{handler}\"),\n",
            id = item.id,
            two_handed = item.two_handed,
            haptic = item.haptic_intensity,
            handler = item.on_activate,
        ));
    }

    let mut panel_rows = String::new();
    for panel in &spec.spatial_panels {
        panel_rows.push_str(&format!(
            "            new SpatialPanelDescriptor(\"{id}\", \"{title}\", {dist}, {follow}),\n",
            id = panel.id,
            title = panel.title,
            dist = panel.distance_cm,
            follow = panel.follow_head,
        ));
    }

    format!(
        "// AUTO-GENERATED BY `quest sync` FROM specs/*_xr.yaml. DO NOT EDIT BY HAND.\nusing UnityEngine;\n\nnamespace QuestForge.Generated\n{{\n    public interface I{scene}XrActions\n    {{\n{iface_methods}    }}\n\n    public readonly struct InteractableDescriptor\n    {{\n        public readonly string Id;\n        public readonly string Kind;\n        public readonly bool TwoHanded;\n        public readonly byte HapticIntensity;\n        public readonly string Handler;\n\n        public InteractableDescriptor(string id, string kind, bool twoHanded, byte hapticIntensity, string handler)\n        {{\n            Id = id;\n            Kind = kind;\n            TwoHanded = twoHanded;\n            HapticIntensity = hapticIntensity;\n            Handler = handler;\n        }}\n    }}\n\n    public readonly struct SpatialPanelDescriptor\n    {{\n        public readonly string Id;\n        public readonly string Title;\n        public readonly uint DistanceCm;\n        public readonly bool FollowHead;\n\n        public SpatialPanelDescriptor(string id, string title, uint distanceCm, bool followHead)\n        {{\n            Id = id;\n            Title = title;\n            DistanceCm = distanceCm;\n            FollowHead = followHead;\n        }}\n    }}\n\n    public sealed class {scene}XrController : MonoBehaviour\n    {{\n        public const string SceneName = \"{scene}\";\n        public const string LocomotionMode = \"{loco}\";\n        public const int SnapTurnDegrees = {snap};\n\n        public static readonly InteractableDescriptor[] Interactables = new[]\n        {{\n{interactable_rows}        }};\n\n        public static readonly SpatialPanelDescriptor[] SpatialPanels = new[]\n        {{\n{panel_rows}        }};\n\n        private I{scene}XrActions _handler;\n\n        private void Awake()\n        {{\n            _handler = GetComponent<I{scene}XrActions>();\n        }}\n\n        public void BindActions(I{scene}XrActions handler)\n        {{\n            _handler = handler;\n        }}\n\n        public bool DispatchAction(string actionId)\n        {{\n            switch (actionId)\n            {{\n{dispatch_cases}                default:\n                    return false;\n            }}\n        }}\n    }}\n}}\n"
    )
}

fn generate_unreal_scene_h(spec: &XrSceneSpec) -> String {
    let scene = &spec.scene;
    let loco = locomotion_label(spec.locomotion.mode);
    let snap = spec.locomotion.snap_turn_deg;
    let handlers = collect_scene_handlers(spec);

    let mut virtual_methods = String::new();
    let mut dispatch_branches = String::new();
    for (id, method) in &handlers {
        virtual_methods.push_str(&format!("        virtual void {method}() = 0;\n"));
        dispatch_branches.push_str(&format!(
            "        if (std::strcmp(ActionId, \"{id}\") == 0) {{ Handler.{method}(); return true; }}\n"
        ));
    }

    format!(
        "// AUTO-GENERATED BY `quest sync` FROM specs/*_xr.yaml. DO NOT EDIT BY HAND.\n#pragma once\n#include <cstring>\n\nnamespace QuestForgeGenerated\n{{\n    class I{scene}XrActions\n    {{\n    public:\n        virtual ~I{scene}XrActions() = default;\n{virtual_methods}    }};\n\n    struct F{scene}XrScene\n    {{\n        static constexpr const char* kSceneName = \"{scene}\";\n        static constexpr const char* kLocomotionMode = \"{loco}\";\n        static constexpr int kSnapTurnDegrees = {snap};\n\n        static bool DispatchAction(const char* ActionId, I{scene}XrActions& Handler)\n        {{\n{dispatch_branches}            return false;\n        }}\n    }};\n}}\n"
    )
}

fn starter_main_arena_xr_yaml() -> String {
    r#"scene: MainArena
locomotion:
  mode: teleport
  snap_turn_deg: 45
interactables:
  - id: energy_core
    kind: grabbable
    two_handed: true
    haptic_intensity: 80
    on_activate: OnEnergyCoreGrabbed
  - id: start_button
    kind: poke_button
    two_handed: false
    haptic_intensity: 50
    on_activate: OnStartButtonPoked
spatial_panels:
  - id: hud_panel
    title: "Mission HUD"
    distance_cm: 150
    follow_head: true
    actions:
      - id: reset_arena
        label: "Reset Arena"
        handler: OnResetArena
"#
    .to_string()
}

fn starter_unity_actions_cs() -> String {
    r#"using UnityEngine;
using QuestForge.Generated;

namespace QuestForge.App
{
    public sealed class MainArenaActions : MonoBehaviour, IMainArenaXrActions
    {
        public void OnEnergyCoreGrabbed()
        {
            Debug.Log("[MainArena] Energy core grabbed");
        }

        public void OnStartButtonPoked()
        {
            Debug.Log("[MainArena] Start button poked");
        }

        public void OnResetArena()
        {
            Debug.Log("[MainArena] Arena reset requested");
        }
    }
}
"#
    .to_string()
}

fn starter_unreal_actions_h() -> String {
    r#"#pragma once
#include "MainArenaXrScene.gen.h"

namespace QuestForgeApp
{
    class FMainArenaActions final : public QuestForgeGenerated::IMainArenaXrActions
    {
    public:
        void OnEnergyCoreGrabbed() override {}
        void OnStartButtonPoked() override {}
        void OnResetArena() override {}
    };
}
"#
    .to_string()
}

fn pascal_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (idx, ch) in name.chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if idx > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

fn generate_rust_toolchain_toml(rust_ver: &str) -> String {
    format!(
        "[toolchain]\nchannel = \"{rust_ver}\"\ncomponents = [\"rustfmt\", \"clippy\"]\ntargets = [\"aarch64-linux-android\"]\n"
    )
}

fn generate_rust_cargo_toml(cfg: &QuestforgeConfig, rust_ver: &str) -> String {
    let crate_name = cfg
        .project
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!(
        "# AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml.\n[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\nrust-version = \"{rust_ver}\"\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\npath = \"src/lib.rs\"\n\n[target.'cfg(target_os = \"android\")'.dependencies]\nopenxr = \"0.19\"\n\n[workspace]\n"
    )
}

fn generate_rust_bootstrap_rs(cfg: &QuestforgeConfig) -> String {
    let hands_enabled = cfg.xr_features.input_modes.contains(&InputMode::Hands);
    let controllers_enabled = cfg
        .xr_features
        .input_modes
        .contains(&InputMode::Controllers);
    let fov = foveation_label(cfg.xr_features.foveated_rendering);
    let exts = required_openxr_extensions(cfg)
        .into_iter()
        .map(|e| format!("    \"{e}\","))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "//! AUTO-GENERATED BY `quest sync` FROM quest_forge.yaml. DO NOT EDIT BY HAND.\n\n/// Target headset refresh rate in Hz.\npub const TARGET_REFRESH_RATE_HZ: u32 = {hz};\n/// True if Mixed Reality passthrough is enabled.\npub const PASSTHROUGH_ENABLED: bool = {passthrough};\n/// True if persistent spatial anchors are enabled.\npub const SPATIAL_ANCHORS_ENABLED: bool = {anchors};\n/// True if optical hand tracking is enabled.\npub const HAND_TRACKING_ENABLED: bool = {hands};\n/// True if Touch controllers are enabled.\npub const CONTROLLERS_ENABLED: bool = {controllers};\n/// Foveated rendering level.\npub const FOVEATION_LEVEL: &str = \"{fov}\";\n\n/// Meta Horizon OS OpenXR extensions required by `quest_forge.yaml`.\npub const REQUIRED_OPENXR_EXTENSIONS: &[&str] = &[\n{exts}\n];\n\n/// Resolved OpenXR session configuration passed to the Horizon OS / Simulator runtime.\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub struct OpenXrSessionConfig {{\n    /// Application display name.\n    pub app_name: &'static str,\n    /// Target display refresh rate in Hz.\n    pub refresh_rate_hz: u32,\n    /// Enable `XR_FB_passthrough`.\n    pub passthrough: bool,\n    /// Enable `XR_FB_spatial_entity`.\n    pub spatial_anchors: bool,\n    /// Enable `XR_EXT_hand_tracking`.\n    pub hand_tracking: bool,\n    /// Foveated rendering level (`XR_FB_foveation`).\n    pub foveation_level: &'static str,\n    /// Required OpenXR extension names.\n    pub extensions: &'static [&'static str],\n}}\n\n/// Builds the declarative `OpenXrSessionConfig` for this project.\n#[must_use]\npub const fn build_openxr_session_config() -> OpenXrSessionConfig {{\n    OpenXrSessionConfig {{\n        app_name: \"{app_name}\",\n        refresh_rate_hz: TARGET_REFRESH_RATE_HZ,\n        passthrough: PASSTHROUGH_ENABLED,\n        spatial_anchors: SPATIAL_ANCHORS_ENABLED,\n        hand_tracking: HAND_TRACKING_ENABLED,\n        foveation_level: FOVEATION_LEVEL,\n        extensions: REQUIRED_OPENXR_EXTENSIONS,\n    }}\n}}\n\n/// Converts an OpenXR `xrWaitFrame` predicted display period (nanoseconds) into\n/// frame delta seconds and invokes the scene's `step_frame(dt)` callback.\npub fn run_frame_step<F: FnMut(f32)>(predicted_display_period_ns: i64, mut step: F) {{\n    let fallback_dt = 1.0_f32 / (TARGET_REFRESH_RATE_HZ as f32);\n    let dt = if predicted_display_period_ns > 0 {{\n        (predicted_display_period_ns as f32) / 1_000_000_000.0_f32\n    }} else {{\n        fallback_dt\n    }};\n    step(dt);\n}}\n\n/// Creates an OpenXR `openxr::Instance` on Meta Horizon OS (`aarch64-linux-android`).\n#[cfg(target_os = \"android\")]\npub fn create_android_openxr_instance() -> Result<openxr::Instance, String> {{\n    let entry = unsafe {{ openxr::Entry::load() }}.map_err(|e| e.to_string())?;\n    let mut exts = openxr::ExtensionSet::default();\n    exts.khr_android_create_instance = true;\n    if HAND_TRACKING_ENABLED {{\n        exts.ext_hand_tracking = true;\n    }}\n    if PASSTHROUGH_ENABLED {{\n        exts.fb_passthrough = true;\n    }}\n    exts.fb_display_refresh_rate = true;\n    entry\n        .create_instance(\n            &openxr::ApplicationInfo {{\n                application_name: \"{app_name}\",\n                application_version: 1,\n                engine_name: \"QuestForge\",\n                engine_version: 1,\n                api_version: openxr::Version::new(1, 0, 0),\n            }},\n            &exts,\n            &[],\n        )\n        .map_err(|e| e.to_string())\n}}\n",
        app_name = cfg.project.name,
        hz = cfg.xr_features.refresh_rate_hz,
        passthrough = cfg.xr_features.passthrough,
        anchors = cfg.xr_features.spatial_anchors,
        hands = hands_enabled,
        controllers = controllers_enabled,
    )
}

fn generate_rust_scene_rs(spec: &XrSceneSpec) -> String {
    let scene = &spec.scene;
    let loco = locomotion_label(spec.locomotion.mode);
    let snap = spec.locomotion.snap_turn_deg;
    let handlers = collect_scene_handlers(spec);

    let mut trait_methods = String::new();
    let mut match_arms = String::new();
    for (id, method) in &handlers {
        let fn_name = pascal_to_snake(method);
        trait_methods.push_str(&format!(
            "    /// Invoked when `{id}` is activated.\n    fn {fn_name}(&mut self);\n"
        ));
        match_arms.push_str(&format!(
            "            \"{id}\" => {{\n                handler.{fn_name}();\n                true\n            }}\n"
        ));
    }

    let mut gesture_arms = String::new();
    for item in &spec.interactables {
        if item.kind == InteractableKind::HandGesture {
            if let Some(ref g) = item.gesture {
                let fn_name = pascal_to_snake(&item.on_activate);
                gesture_arms.push_str(&format!(
                    "            \"{g}\" => {{\n                handler.{fn_name}();\n                true\n            }}\n"
                ));
            }
        }
    }

    format!(
        "//! AUTO-GENERATED BY `quest sync` FROM specs/*_xr.yaml. DO NOT EDIT BY HAND.\n\n/// Strongly-typed action callbacks for the `{scene}` XR scene.\npub trait {scene}XrActions {{\n{trait_methods}}}\n\n/// Generated descriptor and action dispatcher for `{scene}`.\n#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]\npub struct {scene}XrScene;\n\nimpl {scene}XrScene {{\n    /// Canonical scene name.\n    pub const SCENE_NAME: &'static str = \"{scene}\";\n    /// Configured locomotion mode.\n    pub const LOCOMOTION_MODE: &'static str = \"{loco}\";\n    /// Snap-turn angle in degrees.\n    pub const SNAP_TURN_DEGREES: u32 = {snap};\n\n    /// Dispatches an interaction or spatial UI action ID to `handler`.\n    pub fn dispatch_action<H: {scene}XrActions>(action_id: &str, handler: &mut H) -> bool {{\n        match action_id {{\n{match_arms}            _ => false,\n        }}\n    }}\n\n    /// Dispatches a recognized hand gesture name (e.g. \"wave\", \"pinch\") to `handler`.\n    pub fn dispatch_gesture<H: {scene}XrActions>(gesture: &str, handler: &mut H) -> bool {{\n        match gesture {{\n{gesture_arms}            _ => false,\n        }}\n    }}\n}}\n"
    )
}

fn starter_rust_lib_rs() -> String {
    r#"//! Native Rust OpenXR application crate scaffolded by QuestForge.

pub mod main_arena_actions;
pub mod main_arena_xr_scene_gen;
pub mod questforge_bootstrap_gen;
"#
    .to_string()
}

fn starter_rust_actions_rs() -> String {
    r#"//! Developer-owned action handlers for the `MainArena` XR scene.

use crate::main_arena_xr_scene_gen::MainArenaXrActions;

/// Runtime state and callback handler for `MainArena`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MainArenaHandler {
    /// Count of energy core grabs.
    pub grabs: u32,
}

impl MainArenaXrActions for MainArenaHandler {
    fn on_energy_core_grabbed(&mut self) {
        self.grabs += 1;
    }

    fn on_start_button_poked(&mut self) {}

    fn on_reset_arena(&mut self) {
        self.grabs = 0;
    }
}
"#
    .to_string()
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
        EngineKind::Rust => "Native Rust OpenXR",
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
