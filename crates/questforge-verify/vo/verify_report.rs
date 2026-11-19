//! Static APK verification, Meta Horizon VRC policy checks, and scorecard reporting.

use std::fs;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use questforge_config::QuestforgeConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::VerifyError;

const MAX_SINGLE_APK_BYTES: u64 = 1_073_741_824; // 1 GiB Meta Store single-APK guideline
const APK_SIG_BLOCK_MAGIC: &[u8] = b"APK Sig Block 42";

/// Verification rule severity/outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleStatus {
    /// Policy passed.
    Pass,
    /// Non-blocking advisory warning.
    Warn,
    /// Blocking policy violation.
    Fail,
}

/// Supported compliance report output formats.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// Human-readable terminal summary.
    Cli,
    /// Structured JSON scorecard.
    Json,
    /// SARIF 2.1.0 security/compliance format.
    Sarif,
    /// Standalone HTML scorecard.
    Html,
    /// Minimal PDF 1.4 scorecard document.
    Pdf,
}

/// Individual policy check outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyFinding {
    /// Rule identifier (`QV-001` .. `QV-006`).
    pub id: String,
    /// Rule name.
    pub name: String,
    /// Pass, warn, or fail status.
    pub status: RuleStatus,
    /// Observed details from the APK.
    pub details: String,
    /// Actionable remediation guidance on warn/fail.
    pub remediation: Option<String>,
}

/// Options controlling `quest verify`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyOptions {
    /// Path to the `.apk` file to verify.
    pub apk_path: PathBuf,
    /// Optional project directory containing `quest_forge.yaml`.
    pub project_dir: Option<PathBuf>,
    /// Optional max APK size override in bytes.
    pub max_apk_bytes: Option<u64>,
}

/// Aggregate release verification scorecard.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    /// Verified APK path.
    pub apk_path: PathBuf,
    /// APK size in bytes.
    pub apk_size_bytes: u64,
    /// Lowercase hex SHA-256 of the APK.
    pub apk_sha256: String,
    /// Detected native ABIs (e.g. `arm64-v8a`, `armeabi-v7a`).
    pub abis: Vec<String>,
    /// Release readiness score (`0`..=`100`).
    pub score: u8,
    /// True if zero failing checks occurred.
    pub ready: bool,
    /// Individual rule outcomes.
    pub findings: Vec<PolicyFinding>,
}

impl VerifyReport {
    /// Returns `true` if any finding has `RuleStatus::Fail`.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.findings.iter().any(|f| f.status == RuleStatus::Fail)
    }

    /// Renders the report in the requested format (`Cli`, `Json`, `Sarif`, `Html`, or `Pdf` bytes).
    pub fn render_bytes(&self, format: ReportFormat) -> Result<Vec<u8>, VerifyError> {
        match format {
            ReportFormat::Cli => Ok(self.render_cli().into_bytes()),
            ReportFormat::Json => Ok(serde_json::to_string_pretty(self)?.into_bytes()),
            ReportFormat::Sarif => Ok(self.render_sarif()?.into_bytes()),
            ReportFormat::Html => Ok(self.render_html().into_bytes()),
            ReportFormat::Pdf => Ok(self.render_pdf()),
        }
    }

    fn render_cli(&self) -> String {
        let mut out = format!(
            "QuestForge Release Verification: {} (score: {}/100, ready: {})\n",
            self.apk_path.display(),
            self.score,
            self.ready
        );
        for f in &self.findings {
            let badge = match f.status {
                RuleStatus::Pass => "PASS",
                RuleStatus::Warn => "WARN",
                RuleStatus::Fail => "FAIL",
            };
            out.push_str(&format!("[{badge}] {} ({}): {}\n", f.id, f.name, f.details));
            if let Some(fix) = &f.remediation {
                out.push_str(&format!("       fix: {fix}\n"));
            }
        }
        out
    }

    fn render_sarif(&self) -> Result<String, VerifyError> {
        let results: Vec<serde_json::Value> = self
            .findings
            .iter()
            .filter(|f| f.status != RuleStatus::Pass)
            .map(|f| {
                let level = match f.status {
                    RuleStatus::Fail => "error",
                    RuleStatus::Warn => "warning",
                    RuleStatus::Pass => "note",
                };
                serde_json::json!({
                    "ruleId": f.id,
                    "level": level,
                    "message": {
                        "text": format!("{}: {}", f.name, f.details)
                    },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": {
                                "uri": self.apk_path.display().to_string()
                            }
                        }
                    }]
                })
            })
            .collect();

        let sarif = serde_json::json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": {
                    "driver": {
                        "name": "quest-verify",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                },
                "results": results
            }]
        });
        Ok(serde_json::to_string_pretty(&sarif)?)
    }

    fn render_html(&self) -> String {
        let mut rows = String::new();
        for f in &self.findings {
            let status = match f.status {
                RuleStatus::Pass => "pass",
                RuleStatus::Warn => "warn",
                RuleStatus::Fail => "fail",
            };
            let fix = f.remediation.as_deref().unwrap_or("-");
            rows.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                escape_html(&f.id),
                escape_html(&f.name),
                status,
                escape_html(&f.details),
                escape_html(fix)
            ));
        }

        format!(
            "<!doctype html>\n<html lang=\"en\">\n<head><meta charset=\"utf-8\"><title>QuestForge Verify Report</title></head>\n<body>\n<h1>QuestForge Release Scorecard ({}/100)</h1>\n<p>APK: {} | SHA-256: {} | Ready: {}</p>\n<table border=\"1\" cellpadding=\"6\">\n<thead><tr><th>Rule</th><th>Name</th><th>Status</th><th>Details</th><th>Remediation</th></tr></thead>\n<tbody>\n{}</tbody>\n</table>\n</body>\n</html>\n",
            self.score,
            escape_html(&self.apk_path.display().to_string()),
            escape_html(&self.apk_sha256),
            self.ready,
            rows
        )
    }

    fn render_pdf(&self) -> Vec<u8> {
        let mut lines = vec![format!(
            "QuestForge Release Scorecard: {}/100 (ready={})",
            self.score, self.ready
        )];
        for f in &self.findings {
            let status = match f.status {
                RuleStatus::Pass => "PASS",
                RuleStatus::Warn => "WARN",
                RuleStatus::Fail => "FAIL",
            };
            lines.push(format!("[{status}] {} {} - {}", f.id, f.name, f.details));
        }

        let mut stream = String::from("BT\n/F1 10 Tf\n50 750 Td\n14 TL\n");
        for (idx, line) in lines.iter().enumerate() {
            let escaped = line
                .replace('\\', "\\\\")
                .replace('(', "\\(")
                .replace(')', "\\)");
            if idx == 0 {
                stream.push_str(&format!("({escaped}) Tj\n"));
            } else {
                stream.push_str(&format!("T* ({escaped}) Tj\n"));
            }
        }
        stream.push_str("ET\n");

        let obj1 = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
        let obj2 = "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n";
        let obj3 = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n";
        let obj4 = format!(
            "4 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
            stream.len(),
            stream
        );
        let obj5 = "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n";

        let mut pdf = String::from("%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.push_str(obj1);
        let o2 = pdf.len();
        pdf.push_str(obj2);
        let o3 = pdf.len();
        pdf.push_str(obj3);
        let o4 = pdf.len();
        pdf.push_str(&obj4);
        let o5 = pdf.len();
        pdf.push_str(obj5);

        let xref_pos = pdf.len();
        pdf.push_str(&format!(
            "xref\n0 6\n0000000000 65535 f \n{o1:010} 00000 n \n{o2:010} 00000 n \n{o3:010} 00000 n \n{o4:010} 00000 n \n{o5:010} 00000 n \ntrailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n"
        ));
        pdf.into_bytes()
    }
}

/// Runs static verification and Meta Horizon VRC checks on an APK.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ApkVerifier;

impl ApkVerifier {
    /// Verifies an APK file against `VerifyOptions`.
    pub fn verify(opts: &VerifyOptions) -> Result<VerifyReport, VerifyError> {
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

        let bytes = fs::read(&opts.apk_path).map_err(|source| VerifyError::Io {
            path: opts.apk_path.clone(),
            source,
        })?;
        let apk_size_bytes = bytes.len() as u64;
        let apk_sha256 = sha256_hex(&bytes);

        let mut archive =
            ZipArchive::new(Cursor::new(&bytes)).map_err(|err| VerifyError::InvalidApk {
                path: opts.apk_path.clone(),
                message: err.to_string(),
            })?;

        let mut entries = Vec::with_capacity(archive.len());
        let mut manifest_bytes = Vec::new();
        let mut resources_arsc_compressed = false;
        for idx in 0..archive.len() {
            let mut file = archive
                .by_index(idx)
                .map_err(|err| VerifyError::InvalidApk {
                    path: opts.apk_path.clone(),
                    message: err.to_string(),
                })?;
            let name = file.name().to_string();
            if name == "resources.arsc" && file.compression() != zip::CompressionMethod::Stored {
                resources_arsc_compressed = true;
            }
            if name == "AndroidManifest.xml" {
                file.read_to_end(&mut manifest_bytes)
                    .map_err(|source| VerifyError::Io {
                        path: opts.apk_path.clone(),
                        source,
                    })?;
            }
            entries.push(name);
        }

        let abis = collect_abis(&entries);
        let manifest_text = decode_manifest_strings(&manifest_bytes);

        let findings = vec![
            check_apk_integrity(&entries),
            check_signature(&entries, &bytes),
            check_arm64_abi(&abis),
            check_vrc_manifest(&manifest_text, cfg.as_ref()),
            check_permissions(&manifest_text),
            check_apk_size(
                apk_size_bytes,
                opts.max_apk_bytes.unwrap_or(MAX_SINGLE_APK_BYTES),
            ),
            check_uncompressed_resources(resources_arsc_compressed),
            check_horizon_sdk_loaders(&entries),
        ];

        let score = compute_score(&findings);
        let ready = !findings.iter().any(|f| f.status == RuleStatus::Fail);

        Ok(VerifyReport {
            apk_path: opts.apk_path.clone(),
            apk_size_bytes,
            apk_sha256,
            abis,
            score,
            ready,
            findings,
        })
    }
}

fn check_apk_integrity(entries: &[String]) -> PolicyFinding {
    let has_manifest = entries.iter().any(|e| e == "AndroidManifest.xml");
    let has_code = entries
        .iter()
        .any(|e| e.ends_with(".dex") || e.ends_with(".so"));

    if has_manifest && has_code {
        PolicyFinding {
            id: "QV-001".to_string(),
            name: "APK Integrity".to_string(),
            status: RuleStatus::Pass,
            details: "AndroidManifest.xml and executable code present".to_string(),
            remediation: None,
        }
    } else {
        PolicyFinding {
            id: "QV-001".to_string(),
            name: "APK Integrity".to_string(),
            status: RuleStatus::Fail,
            details: "APK missing AndroidManifest.xml or compiled code (.dex/.so)".to_string(),
            remediation: Some("Rebuild APK with `quest ci`".to_string()),
        }
    }
}

fn check_signature(entries: &[String], raw_apk: &[u8]) -> PolicyFinding {
    let has_v1 = entries.iter().any(|e| e == "META-INF/MANIFEST.MF")
        && entries.iter().any(|e| {
            e.starts_with("META-INF/")
                && (e.ends_with(".RSA") || e.ends_with(".DSA") || e.ends_with(".EC"))
        });
    let has_v2_plus = raw_apk
        .windows(APK_SIG_BLOCK_MAGIC.len())
        .any(|w| w == APK_SIG_BLOCK_MAGIC);

    if has_v1 || has_v2_plus {
        PolicyFinding {
            id: "QV-002".to_string(),
            name: "Signature Validation".to_string(),
            status: RuleStatus::Pass,
            details: if has_v2_plus {
                "APK Signature Scheme v2+ block detected".to_string()
            } else {
                "JAR v1 signature detected in META-INF/".to_string()
            },
            remediation: None,
        }
    } else {
        PolicyFinding {
            id: "QV-002".to_string(),
            name: "Signature Validation".to_string(),
            status: RuleStatus::Fail,
            details: "APK is unsigned (no v1 META-INF signature or v2+ signing block)".to_string(),
            remediation: Some("Sign APK with apksigner using your release keystore".to_string()),
        }
    }
}

fn check_arm64_abi(abis: &[String]) -> PolicyFinding {
    let has_arm64 = abis.iter().any(|a| a == "arm64-v8a");
    if has_arm64 {
        PolicyFinding {
            id: "QV-003".to_string(),
            name: "64-bit ARM64 Compliance".to_string(),
            status: RuleStatus::Pass,
            details: format!("native ABIs: {}", abis.join(", ")),
            remediation: None,
        }
    } else {
        PolicyFinding {
            id: "QV-003".to_string(),
            name: "64-bit ARM64 Compliance".to_string(),
            status: RuleStatus::Fail,
            details: "missing lib/arm64-v8a/ native libraries (required for Meta Horizon Store)"
                .to_string(),
            remediation: Some("Enable IL2CPP + ARM64 in Unity/Unreal Android settings".to_string()),
        }
    }
}

fn check_vrc_manifest(manifest: &str, cfg: Option<&QuestforgeConfig>) -> PolicyFinding {
    let has_vr_intent = manifest.contains("com.oculus.intent.category.VR")
        || manifest.contains("android.hardware.vr.headtracking");
    let is_debuggable = manifest.contains("debuggable=\"true\"")
        || manifest.contains("android:debuggable=\"true\"");
    let pkg_mismatch = cfg
        .map(|c| !manifest.contains(&c.project.package_id))
        .unwrap_or(false);

    if is_debuggable {
        return PolicyFinding {
            id: "QV-004".to_string(),
            name: "Meta VRC Manifest Policy".to_string(),
            status: RuleStatus::Fail,
            details: "android:debuggable=\"true\" is forbidden for release submissions".to_string(),
            remediation: Some("Build a non-development release APK".to_string()),
        };
    }

    if pkg_mismatch {
        return PolicyFinding {
            id: "QV-004".to_string(),
            name: "Meta VRC Manifest Policy".to_string(),
            status: RuleStatus::Fail,
            details: "AndroidManifest.xml does not match project.package_id in quest_forge.yaml"
                .to_string(),
            remediation: Some("Align Android package name with quest_forge.yaml".to_string()),
        };
    }

    if has_vr_intent {
        PolicyFinding {
            id: "QV-004".to_string(),
            name: "Meta VRC Manifest Policy".to_string(),
            status: RuleStatus::Pass,
            details: "VR headtracking / Oculus VR intent metadata present; release non-debuggable"
                .to_string(),
            remediation: None,
        }
    } else {
        PolicyFinding {
            id: "QV-004".to_string(),
            name: "Meta VRC Manifest Policy".to_string(),
            status: RuleStatus::Fail,
            details: "missing com.oculus.intent.category.VR or android.hardware.vr.headtracking"
                .to_string(),
            remediation: Some(
                "Add Quest VR intent-filter and headtracking uses-feature to AndroidManifest.xml"
                    .to_string(),
            ),
        }
    }
}

fn check_permissions(manifest: &str) -> PolicyFinding {
    let sensitive = [
        "READ_EXTERNAL_STORAGE",
        "WRITE_EXTERNAL_STORAGE",
        "ACCESS_FINE_LOCATION",
        "RECORD_AUDIO",
    ];
    let found: Vec<&str> = sensitive
        .into_iter()
        .filter(|perm| manifest.contains(perm))
        .collect();

    if found.is_empty() {
        PolicyFinding {
            id: "QV-005".to_string(),
            name: "Permissions Audit".to_string(),
            status: RuleStatus::Pass,
            details: "no high-risk Android permissions detected".to_string(),
            remediation: None,
        }
    } else {
        PolicyFinding {
            id: "QV-005".to_string(),
            name: "Permissions Audit".to_string(),
            status: RuleStatus::Warn,
            details: format!("sensitive permissions present: {}", found.join(", ")),
            remediation: Some(
                "Ensure each sensitive permission is justified in Meta Store Data Use Checkup"
                    .to_string(),
            ),
        }
    }
}

fn check_apk_size(size_bytes: u64, max_bytes: u64) -> PolicyFinding {
    if size_bytes <= max_bytes {
        PolicyFinding {
            id: "QV-006".to_string(),
            name: "APK Packaging Size".to_string(),
            status: RuleStatus::Pass,
            details: format!("{size_bytes} bytes (within {max_bytes} byte limit)"),
            remediation: None,
        }
    } else {
        PolicyFinding {
            id: "QV-006".to_string(),
            name: "APK Packaging Size".to_string(),
            status: RuleStatus::Fail,
            details: format!("{size_bytes} bytes exceeds {max_bytes} byte limit"),
            remediation: Some("Split large assets into OBB/Addressables".to_string()),
        }
    }
}

fn check_uncompressed_resources(is_compressed: bool) -> PolicyFinding {
    if is_compressed {
        PolicyFinding {
            id: "QV-007".to_string(),
            name: "Uncompressed Resources Check".to_string(),
            status: RuleStatus::Fail,
            details: "resources.arsc is compressed in the APK (forbidden by Android 11+ and Meta Horizon Store)".to_string(),
            remediation: Some("Package resources.arsc with compression level 0 (ZIP_STORED) and 4-byte align via zipalign".to_string()),
        }
    } else {
        PolicyFinding {
            id: "QV-007".to_string(),
            name: "Uncompressed Resources Check".to_string(),
            status: RuleStatus::Pass,
            details: "resources.arsc is stored uncompressed (ZIP_STORED) for direct memory mapping"
                .to_string(),
            remediation: None,
        }
    }
}

fn check_horizon_sdk_loaders(entries: &[String]) -> PolicyFinding {
    let has_openxr = entries
        .iter()
        .any(|e| e == "lib/arm64-v8a/libopenxr_loader.so");
    let has_platform = entries
        .iter()
        .any(|e| e == "lib/arm64-v8a/libovrplatformloader.so");

    if has_openxr || has_platform {
        let loaders: Vec<&str> = [
            has_openxr.then_some("libopenxr_loader.so"),
            has_platform.then_some("libovrplatformloader.so"),
        ]
        .into_iter()
        .flatten()
        .collect();
        PolicyFinding {
            id: "QV-008".to_string(),
            name: "Meta Horizon / OpenXR SDK Detection".to_string(),
            status: RuleStatus::Pass,
            details: format!(
                "native VR loaders detected in arm64-v8a: {}",
                loaders.join(", ")
            ),
            remediation: None,
        }
    } else {
        PolicyFinding {
            id: "QV-008".to_string(),
            name: "Meta Horizon / OpenXR SDK Detection".to_string(),
            status: RuleStatus::Warn,
            details: "libopenxr_loader.so or libovrplatformloader.so not found in lib/arm64-v8a/".to_string(),
            remediation: Some("Bundle libopenxr_loader.so or libovrplatformloader.so in lib/arm64-v8a/ to satisfy Meta store ingestion".to_string()),
        }
    }
}

fn collect_abis(entries: &[String]) -> Vec<String> {
    let mut abis = Vec::new();
    for entry in entries {
        if let Some(rest) = entry.strip_prefix("lib/") {
            if let Some((abi, _)) = rest.split_once('/') {
                if !abi.is_empty() && !abis.iter().any(|existing| existing == abi) {
                    abis.push(abi.to_string());
                }
            }
        }
    }
    abis.sort();
    abis
}

fn decode_manifest_strings(bytes: &[u8]) -> String {
    let utf8 = String::from_utf8_lossy(bytes);
    let mut combined = utf8.into_owned();
    // Binary AXML stores string pool entries as UTF-16LE as well as UTF-8; extract ASCII runs.
    let utf16_ascii: String = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|pair| {
            if pair[1] == 0 && (pair[0].is_ascii_graphic() || pair[0] == b' ') {
                Some(char::from(pair[0]))
            } else {
                None
            }
        })
        .collect();
    if !utf16_ascii.is_empty() {
        combined.push('\n');
        combined.push_str(&utf16_ascii);
    }
    combined
}

fn compute_score(findings: &[PolicyFinding]) -> u8 {
    let mut score: i32 = 100;
    for f in findings {
        match f.status {
            RuleStatus::Pass => {}
            RuleStatus::Warn => score -= 5,
            RuleStatus::Fail => score -= 20,
        }
    }
    score.clamp(0, 100) as u8
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
