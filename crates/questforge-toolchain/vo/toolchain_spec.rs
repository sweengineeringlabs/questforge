//! Toolchain provisioning, SHA-256 verification, and cache management.

use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};

use questforge_config::QuestforgeConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::ToolchainError;

const DEFAULT_ADB_VERSION: &str = "35.0.2";
const DEFAULT_CMDLINE_TOOLS_VERSION: &str = "11076708";

/// Supported host operating system and architecture pair.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformTarget {
    /// OS label (`windows`, `linux`, `macos`).
    pub os: String,
    /// Architecture label (`x64`, `arm64`).
    pub arch: String,
}

impl PlatformTarget {
    /// Detects the current host platform.
    pub fn current() -> Result<Self, ToolchainError> {
        let os = match std::env::consts::OS {
            "windows" => "windows",
            "linux" => "linux",
            "macos" => "macos",
            other => {
                return Err(ToolchainError::UnsupportedPlatform {
                    os: other.to_string(),
                    arch: std::env::consts::ARCH.to_string(),
                });
            }
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x64",
            "aarch64" => "arm64",
            other => {
                return Err(ToolchainError::UnsupportedPlatform {
                    os: os.to_string(),
                    arch: other.to_string(),
                });
            }
        };
        if os == "windows" && arch != "x64" {
            return Err(ToolchainError::UnsupportedPlatform {
                os: os.to_string(),
                arch: arch.to_string(),
            });
        }
        Ok(Self {
            os: os.to_string(),
            arch: arch.to_string(),
        })
    }

    /// Returns `<os>-<arch>` cache key segment.
    #[must_use]
    pub fn cache_key(&self) -> String {
        format!("{}-{}", self.os, self.arch)
    }
}

/// Supported Quest toolchain binaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    /// Android `platform-tools` (`adb`).
    Adb,
    /// Android `cmdline-tools` (`sdkmanager`).
    CmdlineTools,
    /// Meta Horizon Store upload CLI (`ovr-platform-util`).
    OvrPlatformUtil,
}

impl ToolKind {
    /// Returns the canonical cache directory name for this tool.
    #[must_use]
    pub const fn dir_name(self) -> &'static str {
        match self {
            Self::Adb => "adb",
            Self::CmdlineTools => "cmdline-tools",
            Self::OvrPlatformUtil => "ovr-platform-util",
        }
    }

    /// Returns the primary executable relative path inside the tool's install directory.
    #[must_use]
    pub fn binary_rel_path(self, os: &str) -> &'static str {
        match (self, os) {
            (Self::Adb, "windows") => "platform-tools/adb.exe",
            (Self::Adb, _) => "platform-tools/adb",
            (Self::CmdlineTools, "windows") => "cmdline-tools/bin/sdkmanager.bat",
            (Self::CmdlineTools, _) => "cmdline-tools/bin/sdkmanager",
            (Self::OvrPlatformUtil, "windows") => "ovr-platform-util.exe",
            (Self::OvrPlatformUtil, _) => "ovr-platform-util",
        }
    }
}

/// Downloadable archive specification for a single tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolArtifactSpec {
    /// Tool identifier.
    pub tool: ToolKind,
    /// Pinned version string.
    pub version: String,
    /// Download URL.
    pub url: String,
    /// Optional expected lowercase hex SHA-256 digest.
    pub sha256: Option<String>,
}

/// Options controlling `quest setup` execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupOptions {
    /// Optional path to directory containing `quest_forge.yaml`.
    pub project_dir: Option<PathBuf>,
    /// Override `adb` (`platform-tools`) version.
    pub adb_version: Option<String>,
    /// Expected SHA-256 hex digest for `adb`.
    pub adb_sha256: Option<String>,
    /// Override Android `cmdline-tools` version.
    pub cmdline_tools_version: Option<String>,
    /// Expected SHA-256 hex digest for `cmdline-tools`.
    pub cmdline_tools_sha256: Option<String>,
    /// Override `ovr-platform-util` version.
    pub ovr_version: Option<String>,
    /// Optional explicit download URL for `ovr-platform-util`.
    pub ovr_url: Option<String>,
    /// Optional expected SHA-256 for `ovr-platform-util`.
    pub ovr_sha256: Option<String>,
    /// Override cache root (defaults to `~/.questforge/cache`).
    pub cache_dir: Option<PathBuf>,
    /// Skip `cmdline-tools` if false.
    pub include_cmdline_tools: bool,
    /// Enforce zero-trust SHA-256 verification on every downloaded archive (default `true`).
    pub require_checksum: bool,
}

impl Default for SetupOptions {
    fn default() -> Self {
        Self {
            project_dir: None,
            adb_version: None,
            adb_sha256: None,
            cmdline_tools_version: None,
            cmdline_tools_sha256: None,
            ovr_version: None,
            ovr_url: None,
            ovr_sha256: None,
            cache_dir: None,
            include_cmdline_tools: false,
            require_checksum: true,
        }
    }
}

/// Result for an individual installed or cached tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledTool {
    /// Tool identifier.
    pub tool: ToolKind,
    /// Resolved version.
    pub version: String,
    /// Directory containing extracted tool files.
    pub install_dir: PathBuf,
    /// Path to the primary executable.
    pub binary_path: PathBuf,
    /// True if reused from cache without downloading.
    pub cache_hit: bool,
}

/// Aggregate report emitted by `quest setup`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupReport {
    /// Host platform target.
    pub platform: PlatformTarget,
    /// Cache root used.
    pub cache_root: PathBuf,
    /// Installed or cached tools.
    pub tools: Vec<InstalledTool>,
}

/// Resolves, downloads, verifies, and caches Quest toolchain binaries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToolchainProvisioner;

impl ToolchainProvisioner {
    /// Resolves default cache directory (`~/.questforge/cache`).
    pub fn default_cache_dir() -> Result<PathBuf, ToolchainError> {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
            .ok_or_else(|| ToolchainError::Io {
                path: PathBuf::from("~"),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "HOME / USERPROFILE not set",
                ),
            })?;
        Ok(home.join(".questforge").join("cache"))
    }

    /// Resolves the list of tool artifact specs from options and optional `quest_forge.yaml`.
    pub fn resolve_specs(
        opts: &SetupOptions,
        platform: &PlatformTarget,
    ) -> Result<Vec<ToolArtifactSpec>, ToolchainError> {
        let cfg = if let Some(dir) = &opts.project_dir {
            let manifest = dir.join("quest_forge.yaml");
            if manifest.exists() {
                Some(QuestforgeConfig::load_from_dir(dir)?)
            } else {
                None
            }
        } else {
            None
        };

        let adb_version = opts
            .adb_version
            .clone()
            .or_else(|| cfg.as_ref().map(|c| c.toolchain.metavr_cli.adb.clone()))
            .unwrap_or_else(|| DEFAULT_ADB_VERSION.to_string());

        let adb_sha256 = opts.adb_sha256.clone().or_else(|| {
            cfg.as_ref()
                .and_then(|c| c.toolchain.metavr_cli.adb_sha256.clone())
        });

        let mut specs = vec![ToolArtifactSpec {
            tool: ToolKind::Adb,
            version: adb_version.clone(),
            url: default_adb_url(platform, &adb_version),
            sha256: adb_sha256,
        }];

        if opts.include_cmdline_tools || opts.cmdline_tools_version.is_some() {
            let cmd_ver = opts
                .cmdline_tools_version
                .clone()
                .unwrap_or_else(|| DEFAULT_CMDLINE_TOOLS_VERSION.to_string());
            let cmd_sha = opts.cmdline_tools_sha256.clone().or_else(|| {
                cfg.as_ref()
                    .and_then(|c| c.toolchain.metavr_cli.cmdline_tools_sha256.clone())
            });
            specs.push(ToolArtifactSpec {
                tool: ToolKind::CmdlineTools,
                version: cmd_ver.clone(),
                url: default_cmdline_tools_url(platform, &cmd_ver),
                sha256: cmd_sha,
            });
        }

        let ovr_version = opts.ovr_version.clone().or_else(|| {
            cfg.as_ref()
                .and_then(|c| c.toolchain.metavr_cli.ovr_platform_util.clone())
        });

        if let Some(version) = ovr_version {
            let url = opts
                .ovr_url
                .clone()
                .or_else(|| default_ovr_url(platform))
                .ok_or_else(|| ToolchainError::MissingSpec {
                    tool: ToolKind::OvrPlatformUtil.dir_name().to_string(),
                    version: version.clone(),
                    reason: "provide --ovr-url and --ovr-sha256 for ovr-platform-util".to_string(),
                })?;
            let ovr_sha = opts
                .ovr_sha256
                .clone()
                .or_else(|| {
                    cfg.as_ref()
                        .and_then(|c| c.toolchain.metavr_cli.ovr_sha256.clone())
                })
                .or_else(|| default_ovr_sha256(platform));
            specs.push(ToolArtifactSpec {
                tool: ToolKind::OvrPlatformUtil,
                version,
                url,
                sha256: ovr_sha,
            });
        }

        Ok(specs)
    }

    /// Provisions all resolved tools using live HTTP downloads.
    pub fn provision(opts: &SetupOptions) -> Result<SetupReport, ToolchainError> {
        let platform = PlatformTarget::current()?;
        let specs = Self::resolve_specs(opts, &platform)?;
        let cache_root = match &opts.cache_dir {
            Some(dir) => dir.clone(),
            None => Self::default_cache_dir()?,
        };
        Self::provision_with_fetcher_policy(
            platform,
            &cache_root,
            &specs,
            opts.require_checksum,
            download_bytes,
        )
    }

    /// Provisions the given specs using a custom byte fetcher under zero-trust checksum policy.
    pub fn provision_with_fetcher<F>(
        platform: PlatformTarget,
        cache_root: &Path,
        specs: &[ToolArtifactSpec],
        fetcher: F,
    ) -> Result<SetupReport, ToolchainError>
    where
        F: Fn(&str) -> Result<Vec<u8>, ToolchainError>,
    {
        Self::provision_with_fetcher_policy(platform, cache_root, specs, true, fetcher)
    }

    /// Provisions the given specs with configurable checksum enforcement.
    pub fn provision_with_fetcher_policy<F>(
        platform: PlatformTarget,
        cache_root: &Path,
        specs: &[ToolArtifactSpec],
        require_checksum: bool,
        fetcher: F,
    ) -> Result<SetupReport, ToolchainError>
    where
        F: Fn(&str) -> Result<Vec<u8>, ToolchainError>,
    {
        let mut installed = Vec::with_capacity(specs.len());
        for spec in specs {
            if require_checksum && spec.sha256.as_deref().is_none_or(|s| s.trim().is_empty()) {
                return Err(ToolchainError::MissingChecksum {
                    tool: spec.tool.dir_name().to_string(),
                    version: spec.version.clone(),
                });
            }

            let install_dir = cache_root
                .join(platform.cache_key())
                .join(spec.tool.dir_name())
                .join(&spec.version);
            let binary_path = install_dir.join(spec.tool.binary_rel_path(&platform.os));

            if binary_path.exists() {
                installed.push(InstalledTool {
                    tool: spec.tool,
                    version: spec.version.clone(),
                    install_dir,
                    binary_path,
                    cache_hit: true,
                });
                continue;
            }

            let bytes = match fetcher(&spec.url) {
                Ok(b) => b,
                Err(ToolchainError::Download { message, .. })
                    if spec.tool == ToolKind::Adb && message.contains("404") =>
                {
                    let fallback = default_adb_latest_url(&platform);
                    fetcher(&fallback)?
                }
                Err(err) => return Err(err),
            };
            if let Some(expected) = &spec.sha256 {
                verify_sha256(spec.tool, &spec.version, expected, &bytes)?;
            }

            extract_zip_atomic(&bytes, &install_dir)?;
            installed.push(InstalledTool {
                tool: spec.tool,
                version: spec.version.clone(),
                install_dir,
                binary_path,
                cache_hit: false,
            });
        }

        Ok(SetupReport {
            platform,
            cache_root: cache_root.to_path_buf(),
            tools: installed,
        })
    }
}

fn verify_sha256(
    tool: ToolKind,
    version: &str,
    expected: &str,
    bytes: &[u8],
) -> Result<(), ToolchainError> {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let actual = hex::encode(hasher.finalize());
    if !actual.eq_ignore_ascii_case(expected.trim()) {
        return Err(ToolchainError::ChecksumMismatch {
            tool: tool.dir_name().to_string(),
            version: version.to_string(),
            expected: expected.trim().to_ascii_lowercase(),
            actual,
        });
    }
    Ok(())
}

fn extract_zip_atomic(bytes: &[u8], install_dir: &Path) -> Result<(), ToolchainError> {
    let parent = install_dir.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| ToolchainError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    let staging = parent.join(format!(
        ".staging-{}",
        install_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tool")
    ));
    if staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    fs::create_dir_all(&staging).map_err(|source| ToolchainError::Io {
        path: staging.clone(),
        source,
    })?;

    let result = extract_zip_into(bytes, &staging);
    if let Err(err) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(err);
    }

    if install_dir.exists() {
        fs::remove_dir_all(install_dir).map_err(|source| ToolchainError::Io {
            path: install_dir.to_path_buf(),
            source,
        })?;
    }
    fs::rename(&staging, install_dir).map_err(|source| ToolchainError::Io {
        path: install_dir.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn extract_zip_into(bytes: &[u8], dest: &Path) -> Result<(), ToolchainError> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|err| ToolchainError::Extract {
        path: dest.to_path_buf(),
        message: err.to_string(),
    })?;

    for idx in 0..archive.len() {
        let mut entry = archive
            .by_index(idx)
            .map_err(|err| ToolchainError::Extract {
                path: dest.to_path_buf(),
                message: err.to_string(),
            })?;
        let Some(rel_path) = entry.enclosed_name() else {
            return Err(ToolchainError::Extract {
                path: dest.to_path_buf(),
                message: "archive contains unsafe path".to_string(),
            });
        };
        let out_path = dest.join(rel_path);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|source| ToolchainError::Io {
                path: out_path,
                source,
            })?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).map_err(|source| ToolchainError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            let mut out_file =
                fs::File::create(&out_path).map_err(|source| ToolchainError::Io {
                    path: out_path.clone(),
                    source,
                })?;
            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .map_err(|source| ToolchainError::Io {
                    path: out_path.clone(),
                    source,
                })?;
            out_file
                .write_all(&buf)
                .map_err(|source| ToolchainError::Io {
                    path: out_path,
                    source,
                })?;
        }
    }
    Ok(())
}

fn download_bytes(url: &str) -> Result<Vec<u8>, ToolchainError> {
    let resp = ureq::get(url)
        .call()
        .map_err(|err| ToolchainError::Download {
            url: url.to_string(),
            message: err.to_string(),
        })?;
    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|err| ToolchainError::Download {
            url: url.to_string(),
            message: err.to_string(),
        })?;
    Ok(bytes)
}

fn default_adb_url(platform: &PlatformTarget, version: &str) -> String {
    if version.eq_ignore_ascii_case("latest") {
        return default_adb_latest_url(platform);
    }
    let os_tag = match platform.os.as_str() {
        "windows" => "windows",
        "macos" => "darwin",
        _ => "linux",
    };
    format!("https://dl.google.com/android/repository/platform-tools_r{version}-{os_tag}.zip")
}

fn default_adb_latest_url(platform: &PlatformTarget) -> String {
    let os_tag = match platform.os.as_str() {
        "windows" => "windows",
        "macos" => "darwin",
        _ => "linux",
    };
    format!("https://dl.google.com/android/repository/platform-tools-latest-{os_tag}.zip")
}

fn default_cmdline_tools_url(platform: &PlatformTarget, version: &str) -> String {
    let os_tag = match platform.os.as_str() {
        "windows" => "win",
        "macos" => "mac",
        _ => "linux",
    };
    format!(
        "https://dl.google.com/android/repository/commandlinetools-{os_tag}-{version}_latest.zip"
    )
}

fn default_ovr_url(platform: &PlatformTarget) -> Option<String> {
    if platform.os == "windows" {
        Some("https://www.oculus.com/download_app/?id=1076686279105243".to_string())
    } else {
        None
    }
}

fn default_ovr_sha256(platform: &PlatformTarget) -> Option<String> {
    if platform.os == "windows" {
        Some("97c3407c38955efb04c9d2e1860c58fea0b64be08860ff8a6591b2411b88b70a".to_string())
    } else {
        None
    }
}
