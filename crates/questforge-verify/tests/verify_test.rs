//! Integration tests for `questforge-verify`.

use std::fs;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use questforge_verify::{ApkVerifier, ReportFormat, RuleStatus, VerifyOptions};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn write_test_apk(path: &PathBuf, manifest: &str, signed: bool, arm64: bool) {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut w = ZipWriter::new(&mut buf);
        assert!(w
            .start_file("AndroidManifest.xml", SimpleFileOptions::default())
            .is_ok());
        assert!(w.write_all(manifest.as_bytes()).is_ok());

        assert!(w
            .start_file("classes.dex", SimpleFileOptions::default())
            .is_ok());
        assert!(w.write_all(b"dex\n035\0").is_ok());

        if arm64 {
            assert!(w
                .start_file("lib/arm64-v8a/libunity.so", SimpleFileOptions::default())
                .is_ok());
            assert!(w.write_all(b"\x7fELF").is_ok());
        } else {
            assert!(w
                .start_file("lib/armeabi-v7a/libunity.so", SimpleFileOptions::default())
                .is_ok());
            assert!(w.write_all(b"\x7fELF").is_ok());
        }

        if signed {
            assert!(w
                .start_file("META-INF/MANIFEST.MF", SimpleFileOptions::default())
                .is_ok());
            assert!(w.write_all(b"Manifest-Version: 1.0\n").is_ok());
            assert!(w
                .start_file("META-INF/CERT.RSA", SimpleFileOptions::default())
                .is_ok());
            assert!(w.write_all(b"SIG").is_ok());
        }
        assert!(w.finish().is_ok());
    }
    assert!(fs::write(path, buf.into_inner()).is_ok());
}

#[test]
fn test_verify_compliant_apk_passes_and_renders_all_formats_happy() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let apk = dir.path().join("release.apk");
    let manifest = r#"<manifest package="com.acme.vr"><uses-feature android:name="android.hardware.vr.headtracking"/><application><activity><intent-filter><category android:name="com.oculus.intent.category.VR"/></intent-filter></activity></application></manifest>"#;
    write_test_apk(&apk, manifest, true, true);

    let report = ApkVerifier::verify(&VerifyOptions {
        apk_path: apk,
        project_dir: None,
        max_apk_bytes: None,
    });
    assert!(matches!(
        report,
        Ok(ref r) if r.ready && r.score == 100 && !r.has_failures()
    ));

    if let Ok(r) = report {
        let json = r.render_bytes(ReportFormat::Json);
        let sarif = r.render_bytes(ReportFormat::Sarif);
        let html = r.render_bytes(ReportFormat::Html);
        let pdf = r.render_bytes(ReportFormat::Pdf);
        assert!(matches!(json, Ok(ref b) if !b.is_empty()));
        assert!(matches!(sarif, Ok(ref b) if String::from_utf8_lossy(b).contains("2.1.0")));
        assert!(matches!(html, Ok(ref b) if String::from_utf8_lossy(b).contains("<html")));
        assert!(matches!(pdf, Ok(ref b) if b.starts_with(b"%PDF-1.4")));
    }
}

#[test]
fn test_verify_unsigned_32bit_debuggable_apk_fails_error() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let apk = dir.path().join("bad.apk");
    let manifest = r#"<manifest package="com.acme.bad"><application android:debuggable="true"></application></manifest>"#;
    write_test_apk(&apk, manifest, false, false);

    let report = ApkVerifier::verify(&VerifyOptions {
        apk_path: apk,
        project_dir: None,
        max_apk_bytes: None,
    });
    assert!(matches!(
        report,
        Ok(ref r)
            if !r.ready
                && r.has_failures()
                && r.findings.iter().any(|f| f.id == "QV-002" && f.status == RuleStatus::Fail)
                && r.findings.iter().any(|f| f.id == "QV-003" && f.status == RuleStatus::Fail)
                && r.findings.iter().any(|f| f.id == "QV-004" && f.status == RuleStatus::Fail)
    ));
}

#[test]
fn test_verify_sensitive_permissions_warns_edge() {
    let Ok(dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let apk = dir.path().join("mic.apk");
    let manifest = r#"<manifest package="com.acme.mic"><uses-permission android:name="android.permission.RECORD_AUDIO"/><uses-feature android:name="android.hardware.vr.headtracking"/></manifest>"#;
    write_test_apk(&apk, manifest, true, true);

    let report = ApkVerifier::verify(&VerifyOptions {
        apk_path: apk,
        project_dir: None,
        max_apk_bytes: None,
    });
    assert!(matches!(
        report,
        Ok(ref r)
            if r.ready
                && r.score == 95
                && r.findings.iter().any(|f| f.id == "QV-005" && f.status == RuleStatus::Warn)
    ));
}
