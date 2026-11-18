//! Integration tests for `questforge-toolchain`.

use std::cell::Cell;
use std::fs;
use std::io::{Cursor, Write};

use questforge_toolchain::{
    PlatformTarget, SetupOptions, ToolArtifactSpec, ToolKind, ToolchainError, ToolchainProvisioner,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

fn build_test_zip(rel_path: &str, content: &[u8]) -> (Vec<u8>, String) {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut writer = ZipWriter::new(&mut buf);
        assert!(writer
            .start_file(rel_path, SimpleFileOptions::default())
            .is_ok());
        assert!(writer.write_all(content).is_ok());
        assert!(writer.finish().is_ok());
    }
    let bytes = buf.into_inner();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hex::encode(hasher.finalize());
    (bytes, digest)
}

#[test]
fn test_provision_extracts_verifies_and_caches_happy() {
    let Ok(cache_dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let platform = PlatformTarget {
        os: "linux".to_string(),
        arch: "x64".to_string(),
    };
    let (zip_bytes, sha) = build_test_zip("platform-tools/adb", b"fake-adb-binary");
    let specs = vec![ToolArtifactSpec {
        tool: ToolKind::Adb,
        version: "35.0.2".to_string(),
        url: "https://example.invalid/adb.zip".to_string(),
        sha256: Some(sha),
    }];

    let fetch_count = Cell::new(0_u32);
    let first = ToolchainProvisioner::provision_with_fetcher(
        platform.clone(),
        cache_dir.path(),
        &specs,
        |_| {
            fetch_count.set(fetch_count.get() + 1);
            Ok(zip_bytes.clone())
        },
    );
    assert!(matches!(
        first,
        Ok(ref report)
            if report.tools.len() == 1
                && !report.tools[0].cache_hit
                && report.tools[0].binary_path.exists()
    ));
    assert_eq!(fetch_count.get(), 1);

    let second = ToolchainProvisioner::provision_with_fetcher(
        platform.clone(),
        cache_dir.path(),
        &specs,
        |_| {
            fetch_count.set(fetch_count.get() + 1);
            Ok(zip_bytes.clone())
        },
    );
    assert!(matches!(
        second,
        Ok(ref report) if report.tools.len() == 1 && report.tools[0].cache_hit
    ));
    assert_eq!(fetch_count.get(), 1);
}

#[test]
fn test_provision_checksum_mismatch_aborts_cleanly_error() {
    let Ok(cache_dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let platform = PlatformTarget {
        os: "windows".to_string(),
        arch: "x64".to_string(),
    };
    let (zip_bytes, _) = build_test_zip("platform-tools/adb.exe", b"tampered");
    let specs = vec![ToolArtifactSpec {
        tool: ToolKind::Adb,
        version: "35.0.2".to_string(),
        url: "https://example.invalid/adb.zip".to_string(),
        sha256: Some(
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        ),
    }];

    let res = ToolchainProvisioner::provision_with_fetcher(
        platform.clone(),
        cache_dir.path(),
        &specs,
        |_| Ok(zip_bytes.clone()),
    );
    assert!(matches!(res, Err(ToolchainError::ChecksumMismatch { .. })));

    let target_dir = cache_dir
        .path()
        .join("windows-x64")
        .join("adb")
        .join("35.0.2");
    assert!(!target_dir.exists());
}

#[test]
fn test_provision_missing_checksum_rejected_by_zero_trust_error() {
    let Ok(cache_dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let platform = PlatformTarget {
        os: "linux".to_string(),
        arch: "x64".to_string(),
    };
    let specs = vec![ToolArtifactSpec {
        tool: ToolKind::Adb,
        version: "35.0.2".to_string(),
        url: "https://example.invalid/adb.zip".to_string(),
        sha256: None,
    }];

    let res =
        ToolchainProvisioner::provision_with_fetcher(platform, cache_dir.path(), &specs, |_| {
            Ok(Vec::new())
        });
    assert!(matches!(res, Err(ToolchainError::MissingChecksum { .. })));
}

#[test]
fn test_resolve_specs_reads_manifest_and_cli_override_edge() {
    let Ok(proj) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    assert!(fs::write(
        proj.path().join("quest_forge.yaml"),
        r#"
version: "1"
project:
  name: setup-test
  package_id: com.acme.setup
  engine: unity
toolchain:
  metavr_cli:
    adb: "34.0.5"
"#,
    )
    .is_ok());

    let platform = PlatformTarget {
        os: "macos".to_string(),
        arch: "arm64".to_string(),
    };

    let from_manifest = ToolchainProvisioner::resolve_specs(
        &SetupOptions {
            project_dir: Some(proj.path().to_path_buf()),
            ..SetupOptions::default()
        },
        &platform,
    );
    assert!(matches!(
        from_manifest,
        Ok(ref specs) if specs[0].version == "34.0.5"
    ));

    let with_override = ToolchainProvisioner::resolve_specs(
        &SetupOptions {
            project_dir: Some(proj.path().to_path_buf()),
            adb_version: Some("35.0.2".to_string()),
            ..SetupOptions::default()
        },
        &platform,
    );
    assert!(matches!(
        with_override,
        Ok(ref specs) if specs[0].version == "35.0.2"
    ));
}

#[test]
fn test_resolve_specs_wsl_uses_linux_android_tools_and_windows_ovr_exe_happy() {
    let platform = PlatformTarget {
        os: "wsl".to_string(),
        arch: "x64".to_string(),
    };
    assert!(platform.is_wsl());
    assert_eq!(
        ToolKind::OvrPlatformUtil.binary_rel_path(&platform.os),
        "ovr-platform-util.exe"
    );
    assert_eq!(
        ToolKind::Adb.binary_rel_path(&platform.os),
        "platform-tools/adb"
    );

    let specs = ToolchainProvisioner::resolve_specs(
        &SetupOptions {
            include_cmdline_tools: true,
            ovr_version: Some("latest".to_string()),
            ..SetupOptions::default()
        },
        &platform,
    );
    assert!(matches!(
        specs,
        Ok(ref list)
            if list.len() == 3
                && list[0].url.contains("-linux.zip")
                && list[1].url.contains("linux-")
                && list[2].tool == ToolKind::OvrPlatformUtil
                && list[2].url.contains("oculus.com/download_app")
                && list[2].sha256.is_some()
    ));
}

#[test]
fn test_provision_wsl_ovr_raw_exe_caches_under_wsl_platform_happy() {
    let Ok(cache_dir) = TempDir::new() else {
        panic!("failed to create tempdir");
    };
    let platform = PlatformTarget {
        os: "wsl".to_string(),
        arch: "x64".to_string(),
    };
    let exe_bytes = b"MZ-fake-ovr-platform-util-exe".to_vec();
    let sha = hex::encode(Sha256::digest(&exe_bytes));
    let specs = vec![ToolArtifactSpec {
        tool: ToolKind::OvrPlatformUtil,
        version: "211.0.0".to_string(),
        url: "https://example.invalid/ovr-platform-util.exe".to_string(),
        sha256: Some(sha),
    }];

    let res = ToolchainProvisioner::provision_with_fetcher(
        platform,
        cache_dir.path(),
        &specs,
        |_| Ok(exe_bytes.clone()),
    );
    assert!(matches!(
        res,
        Ok(ref report)
            if report.tools.len() == 1
                && report.tools[0].binary_path.ends_with("ovr-platform-util.exe")
                && report.tools[0].binary_path.exists()
    ));
}
