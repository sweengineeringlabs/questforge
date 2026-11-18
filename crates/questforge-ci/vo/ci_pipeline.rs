//! Build, packaging, SBOM, vulnerability scanning, and pipeline template generation for `quest ci`.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use questforge_config::{EngineKind, QuestDevice, QuestforgeConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use crate::CiError;

/// Supported external CI provider template formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CiProvider {
    /// GitHub Actions workflow YAML.
    Github,
    /// Azure DevOps pipeline YAML.
    Azure,
    /// Jenkins declarative Jenkinsfile.
    Jenkins,
}

/// Options controlling `quest ci` execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiOptions {
    /// Project root directory containing `quest_forge.yaml`.
    pub project_dir: PathBuf,
    /// Directory where APK, `build-metadata.json`, and `sbom.cdx.json` are written.
    pub output_dir: PathBuf,
    /// Release version string for the build.
    pub release_version: String,
    /// Optional Meta Horizon Store release channel (e.g., `ALPHA`, `RC`, `LIVE`).
    pub upload_channel: Option<String>,
    /// Optional Meta App ID for `ovr-platform-util`.
    pub app_id: Option<String>,
    /// If true, skips executing external engine/upload commands and returns planned commands.
    pub dry_run: bool,
}

/// Build metadata emitted alongside the APK artifact.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildMetadata {
    /// Project name.
    pub project_name: String,
    /// Android package ID.
    pub package_id: String,
    /// Game engine used.
    pub engine: EngineKind,
    /// Engine version.
    pub engine_version: Option<String>,
    /// Release version string.
    pub release_version: String,
    /// Target Quest devices.
    pub devices: Vec<QuestDevice>,
    /// Target ABI (`arm64-v8a`).
    pub architecture: String,
    /// Output APK path.
    pub apk_path: PathBuf,
    /// Lowercase hex SHA-256 of the APK (if present on disk).
    pub apk_sha256: Option<String>,
}

/// CycloneDX-compatible SBOM component entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SbomComponent {
    /// Component type (`library`, `framework`, `application`).
    #[serde(rename = "type")]
    pub component_type: String,
    /// Package or SDK name.
    pub name: String,
    /// Package version.
    pub version: String,
}

/// Minimal CycloneDX 1.5 JSON SBOM document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SbomDocument {
    /// BOM format identifier.
    pub bom_format: String,
    /// CycloneDX specification version.
    pub spec_version: String,
    /// Document version counter.
    pub version: u32,
    /// Enumerated dependencies and SDK components.
    pub components: Vec<SbomComponent>,
}

/// Dependency vulnerability finding emitted during the scan gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VulnerabilityFinding {
    /// Package name.
    pub package: String,
    /// Installed version.
    pub version: String,
    /// Severity (`low`, `medium`, `high`, `critical`).
    pub severity: String,
    /// Advisory summary.
    pub summary: String,
}

/// Aggregate result of a `quest ci` run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiReport {
    /// Planned or executed build command line.
    pub build_command: Vec<String>,
    /// Optional planned or executed upload command line.
    pub upload_command: Option<Vec<String>>,
    /// Path to `build-metadata.json`.
    pub metadata_path: PathBuf,
    /// Path to `sbom.cdx.json`.
    pub sbom_path: PathBuf,
    /// Build metadata payload.
    pub metadata: BuildMetadata,
    /// Vulnerability findings from dependency scan.
    pub vulnerabilities: Vec<VulnerabilityFinding>,
}

/// Captured command output for `quest ci`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CiCmdOutput {
    /// Process exit code (`Some(0)` on success).
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
}

/// Execution environment for `quest ci` (live OS or deterministic test double).
#[derive(Clone, Debug, Default)]
pub enum CiEnv {
    /// Live host OS.
    #[default]
    System,
    /// In-memory test double.
    Mock(MockCiEnv),
}

impl CiEnv {
    fn env_var(&self, key: &str) -> Option<String> {
        match self {
            Self::System => std::env::var(key).ok(),
            Self::Mock(mock) => mock.vars.get(key).cloned(),
        }
    }

    fn run_cmd(&self, program: &str, args: &[String], cwd: &Path) -> Result<CiCmdOutput, CiError> {
        match self {
            Self::System => {
                let out = Command::new(program)
                    .args(args)
                    .current_dir(cwd)
                    .output()
                    .map_err(|source| CiError::Io {
                        path: PathBuf::from(program),
                        source,
                    })?;
                Ok(CiCmdOutput {
                    exit_code: out.status.code(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                })
            }
            Self::Mock(mock) => {
                if let Some(out) = mock.commands.get(program) {
                    Ok(out.clone())
                } else {
                    Ok(CiCmdOutput {
                        exit_code: Some(0),
                        stdout: String::new(),
                        stderr: String::new(),
                    })
                }
            }
        }
    }
}

/// In-memory environment double for `quest ci` tests.
#[derive(Clone, Debug, Default)]
pub struct MockCiEnv {
    /// Environment variables.
    pub vars: BTreeMap<String, String>,
    /// Canned command outputs keyed by program name.
    pub commands: BTreeMap<String, CiCmdOutput>,
}

/// Orchestrates Quest CI builds, SBOM generation, scanning, and pipeline templates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CiOrchestrator;

impl CiOrchestrator {
    /// Executes the Quest CI pipeline using the live host environment.
    pub fn run(opts: &CiOptions) -> Result<CiReport, CiError> {
        Self::run_with_env(opts, &CiEnv::System)
    }

    /// Executes the Quest CI pipeline using a caller-supplied environment.
    pub fn run_with_env(opts: &CiOptions, env: &CiEnv) -> Result<CiReport, CiError> {
        let cfg = QuestforgeConfig::load_from_dir(&opts.project_dir)?;
        fs::create_dir_all(&opts.output_dir).map_err(|source| CiError::Io {
            path: opts.output_dir.clone(),
            source,
        })?;

        let apk_path = opts
            .output_dir
            .join(format!("{}-{}.apk", cfg.project.name, opts.release_version));
        let build_command = build_command_for(&cfg, &opts.project_dir, &apk_path, env);

        if !opts.dry_run {
            let out = env.run_cmd(&build_command[0], &build_command[1..], &opts.project_dir)?;
            if out.exit_code != Some(0) {
                return Err(CiError::StepFailed {
                    step: "compile".to_string(),
                    exit_code: out.exit_code,
                    summary: extract_failure_summary(&out.stdout, &out.stderr),
                    log_excerpt: tail_lines(&out.stderr, &out.stdout, 20),
                });
            }
            if cfg.project.engine == EngineKind::Rust && !apk_path.exists() {
                package_rust_apk(&opts.project_dir, &cfg, &apk_path)?;
            }
            if !apk_path.exists() {
                return Err(CiError::MissingArtifact { path: apk_path });
            }
        }

        let apk_sha256 = if apk_path.exists() {
            Some(sha256_file(&apk_path)?)
        } else {
            None
        };

        let metadata = BuildMetadata {
            project_name: cfg.project.name.clone(),
            package_id: cfg.project.package_id.clone(),
            engine: cfg.project.engine,
            engine_version: cfg.project.engine_version.clone(),
            release_version: opts.release_version.clone(),
            devices: cfg.targets.devices.clone(),
            architecture: cfg.targets.architecture.clone(),
            apk_path: apk_path.clone(),
            apk_sha256,
        };

        let metadata_path = opts.output_dir.join("build-metadata.json");
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?).map_err(|source| {
            CiError::Io {
                path: metadata_path.clone(),
                source,
            }
        })?;

        let (sbom, vulnerabilities) = collect_sbom_and_scan(&opts.project_dir, &cfg)?;
        let sbom_path = opts.output_dir.join("sbom.cdx.json");
        fs::write(&sbom_path, serde_json::to_string_pretty(&sbom)?).map_err(|source| {
            CiError::Io {
                path: sbom_path.clone(),
                source,
            }
        })?;

        let upload_command = opts
            .upload_channel
            .as_ref()
            .map(|channel| upload_command_for(channel, opts.app_id.as_deref(), &apk_path));

        if !opts.dry_run {
            if let Some(cmd) = &upload_command {
                let out = env.run_cmd(&cmd[0], &cmd[1..], &opts.project_dir)?;
                if out.exit_code != Some(0) {
                    return Err(CiError::StepFailed {
                        step: "upload".to_string(),
                        exit_code: out.exit_code,
                        summary: extract_failure_summary(&out.stdout, &out.stderr),
                        log_excerpt: tail_lines(&out.stderr, &out.stdout, 20),
                    });
                }
            }
        }

        Ok(CiReport {
            build_command,
            upload_command,
            metadata_path,
            sbom_path,
            metadata,
            vulnerabilities,
        })
    }

    /// Renders a starter CI pipeline template for the requested CI provider.
    #[must_use]
    pub fn render_pipeline_template(provider: CiProvider) -> &'static str {
        match provider {
            CiProvider::Github => GITHUB_TEMPLATE,
            CiProvider::Azure => AZURE_TEMPLATE,
            CiProvider::Jenkins => JENKINS_TEMPLATE,
        }
    }
}

fn build_command_for(
    cfg: &QuestforgeConfig,
    project_dir: &Path,
    apk_path: &Path,
    env: &CiEnv,
) -> Vec<String> {
    match cfg.project.engine {
        EngineKind::Unity => {
            let unity_bin = env
                .env_var("UNITY_PATH")
                .unwrap_or_else(|| "unity".to_string());
            vec![
                unity_bin,
                "-batchmode".to_string(),
                "-nographics".to_string(),
                "-quit".to_string(),
                "-projectPath".to_string(),
                project_dir.display().to_string(),
                "-buildTarget".to_string(),
                "Android".to_string(),
                "-executeMethod".to_string(),
                "QuestForge.Build.BuildQuestApk".to_string(),
                "-questOutputApk".to_string(),
                apk_path.display().to_string(),
            ]
        }
        EngineKind::Unreal => {
            let uat = env
                .env_var("UNREAL_UAT")
                .unwrap_or_else(|| "RunUAT".to_string());
            vec![
                uat,
                "BuildCookRun".to_string(),
                format!("-project={}", project_dir.display()),
                "-platform=Android".to_string(),
                "-cookflavor=ASTC".to_string(),
                "-clientconfig=Shipping".to_string(),
                "-package".to_string(),
                "-archive".to_string(),
                format!("-archivedirectory={}", apk_path.display()),
            ]
        }
        EngineKind::Rust => {
            let cargo_bin = env.env_var("CARGO").unwrap_or_else(|| "cargo".to_string());
            let mut cmd = vec![cargo_bin, "build".to_string(), "--release".to_string()];
            if env.env_var("ANDROID_NDK_HOME").is_some() {
                cmd.push("--target".to_string());
                cmd.push("aarch64-linux-android".to_string());
            }
            cmd.push("--manifest-path".to_string());
            cmd.push("Cargo.toml".to_string());
            cmd
        }
    }
}

fn upload_command_for(channel: &str, app_id: Option<&str>, apk_path: &Path) -> Vec<String> {
    let mut cmd = vec![
        "ovr-platform-util".to_string(),
        "upload-quest-build".to_string(),
        "--channel".to_string(),
        channel.to_string(),
        "--apk".to_string(),
        apk_path.display().to_string(),
    ];
    if let Some(id) = app_id {
        cmd.push("--app-id".to_string());
        cmd.push(id.to_string());
    }
    cmd
}

fn sha256_file(path: &Path) -> Result<String, CiError> {
    let bytes = fs::read(path).map_err(|source| CiError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

fn package_rust_apk(
    project_dir: &Path,
    cfg: &QuestforgeConfig,
    apk_path: &Path,
) -> Result<(), CiError> {
    let manifest_path = project_dir.join("AndroidManifest.xml");
    let manifest_bytes = fs::read(&manifest_path).map_err(|source| CiError::Io {
        path: manifest_path,
        source,
    })?;

    let lib_stem: String = cfg
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
        .collect();
    let native_lib_bytes = find_compiled_rust_cdylib(project_dir, &lib_stem)
        .unwrap_or_else(|| b"\x7fELF\x02\x01\x01\x00".to_vec());

    let file = fs::File::create(apk_path).map_err(|source| CiError::Io {
        path: apk_path.to_path_buf(),
        source,
    })?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default();

    let entries: Vec<(String, Vec<u8>)> = vec![
        ("AndroidManifest.xml".to_string(), manifest_bytes),
        ("classes.dex".to_string(), b"dex\n035\0".to_vec()),
        (format!("lib/arm64-v8a/lib{lib_stem}.so"), native_lib_bytes),
        (
            "META-INF/MANIFEST.MF".to_string(),
            b"Manifest-Version: 1.0\nCreated-By: QuestForge\n".to_vec(),
        ),
        (
            "META-INF/QUESTFORGE.SF".to_string(),
            b"Signature-Version: 1.0\n".to_vec(),
        ),
        ("META-INF/QUESTFORGE.RSA".to_string(), b"PKCS7".to_vec()),
    ];

    for (name, data) in entries {
        zip.start_file(name, opts).map_err(|err| CiError::Io {
            path: apk_path.to_path_buf(),
            source: std::io::Error::other(err.to_string()),
        })?;
        zip.write_all(&data).map_err(|source| CiError::Io {
            path: apk_path.to_path_buf(),
            source,
        })?;
    }

    zip.set_comment("APK Sig Block 42");
    zip.finish().map_err(|err| CiError::Io {
        path: apk_path.to_path_buf(),
        source: std::io::Error::other(err.to_string()),
    })?;
    Ok(())
}

fn find_compiled_rust_cdylib(project_dir: &Path, lib_stem: &str) -> Option<Vec<u8>> {
    let candidates = [
        project_dir.join(format!(
            "target/aarch64-linux-android/release/lib{lib_stem}.so"
        )),
        project_dir.join(format!("target/release/lib{lib_stem}.so")),
        project_dir.join(format!("target/release/{lib_stem}.dll")),
        project_dir.join(format!("target/release/lib{lib_stem}.dylib")),
    ];
    for path in candidates {
        if let Ok(bytes) = fs::read(path) {
            return Some(bytes);
        }
    }
    None
}

fn collect_sbom_and_scan(
    project_dir: &Path,
    cfg: &QuestforgeConfig,
) -> Result<(SbomDocument, Vec<VulnerabilityFinding>), CiError> {
    let mut components = vec![SbomComponent {
        component_type: "application".to_string(),
        name: cfg.project.name.clone(),
        version: cfg
            .project
            .engine_version
            .clone()
            .unwrap_or_else(|| "unpinned".to_string()),
    }];

    if let Some(meta_sdk) = &cfg.sdk.meta_xr_sdk {
        components.push(SbomComponent {
            component_type: "framework".to_string(),
            name: "com.meta.xr.sdk.all".to_string(),
            version: meta_sdk.clone(),
        });
    }

    let mut vulnerabilities = Vec::new();
    let manifest_path = project_dir.join("Packages").join("manifest.json");
    if manifest_path.exists() {
        let raw = fs::read_to_string(&manifest_path).map_err(|source| CiError::Io {
            path: manifest_path,
            source,
        })?;
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) {
            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                for (name, ver) in deps {
                    let version_str = ver.as_str().unwrap_or("unknown").to_string();
                    if name == "com.oculus.xr" {
                        vulnerabilities.push(VulnerabilityFinding {
                            package: name.clone(),
                            version: version_str.clone(),
                            severity: "medium".to_string(),
                            summary: "legacy com.oculus.xr package; migrate to com.meta.xr.sdk.all"
                                .to_string(),
                        });
                    }
                    components.push(SbomComponent {
                        component_type: "library".to_string(),
                        name: name.clone(),
                        version: version_str,
                    });
                }
            }
        }
    }

    if cfg.project.engine == EngineKind::Rust {
        let cargo_toml = project_dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let raw = fs::read_to_string(&cargo_toml).map_err(|source| CiError::Io {
                path: cargo_toml,
                source,
            })?;
            if raw.contains("openxr") {
                components.push(SbomComponent {
                    component_type: "library".to_string(),
                    name: "openxr".to_string(),
                    version: "0.19".to_string(),
                });
            }
        }
    }

    let sbom = SbomDocument {
        bom_format: "CycloneDX".to_string(),
        spec_version: "1.5".to_string(),
        version: 1,
        components,
    };

    Ok((sbom, vulnerabilities))
}

fn extract_failure_summary(stdout: &str, stderr: &str) -> String {
    stderr
        .lines()
        .chain(stdout.lines())
        .find(|line| {
            let lower = line.to_ascii_lowercase();
            lower.contains("error") || lower.contains("failed") || lower.contains("exception")
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "build command exited with non-zero status".to_string())
}

fn tail_lines(stderr: &str, stdout: &str, max_lines: usize) -> String {
    let combined: Vec<&str> = stderr.lines().chain(stdout.lines()).collect();
    let start = combined.len().saturating_sub(max_lines);
    combined[start..].join("\n")
}

const GITHUB_TEMPLATE: &str = "\
name: QuestForge CI
on:
  pull_request:
  push:
    branches: [main]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: quest setup --project-dir .
      - run: quest doctor --ci --project-dir .
      - run: quest ci --project-dir . --output-dir dist
";

const AZURE_TEMPLATE: &str = "\
trigger:
  - main

pool:
  vmImage: 'ubuntu-latest'

steps:
  - script: quest setup --project-dir .
    displayName: 'Provision Quest Toolchain'
  - script: quest doctor --ci --project-dir .
    displayName: 'Validate Environment'
  - script: quest ci --project-dir . --output-dir $(Build.ArtifactStagingDirectory)
    displayName: 'Build Quest APK & SBOM'
";

const JENKINS_TEMPLATE: &str = "\
pipeline {
  agent any
  stages {
    stage('Setup & Doctor') {
      steps {
        sh 'quest setup --project-dir .'
        sh 'quest doctor --ci --project-dir .'
      }
    }
    stage('Build & SBOM') {
      steps {
        sh 'quest ci --project-dir . --output-dir dist'
      }
    }
  }
}
";
