//! Environment diagnostics (`QD-001` through `QD-009`) for Quest development.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use questforge_config::{EngineKind, QuestforgeConfig};
use serde::{Deserialize, Serialize};

use crate::DoctorError;

/// Outcome of an individual `quest doctor` check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed.
    Pass,
    /// Non-fatal issue or local-only warning.
    Warn,
    /// Blocking failure.
    Fail,
}

/// Single diagnostic check result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// Stable check identifier (`QD-001` .. `QD-009`).
    pub id: String,
    /// Human-readable check name.
    pub name: String,
    /// Pass, warn, or fail status.
    pub status: CheckStatus,
    /// Observed environment details.
    pub details: String,
    /// Actionable remediation guidance when status is `warn` or `fail`.
    pub remediation: Option<String>,
}

/// Aggregate report emitted by `quest doctor`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DoctorReport {
    /// Project directory evaluated.
    pub project_dir: PathBuf,
    /// Whether strict/CI mode was enabled.
    pub ci_mode: bool,
    /// Individual check outcomes.
    pub checks: Vec<CheckResult>,
}

impl DoctorReport {
    /// Returns `true` if any check has `CheckStatus::Fail`.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|c| c.status == CheckStatus::Fail)
    }
}

/// Options controlling `quest doctor`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DoctorOptions {
    /// Project root directory (containing `quest_forge.yaml`).
    pub project_dir: PathBuf,
    /// CI mode (ignores missing physical USB headset, fails on missing signing or swartch violations).
    pub ci: bool,
    /// Strict mode (treats warnings on signing as failures).
    pub strict: bool,
}

/// Captured command output for deterministic testing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CmdOutput {
    /// True if exit code was zero.
    pub success: bool,
    /// Combined or primary stdout.
    pub stdout: String,
    /// Stderr text.
    pub stderr: String,
}

/// Environment abstraction for `quest doctor` (live OS or deterministic test double).
#[derive(Clone, Debug, Default)]
pub enum DoctorEnv {
    /// Live host operating system.
    #[default]
    System,
    /// In-memory environment double for deterministic tests.
    Mock(MockDoctorEnv),
}

impl DoctorEnv {
    /// Reads an environment variable.
    #[must_use]
    pub fn env_var(&self, key: &str) -> Option<String> {
        match self {
            Self::System => std::env::var(key).ok(),
            Self::Mock(mock) => mock.vars.get(key).cloned(),
        }
    }

    /// Executes an external command and returns its output, or `None` if not found.
    #[must_use]
    pub fn run_cmd(&self, program: &str, args: &[&str], cwd: &Path) -> Option<CmdOutput> {
        match self {
            Self::System => {
                let out = Command::new(program)
                    .args(args)
                    .current_dir(cwd)
                    .output()
                    .ok()?;
                Some(CmdOutput {
                    success: out.status.success(),
                    stdout: String::from_utf8_lossy(&out.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                })
            }
            Self::Mock(mock) => mock.commands.get(program).cloned(),
        }
    }
}

/// In-memory environment double for deterministic unit and integration tests.
#[derive(Clone, Debug, Default)]
pub struct MockDoctorEnv {
    /// Environment variables.
    pub vars: BTreeMap<String, String>,
    /// Canned command outputs keyed by program name.
    pub commands: BTreeMap<String, CmdOutput>,
}

/// Runs `QD-001` through `QD-009` against a QuestForge project directory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DoctorRunner;

impl DoctorRunner {
    /// Runs all diagnostic checks using the live host environment.
    pub fn run(opts: &DoctorOptions) -> Result<DoctorReport, DoctorError> {
        Self::run_with_env(opts, &DoctorEnv::System)
    }

    /// Runs all diagnostic checks using a caller-supplied environment.
    pub fn run_with_env(
        opts: &DoctorOptions,
        env: &DoctorEnv,
    ) -> Result<DoctorReport, DoctorError> {
        let (qd001, maybe_cfg) = check_project_config(&opts.project_dir);
        let cfg_ref = maybe_cfg.as_ref();

        let checks = vec![
            qd001,
            check_engine_installation(&opts.project_dir, cfg_ref, env),
            check_jdk(cfg_ref, env),
            check_android_sdk(cfg_ref, env),
            check_android_ndk(cfg_ref, env),
            check_xr_sdk(&opts.project_dir, cfg_ref),
            check_adb_and_device(&opts.project_dir, opts.ci, env),
            check_signing_keystore(cfg_ref, opts.ci || opts.strict, env),
            check_swartch_gate(&opts.project_dir, opts.ci, env),
        ];

        Ok(DoctorReport {
            project_dir: opts.project_dir.clone(),
            ci_mode: opts.ci,
            checks,
        })
    }
}

fn check_project_config(project_dir: &Path) -> (CheckResult, Option<QuestforgeConfig>) {
    let manifest_path = project_dir.join("quest_forge.yaml");
    if !manifest_path.exists() {
        return (
            CheckResult {
                id: "QD-001".to_string(),
                name: "Project Config".to_string(),
                status: CheckStatus::Fail,
                details: format!("missing {}", manifest_path.display()),
                remediation: Some("Run `quest init` to create quest_forge.yaml".to_string()),
            },
            None,
        );
    }

    match QuestforgeConfig::load_from_dir(project_dir) {
        Ok(cfg) => (
            CheckResult {
                id: "QD-001".to_string(),
                name: "Project Config".to_string(),
                status: CheckStatus::Pass,
                details: format!(
                    "quest_forge.yaml valid (engine: {:?}, version: {})",
                    cfg.project.engine,
                    cfg.project.engine_version.as_deref().unwrap_or("unpinned")
                ),
                remediation: None,
            },
            Some(cfg),
        ),
        Err(err) => (
            CheckResult {
                id: "QD-001".to_string(),
                name: "Project Config".to_string(),
                status: CheckStatus::Fail,
                details: err.to_string(),
                remediation: Some("Fix validation errors in quest_forge.yaml".to_string()),
            },
            None,
        ),
    }
}

fn check_engine_installation(
    _project_dir: &Path,
    cfg: Option<&QuestforgeConfig>,
    env: &DoctorEnv,
) -> CheckResult {
    let Some(cfg) = cfg else {
        return CheckResult {
            id: "QD-002".to_string(),
            name: "Game Engine".to_string(),
            status: CheckStatus::Warn,
            details: "skipped because quest_forge.yaml could not be loaded".to_string(),
            remediation: Some("Fix QD-001 first".to_string()),
        };
    };

    match cfg.project.engine {
        EngineKind::Unity => {
            let ver = cfg
                .project
                .engine_version
                .as_deref()
                .unwrap_or("2022.3.42f1");
            let found = env
                .env_var("UNITY_PATH")
                .map(PathBuf::from)
                .filter(|p| p.exists())
                .or_else(|| find_unity_editor_dir(ver));
            match found {
                Some(path) => CheckResult {
                    id: "QD-002".to_string(),
                    name: "Game Engine".to_string(),
                    status: CheckStatus::Pass,
                    details: format!("Unity {ver} found at {}", path.display()),
                    remediation: None,
                },
                None => CheckResult {
                    id: "QD-002".to_string(),
                    name: "Game Engine".to_string(),
                    status: CheckStatus::Fail,
                    details: format!("Unity {ver} (with Android Build Support & IL2CPP) not found"),
                    remediation: Some(format!(
                        "Install via Unity Hub: unity-hub -- --headless install --version {ver} -m android -m android-sdk-ndk-tools"
                    )),
                },
            }
        }
        EngineKind::Unreal => {
            let found = env
                .env_var("UNREAL_PATH")
                .map(PathBuf::from)
                .filter(|p| p.exists());
            match found {
                Some(path) => CheckResult {
                    id: "QD-002".to_string(),
                    name: "Game Engine".to_string(),
                    status: CheckStatus::Pass,
                    details: format!("Unreal Engine found at {}", path.display()),
                    remediation: None,
                },
                None => CheckResult {
                    id: "QD-002".to_string(),
                    name: "Game Engine".to_string(),
                    status: CheckStatus::Fail,
                    details: "Unreal Engine installation not found (UNREAL_PATH unset)".to_string(),
                    remediation: Some(
                        "Set UNREAL_PATH to your Unreal Engine root directory".to_string(),
                    ),
                },
            }
        }
    }
}

fn find_unity_editor_dir(version: &str) -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(format!(r"C:\Program Files\Unity\Hub\Editor\{version}")),
        PathBuf::from(format!("/Applications/Unity/Hub/Editor/{version}")),
        PathBuf::from(format!("/opt/unity/Editor/{version}")),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn check_jdk(cfg: Option<&QuestforgeConfig>, env: &DoctorEnv) -> CheckResult {
    let expected = cfg.map_or("17", |c| c.toolchain.jdk.as_str());
    let java_home = env.env_var("JAVA_HOME");
    let cmd = env.run_cmd("java", &["-version"], Path::new("."));

    match (java_home, cmd) {
        (Some(home), Some(out)) if out.success => CheckResult {
            id: "QD-003".to_string(),
            name: "JDK".to_string(),
            status: CheckStatus::Pass,
            details: format!("JAVA_HOME={home} (expected JDK {expected})"),
            remediation: None,
        },
        (None, Some(out)) if out.success => CheckResult {
            id: "QD-003".to_string(),
            name: "JDK".to_string(),
            status: CheckStatus::Warn,
            details: "java is on PATH, but JAVA_HOME is not set".to_string(),
            remediation: Some(format!("Export JAVA_HOME pointing to JDK {expected}")),
        },
        _ => CheckResult {
            id: "QD-003".to_string(),
            name: "JDK".to_string(),
            status: CheckStatus::Fail,
            details: format!("JDK {expected} not detected"),
            remediation: Some(format!("Install OpenJDK {expected} and set JAVA_HOME")),
        },
    }
}

fn check_android_sdk(cfg: Option<&QuestforgeConfig>, env: &DoctorEnv) -> CheckResult {
    let target_sdk = cfg.map_or(32, |c| c.targets.target_sdk);
    let build_tools = cfg.map_or("34.0.0", |c| c.toolchain.android_build_tools.as_str());

    let Some(sdk_root) = env
        .env_var("ANDROID_HOME")
        .or_else(|| env.env_var("ANDROID_SDK_ROOT"))
    else {
        return CheckResult {
            id: "QD-004".to_string(),
            name: "Android SDK".to_string(),
            status: CheckStatus::Fail,
            details: "ANDROID_HOME / ANDROID_SDK_ROOT is not set".to_string(),
            remediation: Some(format!(
                "Run `quest setup --cmdline-tools-version 11076708` and `sdkmanager \"platforms;android-{target_sdk}\" \"build-tools;{build_tools}\"`"
            )),
        };
    };

    let root = PathBuf::from(&sdk_root);
    let platform_dir = root.join("platforms").join(format!("android-{target_sdk}"));
    let build_tools_dir = root.join("build-tools").join(build_tools);

    if platform_dir.exists() && build_tools_dir.exists() {
        CheckResult {
            id: "QD-004".to_string(),
            name: "Android SDK".to_string(),
            status: CheckStatus::Pass,
            details: format!(
                "android-{target_sdk} and build-tools {build_tools} found under {sdk_root}"
            ),
            remediation: None,
        }
    } else {
        CheckResult {
            id: "QD-004".to_string(),
            name: "Android SDK".to_string(),
            status: CheckStatus::Fail,
            details: format!(
                "missing android-{target_sdk} or build-tools {build_tools} under {sdk_root}"
            ),
            remediation: Some(format!(
                "sdkmanager \"platforms;android-{target_sdk}\" \"build-tools;{build_tools}\""
            )),
        }
    }
}

fn check_android_ndk(cfg: Option<&QuestforgeConfig>, env: &DoctorEnv) -> CheckResult {
    let ndk_ver = cfg.map_or("23.1.7779620", |c| c.toolchain.android_ndk.as_str());

    if let Some(ndk_home) = env.env_var("ANDROID_NDK_HOME") {
        if Path::new(&ndk_home).exists() {
            return CheckResult {
                id: "QD-005".to_string(),
                name: "Android NDK".to_string(),
                status: CheckStatus::Pass,
                details: format!("ANDROID_NDK_HOME={ndk_home}"),
                remediation: None,
            };
        }
    }

    if let Some(sdk_root) = env
        .env_var("ANDROID_HOME")
        .or_else(|| env.env_var("ANDROID_SDK_ROOT"))
    {
        let ndk_dir = PathBuf::from(&sdk_root).join("ndk").join(ndk_ver);
        if ndk_dir.exists() {
            return CheckResult {
                id: "QD-005".to_string(),
                name: "Android NDK".to_string(),
                status: CheckStatus::Pass,
                details: format!("NDK {ndk_ver} found at {}", ndk_dir.display()),
                remediation: None,
            };
        }
    }

    CheckResult {
        id: "QD-005".to_string(),
        name: "Android NDK".to_string(),
        status: CheckStatus::Fail,
        details: format!("Android NDK {ndk_ver} not found"),
        remediation: Some(format!("sdkmanager \"ndk;{ndk_ver}\"")),
    }
}

fn check_xr_sdk(project_dir: &Path, cfg: Option<&QuestforgeConfig>) -> CheckResult {
    let manifest_path = project_dir.join("Packages").join("manifest.json");
    if !manifest_path.exists() {
        return CheckResult {
            id: "QD-006".to_string(),
            name: "Meta XR & OpenXR SDK".to_string(),
            status: CheckStatus::Warn,
            details: format!("{} not found", manifest_path.display()),
            remediation: Some(
                "Ensure com.meta.xr.sdk.all and com.unity.xr.openxr are installed in Packages/manifest.json"
                    .to_string(),
            ),
        };
    }

    let Ok(raw) = fs::read_to_string(&manifest_path) else {
        return CheckResult {
            id: "QD-006".to_string(),
            name: "Meta XR & OpenXR SDK".to_string(),
            status: CheckStatus::Fail,
            details: format!("unable to read {}", manifest_path.display()),
            remediation: Some("Check file permissions on Packages/manifest.json".to_string()),
        };
    };

    let has_meta = raw.contains("com.meta.xr.sdk");
    let requires_openxr = cfg.is_none_or(|c| c.sdk.openxr);
    let has_openxr = !requires_openxr || raw.contains("com.unity.xr.openxr");

    if has_meta && has_openxr {
        CheckResult {
            id: "QD-006".to_string(),
            name: "Meta XR & OpenXR SDK".to_string(),
            status: CheckStatus::Pass,
            details: "Meta XR SDK and OpenXR dependencies present".to_string(),
            remediation: None,
        }
    } else {
        CheckResult {
            id: "QD-006".to_string(),
            name: "Meta XR & OpenXR SDK".to_string(),
            status: CheckStatus::Fail,
            details: "missing com.meta.xr.sdk.* or com.unity.xr.openxr in Packages/manifest.json"
                .to_string(),
            remediation: Some(
                "Add com.meta.xr.sdk.all and com.unity.xr.openxr via Unity Package Manager"
                    .to_string(),
            ),
        }
    }
}

fn check_adb_and_device(project_dir: &Path, ci_mode: bool, env: &DoctorEnv) -> CheckResult {
    let Some(ver_out) = env.run_cmd("adb", &["version"], project_dir) else {
        return CheckResult {
            id: "QD-007".to_string(),
            name: "ADB & Headset".to_string(),
            status: CheckStatus::Fail,
            details: "adb not found on PATH".to_string(),
            remediation: Some("Run `quest setup` to install adb".to_string()),
        };
    };

    if !ver_out.success {
        return CheckResult {
            id: "QD-007".to_string(),
            name: "ADB & Headset".to_string(),
            status: CheckStatus::Fail,
            details: "adb failed to execute".to_string(),
            remediation: Some("Reinstall platform-tools via `quest setup`".to_string()),
        };
    }

    if ci_mode {
        return CheckResult {
            id: "QD-007".to_string(),
            name: "ADB & Headset".to_string(),
            status: CheckStatus::Pass,
            details: "adb available (physical headset check skipped in --ci mode)".to_string(),
            remediation: None,
        };
    }

    let dev_out = env.run_cmd("adb", &["devices"], project_dir);
    let has_authorized = dev_out.as_ref().is_some_and(|o| {
        o.stdout
            .lines()
            .skip(1)
            .any(|line| line.ends_with("\tdevice") || line.contains(" device"))
    });
    let has_unauthorized = dev_out
        .as_ref()
        .is_some_and(|o| o.stdout.lines().any(|line| line.contains("unauthorized")));

    if has_unauthorized {
        CheckResult {
            id: "QD-007".to_string(),
            name: "ADB & Headset".to_string(),
            status: CheckStatus::Fail,
            details: "connected device is unauthorized".to_string(),
            remediation: Some(
                "Put on the headset and accept the RSA USB debugging prompt".to_string(),
            ),
        }
    } else if has_authorized {
        CheckResult {
            id: "QD-007".to_string(),
            name: "ADB & Headset".to_string(),
            status: CheckStatus::Pass,
            details: "adb available and authorized device connected".to_string(),
            remediation: None,
        }
    } else {
        CheckResult {
            id: "QD-007".to_string(),
            name: "ADB & Headset".to_string(),
            status: CheckStatus::Warn,
            details: "adb available, no Quest device or emulator connected".to_string(),
            remediation: Some(
                "Connect a Quest headset in Developer Mode or boot an Android emulator".to_string(),
            ),
        }
    }
}

fn check_signing_keystore(
    cfg: Option<&QuestforgeConfig>,
    strict: bool,
    env: &DoctorEnv,
) -> CheckResult {
    let keystore_env = cfg.map_or("QUEST_KEYSTORE_PATH", |c| c.signing.keystore_env.as_str());
    let alias_env = cfg.map_or("QUEST_KEY_ALIAS", |c| c.signing.alias_env.as_str());

    let keystore = env.env_var(keystore_env);
    let alias = env.env_var(alias_env);

    match (keystore, alias) {
        (Some(path), Some(_)) if Path::new(&path).exists() => CheckResult {
            id: "QD-008".to_string(),
            name: "Signing Keystore".to_string(),
            status: CheckStatus::Pass,
            details: format!("keystore configured at {path}"),
            remediation: None,
        },
        _ => CheckResult {
            id: "QD-008".to_string(),
            name: "Signing Keystore".to_string(),
            status: if strict {
                CheckStatus::Fail
            } else {
                CheckStatus::Warn
            },
            details: format!("missing {keystore_env} or {alias_env} (or keystore file missing)"),
            remediation: Some(format!(
                "Set {keystore_env} and {alias_env} to a valid Android release keystore"
            )),
        },
    }
}

fn check_swartch_gate(project_dir: &Path, ci_mode: bool, env: &DoctorEnv) -> CheckResult {
    let vocab = if project_dir.join("config/vocabulary.toml").exists() {
        Some("config/vocabulary.toml")
    } else if project_dir.join("vocabulary.toml").exists() {
        Some("vocabulary.toml")
    } else {
        None
    };

    let Some(vocab_file) = vocab else {
        return CheckResult {
            id: "QD-009".to_string(),
            name: "Swartch Architecture Gate".to_string(),
            status: if ci_mode {
                CheckStatus::Fail
            } else {
                CheckStatus::Warn
            },
            details: "missing config/vocabulary.toml or vocabulary.toml".to_string(),
            remediation: Some(
                "Add config/vocabulary.toml via `quest init` to enable swartch checks".to_string(),
            ),
        };
    };

    match env.run_cmd("swartch", &["--vocabulary", vocab_file, "."], project_dir) {
        Some(out) if out.success => CheckResult {
            id: "QD-009".to_string(),
            name: "Swartch Architecture Gate".to_string(),
            status: CheckStatus::Pass,
            details: "swartch audit passed".to_string(),
            remediation: None,
        },
        Some(out) => CheckResult {
            id: "QD-009".to_string(),
            name: "Swartch Architecture Gate".to_string(),
            status: CheckStatus::Fail,
            details: format!("swartch audit failed: {}", out.stdout.trim()),
            remediation: Some(
                "Run `swartch --fix --dry-run` to preview remediations or `swartch --rule <code> --help`"
                    .to_string(),
            ),
        },
        None => CheckResult {
            id: "QD-009".to_string(),
            name: "Swartch Architecture Gate".to_string(),
            status: if ci_mode {
                CheckStatus::Fail
            } else {
                CheckStatus::Warn
            },
            details: "swartch CLI not found on PATH".to_string(),
            remediation: Some("Install swartch CLI and ensure it is on PATH".to_string()),
        },
    }
}
