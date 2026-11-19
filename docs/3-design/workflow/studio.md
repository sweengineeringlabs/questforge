# Studio Workflow (`--role studio`)

**Audience**: XR Game Studios and Unity / Unreal / Native Rust OpenXR developers

**Target Persona:** XR Game Studios building for Meta Quest with **Unity (`engine: unity`)**, **Unreal Engine (`engine: unreal`)**, or **Native Rust OpenXR (`engine: rust`)**.

---

## Table of Contents

- [1. Studio Inner-Loop Flow](#1-studio-inner-loop-flow)
- [2. Declarative XR Codegen Architecture (`quest sync` & `quest check`)](#2-declarative-xr-codegen-architecture-quest-sync--quest-check)
- [3. End-to-End Studio Sequence](#3-end-to-end-studio-sequence)

---

## 1. Studio Inner-Loop Flow

```mermaid
flowchart LR
    A["1. quest init"] --> B["2. Edit xr_features & specs/*_xr.yaml"]
    B --> C["3. quest sync (Auto-Gen Manifest, Bootstrap & Scene)"]
    C --> D["4. Implement Action Handlers"]
    D --> E["5. quest check (QF-SPEC / SYNC / VRC / OXR / PERF)"]
    E -->|"0 Offences"| F["6. quest ci + quest verify (100/100)"]
    E -->|"Offence"| D
    F --> G["7. quest ops deploy"]
```

---

## 2. Declarative XR Codegen Architecture (`quest sync` & `quest check`)

QuestForge generates engine manifests, `AndroidManifest.xml`, runtime OpenXR session/extension startup code, and strongly-typed scene interaction controllers directly from `quest_forge.yaml` and `specs/*_xr.yaml`:

```mermaid
flowchart TD
    Manifest["quest_forge.yaml (xr_features, sdk, targets)"]
    SceneSpec["specs/main_arena_xr.yaml (Locomotion, Interactables, Spatial Panels)"]
    Sync["quest sync (Declarative XR Code Generator)"]
    AndroidManifest["AndroidManifest.xml (Auto-Generated VRC XML)"]
    Bootstrap["QuestForgeBootstrap.gen.* (90/120Hz, Foveation, Passthrough, OpenXR Extensions)"]
    SceneGen["MainArenaXrScene.gen.* (IMainArenaXrActions + MainArenaXrController)"]
    UserCode["MainArenaActions.* (Studio Implements Only Action Callbacks)"]
    Check["quest check (QF-SPEC-001, QF-SYNC-001, QF-VRC-001, QF-OXR-001, QF-PERF-001)"]

    Manifest --> Sync
    SceneSpec --> Sync
    Sync --> AndroidManifest & Bootstrap & SceneGen
    SceneGen --> UserCode
    AndroidManifest & Bootstrap & UserCode --> Check
```

### Multi-Engine Codegen Matrix

| Engine Target | Status | `quest_forge.yaml` | Generated Bootstrap | Generated Scene Code (`specs/*_xr.yaml`) | User Handler File |
|---|---|---|---|---|---|
| **Unity (C# / IL2CPP)** | **Shipped** | `engine: unity` | `Assets/Generated/QuestForgeBootstrap.gen.cs` | `Assets/Generated/<Scene>XrScene.gen.cs` | `Assets/Scripts/MainArenaActions.cs` |
| **Unreal Engine (C++)** | **Shipped** | `engine: unreal` | `Source/QuestForge/QuestForgeBootstrap.gen.h` | `Source/QuestForge/<Scene>XrScene.gen.h` | `Source/QuestForge/MainArenaActions.h` |
| **Native Rust OpenXR** | **Shipped** | `engine: rust` | `src/questforge_bootstrap_gen.rs` | `src/<scene>_xr_scene_gen.rs` | `src/main_arena_actions.rs` |
| **Godot 4 OpenXR** | **Roadmap (`#18`)** | `engine: godot` | `addons/questforge/bootstrap.gen.gd` | `addons/questforge/<scene>_xr_scene.gen.gd` | `scripts/main_arena_actions.gd` |
| **Native C++ OpenXR** | **Roadmap (`#18`)** | `engine: cpp` | `include/questforge_bootstrap.gen.hpp` | `include/<scene>_xr_scene.gen.hpp` | `src/main_arena_actions.cpp` |
| **WebXR + `wgpu` (`mweb`)** | **Roadmap (`#18`)** | `engine: webxr` | `src/generated/questforge_bootstrap.gen.ts` | `src/generated/<scene>_xr_scene.gen.ts` | `src/features/main_arena_actions.ts` |

---

## 3. End-to-End Studio Sequence

```mermaid
sequenceDiagram
    autonumber
    actor Studio as XR Studio Developer
    participant CLI as quest CLI / quest agent --role studio
    participant Cache as ~/.questforge/cache
    participant Engine as Unity / Unreal / Rust OpenXR
    participant Quest as Local Quest Headset

    Studio->>CLI: quest init my-quest-app --engine rust --package-id com.studio.app
    CLI-->>Studio: Scaffolds quest_forge.yaml, specs/main_arena_xr.yaml, generated bindings
    Studio->>CLI: quest sync && quest check
    CLI-->>Studio: QF-SPEC / QF-SYNC / QF-VRC / QF-OXR / QF-PERF -> 0 offences
    Studio->>CLI: quest setup --adb-version latest --ovr-version latest
    CLI->>Cache: Downloads & verifies SHA-256 of platform-tools & ovr-platform-util
    Studio->>CLI: quest doctor --fix
    CLI-->>Studio: QD-001..QD-009 healthy
    Studio->>CLI: quest ci --output-dir dist --release-version 0.1.0
    CLI->>Engine: Runs headless ARM64 build
    Engine-->>CLI: Emits dist/my-quest-app-0.1.0.apk + build-metadata.json + sbom.cdx.json
    Studio->>CLI: quest verify dist/my-quest-app-0.1.0.apk
    CLI-->>Studio: Audits QV-001..QV-006 -> Score 100/100 (READY)
    Studio->>CLI: quest ops deploy local-dev dist/my-quest-app-0.1.0.apk
    CLI->>Quest: Installs APK via adb install -r
```

See the **[Developer Guide](../../4-development/developer_guide.md)** for full CLI commands.
