//! Project scaffolding for Unity and Unreal Meta Quest repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use questforge_config::{
    EngineKind, MetaVrCliConfig, ProjectConfig, QuestforgeConfig, SdkConfig, SigningConfig,
    TargetsConfig, ToolchainConfig,
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

/// Scaffolds standardized Quest-ready repositories.
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
                engine_version: Some(engine_ver.clone()),
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
            signing: SigningConfig::default(),
        };

        let yaml = serde_yaml::to_string(&cfg)?;
        let mut files: Vec<(&str, String)> = vec![
            ("quest_forge.yaml", yaml),
            ("config/vocabulary.toml", minimal_vocabulary_toml()),
            ("scripts/ci.sh", starter_ci_sh()),
            ("configs/app.toml", starter_app_toml(&opts.name)),
            (".gitignore", gitignore_for(opts.engine)),
            (".gitattributes", gitattributes_for_xr()),
            ("CODEOWNERS", "* @platform-team\n".to_string()),
            ("SECURITY.md", security_md()),
            ("docs/onboarding.md", onboarding_md(&opts.name)),
            ("README.md", readme_md(&opts.name, opts.engine)),
        ];

        match opts.engine {
            EngineKind::Unity => {
                files.push((
                    "ProjectSettings/ProjectVersion.txt",
                    format!("m_EditorVersion: {engine_ver}\n"),
                ));
                files.push(("Packages/manifest.json", unity_packages_manifest()));
                files.push(("Assets/Editor/QuestForgeBuild.cs", unity_build_script_cs()));
                files.push(("src/.gitkeep", String::new()));
            }
            EngineKind::Unreal => {
                files.push((
                    "QuestApp.uproject",
                    format!("{{\n  \"FileVersion\": 3,\n  \"EngineAssociation\": \"{engine_ver}\"\n}}\n"),
                ));
                files.push(("src/.gitkeep", String::new()));
            }
        }

        let mut created_files = Vec::with_capacity(files.len());
        for (rel, content) in files {
            let dest = opts.target_dir.join(rel);
            write_file_checked(&dest, &content, opts.force)?;
            created_files.push(PathBuf::from(rel));
        }

        Ok(InitReport {
            target_dir: opts.target_dir.clone(),
            engine: opts.engine,
            created_files,
        })
    }
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

fn unity_packages_manifest() -> String {
    r#"{
  "dependencies": {
    "com.meta.xr.sdk.all": "68.0.0",
    "com.unity.xr.openxr": "1.10.0"
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
2. Validate your workstation: `quest doctor`
3. Run the local CI gate: `bash scripts/ci.sh`
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

Meta Quest / Horizon OS project ({engine_label}), configured via `quest_forge.yaml`.

Run `quest doctor` to verify your local environment.
"
    )
}
