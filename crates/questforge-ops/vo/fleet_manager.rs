//! Device discovery, fleet inventory, APK deployment/rollback, telemetry, logcat, and screenshots.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::OpsError;

/// Registered Quest device or Android emulator in the lab/fleet inventory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceRecord {
    /// ADB serial or network address (`<ip>:<port>`).
    pub serial: String,
    /// Logical device group (e.g. `qa-lab`, `canary`, `prod`).
    pub group: String,
    /// Connection state (`device`, `offline`, `unauthorized`).
    pub state: String,
    /// Detected OS/firmware build ID (`ro.build.display.id`).
    pub firmware_version: Option<String>,
    /// Battery percentage (`0`..=`100`).
    pub battery_level: Option<u8>,
    /// Battery health label (`good`, `overheat`, `dead`, etc.).
    pub battery_health: Option<String>,
    /// Currently deployed APK path.
    pub current_apk: Option<PathBuf>,
    /// Previously deployed APK path for rollback (`QO-005`).
    pub previous_apk: Option<PathBuf>,
}

/// Scheduled or executed deployment record (`QO-004`, `QO-010`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentRecord {
    /// Target device serial or group name.
    pub target: String,
    /// Deployed APK path.
    pub apk_path: PathBuf,
    /// Optional scheduled execution window (RFC 3339 or cron label).
    pub scheduled_at: Option<String>,
    /// Deployment outcome (`deployed`, `scheduled`, `rolled_back`).
    pub status: String,
    /// Serial numbers affected.
    pub devices: Vec<String>,
}

/// Persistent fleet inventory registry stored as JSON.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetInventory {
    /// Registered devices keyed by serial.
    pub devices: BTreeMap<String, DeviceRecord>,
    /// Recorded deployments and rollbacks.
    pub deployments: Vec<DeploymentRecord>,
}

impl FleetInventory {
    /// Loads inventory from `path`, returning an empty inventory if the file does not exist yet.
    pub fn load(path: &Path) -> Result<Self, OpsError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).map_err(|source| OpsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// Persists inventory to `path`, creating parent directories as needed.
    pub fn save(&self, path: &Path) -> Result<(), OpsError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| OpsError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json).map_err(|source| OpsError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// Captured `adb` command output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdbOutput {
    /// True if `adb` exited with code 0.
    pub success: bool,
    /// Raw stdout bytes (used for binary `screencap -p` as well as text).
    pub stdout: Vec<u8>,
    /// Stderr text.
    pub stderr: String,
}

/// Environment abstraction for `quest ops` (live `adb` or deterministic test double).
#[derive(Clone, Debug, Default)]
pub enum OpsEnv {
    /// Live host `adb` binary.
    #[default]
    System,
    /// In-memory `adb` mock keyed by joined argument string.
    Mock(MockOpsEnv),
}

impl OpsEnv {
    fn run_adb(&self, args: &[String]) -> Result<AdbOutput, OpsError> {
        match self {
            Self::System => {
                let out =
                    Command::new("adb")
                        .args(args)
                        .output()
                        .map_err(|source| OpsError::Io {
                            path: PathBuf::from("adb"),
                            source,
                        })?;
                Ok(AdbOutput {
                    success: out.status.success(),
                    stdout: out.stdout,
                    stderr: String::from_utf8_lossy(&out.stderr).to_string(),
                })
            }
            Self::Mock(mock) => {
                let key = args.join(" ");
                if let Some(out) = mock.responses.get(&key) {
                    Ok(out.clone())
                } else {
                    Ok(AdbOutput {
                        success: true,
                        stdout: Vec::new(),
                        stderr: String::new(),
                    })
                }
            }
        }
    }

    fn post_json(
        &self,
        url: &str,
        bearer_token: &str,
        payload: &serde_json::Value,
    ) -> Result<String, OpsError> {
        match self {
            Self::System => {
                let auth = format!("Bearer {bearer_token}");
                let resp = ureq::post(url)
                    .set("Authorization", &auth)
                    .set("Content-Type", "application/json")
                    .send_json(payload.clone())
                    .map_err(|err| OpsError::AdbFailed {
                        args: format!("POST {url}"),
                        message: err.to_string(),
                    })?;
                resp.into_string().map_err(|err| OpsError::Io {
                    path: PathBuf::from(url),
                    source: std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()),
                })
            }
            Self::Mock(mock) => Ok(mock
                .http_responses
                .get(url)
                .cloned()
                .unwrap_or_else(|| r#"{"id":"rel-mock-1","status":"published"}"#.to_string())),
        }
    }
}

/// In-memory `adb` and HTTP test double keyed by `"<arg1> <arg2> ..."` or URL.
#[derive(Clone, Debug, Default)]
pub struct MockOpsEnv {
    /// Canned `adb` responses.
    pub responses: BTreeMap<String, AdbOutput>,
    /// Canned HTTP JSON responses keyed by endpoint URL.
    pub http_responses: BTreeMap<String, String>,
}

/// Supported Enterprise XR MDM / Meta Horizon Managed Services providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MdmProvider {
    /// Meta Horizon Managed Services (HMS).
    MetaHms,
    /// ManageXR enterprise kiosk MDM.
    ManageXr,
    /// ArborXR enterprise kiosk MDM.
    ArborXr,
}

impl MdmProvider {
    /// Returns the default API endpoint for this MDM provider.
    #[must_use]
    pub const fn default_endpoint(self) -> &'static str {
        match self {
            Self::MetaHms => "https://graph.oculus.com/v1/hms/releases",
            Self::ManageXr => "https://api.managexr.com/v1/apps/releases",
            Self::ArborXr => "https://api.arborxr.com/v1/releases",
        }
    }

    /// Returns the canonical provider slug.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MetaHms => "meta-hms",
            Self::ManageXr => "managexr",
            Self::ArborXr => "arborxr",
        }
    }
}

/// Request payload for publishing a verified APK to Meta HMS or an enterprise MDM.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HmsReleaseRequest {
    /// Target MDM provider (`meta-hms`, `managexr`, `arborxr`).
    pub provider: MdmProvider,
    /// Application or organization app ID in the MDM.
    pub app_id: String,
    /// Path to the verified `.apk` artifact.
    pub apk_path: PathBuf,
    /// Release channel or track (e.g. `ALPHA`, `BETA`, `PRODUCTION`).
    pub channel: String,
    /// Target MDM device group (e.g. `qa-lab`, `training-floor`).
    pub device_group: String,
    /// Optional custom endpoint URL overriding the provider default.
    pub endpoint: Option<String>,
    /// Bearer access token for the MDM API.
    pub access_token: String,
}

/// Receipt returned after publishing an APK to Meta HMS or an enterprise MDM.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HmsReleaseReceipt {
    /// Provider slug (`meta-hms`, `managexr`, `arborxr`).
    pub provider: String,
    /// Remote release identifier returned by the MDM.
    pub release_id: String,
    /// Remote release status (`published`, `queued`, etc.).
    pub status: String,
    /// Target channel.
    pub channel: String,
    /// Target MDM device group.
    pub device_group: String,
    /// Lowercase hex SHA-256 of the published `.apk`.
    pub apk_sha256: String,
}

/// Result of a single `quest ops agent` poll-and-execute cycle (`#13`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSyncReport {
    /// Devices discovered on the lab host.
    pub discovered: usize,
    /// Scheduled deployments executed during this cycle.
    pub executed_deployments: Vec<DeploymentRecord>,
}

/// Fleet operations controller (`QO-001` through `QO-010`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FleetManager;

impl FleetManager {
    /// Discovers connected `adb` devices and upserts them into `inventory` (`QO-001`, `QO-002`).
    pub fn discover(
        inventory: &mut FleetInventory,
        default_group: &str,
        env: &OpsEnv,
    ) -> Result<Vec<DeviceRecord>, OpsError> {
        let args = vec!["devices".to_string()];
        let out = run_adb_checked(env, &args)?;
        let text = String::from_utf8_lossy(&out.stdout);
        let mut discovered = Vec::new();

        for line in text.lines().skip(1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let mut parts = trimmed.split_whitespace();
            let Some(serial) = parts.next() else {
                continue;
            };
            let state = parts.next().unwrap_or("unknown");
            let entry = inventory
                .devices
                .entry(serial.to_string())
                .or_insert_with(|| DeviceRecord {
                    serial: serial.to_string(),
                    group: default_group.to_string(),
                    state: state.to_string(),
                    firmware_version: None,
                    battery_level: None,
                    battery_health: None,
                    current_apk: None,
                    previous_apk: None,
                });
            entry.state = state.to_string();
            discovered.push(entry.clone());
        }
        Ok(discovered)
    }

    /// Registers or updates a device's group assignment (`QO-001`, `QO-009`).
    pub fn register_device(
        inventory: &mut FleetInventory,
        serial: &str,
        group: &str,
    ) -> DeviceRecord {
        let entry = inventory
            .devices
            .entry(serial.to_string())
            .or_insert_with(|| DeviceRecord {
                serial: serial.to_string(),
                group: group.to_string(),
                state: "registered".to_string(),
                firmware_version: None,
                battery_level: None,
                battery_health: None,
                current_apk: None,
                previous_apk: None,
            });
        entry.group = group.to_string();
        entry.clone()
    }

    /// Collects firmware version and battery health telemetry for `serial` (`QO-003`, `QO-008`).
    pub fn refresh_telemetry(
        inventory: &mut FleetInventory,
        serial: &str,
        env: &OpsEnv,
    ) -> Result<DeviceRecord, OpsError> {
        if !inventory.devices.contains_key(serial) {
            return Err(OpsError::UnknownTarget {
                target: serial.to_string(),
            });
        }

        let fw_args = vec![
            "-s".to_string(),
            serial.to_string(),
            "shell".to_string(),
            "getprop".to_string(),
            "ro.build.display.id".to_string(),
        ];
        let fw_out = run_adb_checked(env, &fw_args)?;
        let firmware = String::from_utf8_lossy(&fw_out.stdout).trim().to_string();

        let bat_args = vec![
            "-s".to_string(),
            serial.to_string(),
            "shell".to_string(),
            "dumpsys".to_string(),
            "battery".to_string(),
        ];
        let bat_out = run_adb_checked(env, &bat_args)?;
        let bat_text = String::from_utf8_lossy(&bat_out.stdout);
        let (level, health) = parse_dumpsys_battery(&bat_text);

        let Some(rec) = inventory.devices.get_mut(serial) else {
            return Err(OpsError::UnknownTarget {
                target: serial.to_string(),
            });
        };
        if !firmware.is_empty() {
            rec.firmware_version = Some(firmware);
        }
        rec.battery_level = level;
        rec.battery_health = health;
        Ok(rec.clone())
    }

    /// Deploys (or schedules) an APK to a device serial or group (`QO-004`, `QO-010`).
    pub fn deploy_apk(
        inventory: &mut FleetInventory,
        target: &str,
        apk_path: &Path,
        scheduled_at: Option<&str>,
        env: &OpsEnv,
    ) -> Result<DeploymentRecord, OpsError> {
        let serials = resolve_target_serials(inventory, target)?;
        let status = if scheduled_at.is_some() {
            "scheduled".to_string()
        } else {
            for serial in &serials {
                let args = vec![
                    "-s".to_string(),
                    serial.clone(),
                    "install".to_string(),
                    "-r".to_string(),
                    apk_path.display().to_string(),
                ];
                let _ = run_adb_checked(env, &args)?;
                if let Some(dev) = inventory.devices.get_mut(serial) {
                    dev.previous_apk = dev.current_apk.take();
                    dev.current_apk = Some(apk_path.to_path_buf());
                }
            }
            "deployed".to_string()
        };

        let record = DeploymentRecord {
            target: target.to_string(),
            apk_path: apk_path.to_path_buf(),
            scheduled_at: scheduled_at.map(ToString::to_string),
            status,
            devices: serials,
        };
        inventory.deployments.push(record.clone());
        Ok(record)
    }

    /// Rolls back a device or group to its previously deployed APK (`QO-005`).
    pub fn rollback(
        inventory: &mut FleetInventory,
        target: &str,
        env: &OpsEnv,
    ) -> Result<DeploymentRecord, OpsError> {
        let serials = resolve_target_serials(inventory, target)?;
        let mut rolled_apk = None;

        for serial in &serials {
            let prev = inventory
                .devices
                .get(serial)
                .and_then(|d| d.previous_apk.clone())
                .ok_or_else(|| OpsError::NoRollbackTarget {
                    serial: serial.clone(),
                })?;

            let args = vec![
                "-s".to_string(),
                serial.clone(),
                "install".to_string(),
                "-r".to_string(),
                "-d".to_string(),
                prev.display().to_string(),
            ];
            let _ = run_adb_checked(env, &args)?;
            if let Some(dev) = inventory.devices.get_mut(serial) {
                dev.current_apk = Some(prev.clone());
                dev.previous_apk = None;
            }
            rolled_apk = Some(prev);
        }

        let record = DeploymentRecord {
            target: target.to_string(),
            apk_path: rolled_apk.unwrap_or_default(),
            scheduled_at: None,
            status: "rolled_back".to_string(),
            devices: serials,
        };
        inventory.deployments.push(record.clone());
        Ok(record)
    }

    /// Captures device `logcat -d` output to `out_path` (`QO-006`).
    pub fn capture_logcat(
        serial: &str,
        out_path: &Path,
        env: &OpsEnv,
    ) -> Result<PathBuf, OpsError> {
        let args = vec![
            "-s".to_string(),
            serial.to_string(),
            "logcat".to_string(),
            "-d".to_string(),
        ];
        let out = run_adb_checked(env, &args)?;
        write_artifact(out_path, &out.stdout)?;
        Ok(out_path.to_path_buf())
    }

    /// Captures a device PNG screenshot via `exec-out screencap -p` (`QO-007`).
    pub fn capture_screenshot(
        serial: &str,
        out_path: &Path,
        env: &OpsEnv,
    ) -> Result<PathBuf, OpsError> {
        let args = vec![
            "-s".to_string(),
            serial.to_string(),
            "exec-out".to_string(),
            "screencap".to_string(),
            "-p".to_string(),
        ];
        let out = run_adb_checked(env, &args)?;
        write_artifact(out_path, &out.stdout)?;
        Ok(out_path.to_path_buf())
    }

    /// Executes one remote lab-agent sync cycle: discovers local devices and applies pending scheduled deployments.
    pub fn sync_agent_with_deployments(
        inventory: &mut FleetInventory,
        default_group: &str,
        pending: &[DeploymentRecord],
        env: &OpsEnv,
    ) -> Result<AgentSyncReport, OpsError> {
        let discovered = Self::discover(inventory, default_group, env)?.len();
        let mut executed = Vec::new();
        for dep in pending {
            if dep.status == "scheduled" && resolve_target_serials(inventory, &dep.target).is_ok() {
                let applied = Self::deploy_apk(inventory, &dep.target, &dep.apk_path, None, env)?;
                executed.push(applied);
            }
        }
        Ok(AgentSyncReport {
            discovered,
            executed_deployments: executed,
        })
    }

    /// Polls `server_url/api/deployments` over HTTP and executes matching local deployments.
    pub fn poll_server_once(
        inventory: &mut FleetInventory,
        server_url: &str,
        default_group: &str,
        env: &OpsEnv,
    ) -> Result<AgentSyncReport, OpsError> {
        let url = format!("{}/api/deployments", server_url.trim_end_matches('/'));
        let resp = ureq::get(&url).call().map_err(|err| OpsError::AdbFailed {
            args: url.clone(),
            message: err.to_string(),
        })?;
        let raw = resp.into_string().map_err(|err| OpsError::Io {
            path: PathBuf::from(&url),
            source: std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string()),
        })?;
        let payload: serde_json::Value = serde_json::from_str(&raw)?;
        let pending: Vec<DeploymentRecord> = payload
            .get("deployments")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?
            .unwrap_or_default();
        Self::sync_agent_with_deployments(inventory, default_group, &pending, env)
    }

    /// Publishes a verified APK and its SHA-256 digest to Meta HMS or an enterprise MDM (`ManageXR` / `ArborXR`).
    pub fn publish_to_mdm(
        req: &HmsReleaseRequest,
        env: &OpsEnv,
    ) -> Result<HmsReleaseReceipt, OpsError> {
        if req.access_token.trim().is_empty() {
            return Err(OpsError::AdbFailed {
                args: format!("{} push", req.provider.as_str()),
                message: "missing MDM access token".to_string(),
            });
        }
        let apk_bytes = fs::read(&req.apk_path).map_err(|source| OpsError::Io {
            path: req.apk_path.clone(),
            source,
        })?;
        let mut hasher = Sha256::new();
        hasher.update(&apk_bytes);
        let apk_sha256 = hex::encode(hasher.finalize());

        let endpoint = req
            .endpoint
            .as_deref()
            .unwrap_or_else(|| req.provider.default_endpoint());
        let payload = serde_json::json!({
            "provider": req.provider.as_str(),
            "app_id": req.app_id,
            "channel": req.channel,
            "device_group": req.device_group,
            "apk_sha256": apk_sha256,
            "apk_size_bytes": apk_bytes.len(),
        });

        let raw_resp = env.post_json(endpoint, &req.access_token, &payload)?;
        let parsed: serde_json::Value =
            serde_json::from_str(&raw_resp).unwrap_or_else(|_| serde_json::json!({}));
        let release_id = parsed
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("rel-1")
            .to_string();
        let status = parsed
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("published")
            .to_string();

        Ok(HmsReleaseReceipt {
            provider: req.provider.as_str().to_string(),
            release_id,
            status,
            channel: req.channel.clone(),
            device_group: req.device_group.clone(),
            apk_sha256,
        })
    }
}

fn run_adb_checked(env: &OpsEnv, args: &[String]) -> Result<AdbOutput, OpsError> {
    let out = env.run_adb(args)?;
    if !out.success {
        return Err(OpsError::AdbFailed {
            args: args.join(" "),
            message: out.stderr.trim().to_string(),
        });
    }
    Ok(out)
}

fn resolve_target_serials(
    inventory: &FleetInventory,
    target: &str,
) -> Result<Vec<String>, OpsError> {
    if inventory.devices.contains_key(target) {
        return Ok(vec![target.to_string()]);
    }
    let group_matches: Vec<String> = inventory
        .devices
        .values()
        .filter(|d| d.group == target)
        .map(|d| d.serial.clone())
        .collect();
    if group_matches.is_empty() {
        return Err(OpsError::UnknownTarget {
            target: target.to_string(),
        });
    }
    Ok(group_matches)
}

fn parse_dumpsys_battery(output: &str) -> (Option<u8>, Option<String>) {
    let mut level = None;
    let mut health = None;
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("level:") {
            level = val.trim().parse::<u8>().ok();
        } else if let Some(val) = trimmed.strip_prefix("health:") {
            health = Some(match val.trim() {
                "2" => "good".to_string(),
                "3" => "overheat".to_string(),
                "4" => "dead".to_string(),
                "5" => "over_voltage".to_string(),
                other => other.to_string(),
            });
        }
    }
    (level, health)
}

fn write_artifact(path: &Path, bytes: &[u8]) -> Result<(), OpsError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| OpsError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    fs::write(path, bytes).map_err(|source| OpsError::Io {
        path: path.to_path_buf(),
        source,
    })
}
