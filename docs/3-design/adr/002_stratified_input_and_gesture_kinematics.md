# ADR-002: Stratified Input Architecture and Dual-Mode Gesture Kinematics

**Audience**: Architects, technical leads, XR developers, QA engineers, and QuestForge contributors

## Status

Accepted

## Date

2026-11-19

## Context

Virtual reality applications targeting Meta Quest have historically faced the "Hardware Wall": testing and verifying spatial interactions (such as optical hand tracking and gestures) strictly required an engineer wearing a physical Meta Quest headset.

This creates critical bottlenecks across the software delivery lifecycle:
1. **Cloud CI/CD Failure:** Headless cloud runners (GitHub Actions, GitLab CI) have no cameras, optical sensors, or physical hands, making spatial gesture tests impossible in standard CI pipelines.
2. **Evaluation & Review Blockers:** Hackathon judges, open-source contributors, and remote security auditors who lack a physical Meta Quest 3 headset cannot evaluate or verify gesture-driven applications.
3. **Enterprise Kiosk Dual-Surface Requirements:** Enterprise applications deployed via Meta HMS, ManageXR, or ArborXR (e.g. medical training, manufacturing simulations) frequently require a 2D supervisor tablet/phone screen running the exact same simulation as the headset in the room.

We needed an architectural pattern that allows spatial hand tracking to be tested and demonstrated across all environments—from physical 6DoF headsets to standard Android phones and headless CI emulators—without maintaining fragmented codebases.

## Decision

1. **Declarative Gesture Contract (`questforge-config`):** Extend `InteractableKind` with `HandGesture` and support optional gesture classifications (`wave`, `pinch`, `open_palm`) in `specs/*_xr.yaml`. `quest sync` compiles these into strongly-typed scene action traits and `dispatch_gesture(&str, &mut Handler)` dispatchers.
2. **Hexagonal Input Architecture (Ports & Adapters):** Decouple core simulation domain logic (`OrbitArenaXrActions`) from specific physical sensor hardware. The core game/application loop handles interactions purely via typed callbacks; input adapters translate hardware events into these callbacks.
3. **Three-Tier Input Adapter Stratification:**
   * **Tier A (Immersive OpenXR):** Pure Rust kinematics algorithm (`OpenXrHandWaveDetector`) consuming 26-joint optical tracking data from OpenXR `XR_EXT_hand_tracking`. Evaluates palm lateral velocity ($v_x = \frac{\Delta x}{\Delta t} > 0.20\text{ m/s}$) and direction alternations within an 800ms window with 500ms debounce at 120 Hz with zero heap allocations.
   * **Tier B (Physical Android Device):** Android `Sensor.TYPE_PROXIMITY` listener in `MainActivity.java`. Detects hands-free physical hand sweeps ($< 5\text{ cm}$) over the front IR sensor cluster of any Android phone or tablet, dispatching to native Rust via JNI (`nativeOnGesture(1)`).
   * **Tier C (Headless CI & Emulator):** `GestureDetector.SimpleOnGestureListener` (touch fling/swipe) and synthetic action dispatchers (`nativeDispatchAction`) enabling automated regression tests in headless Android AVDs without human intervention.
4. **Unified Dual-Stack Packaging (`quest ci`):** Enhance `package_rust_apk` to bundle native `arm64-v8a` ELF shared libraries (`lib*.so`), compiled Java bytecode (`classes.dex`), and compiled resources (`resources.arsc`) into a single Meta VRC-compliant APK scoring 100/100 on `quest verify`.

## Comparison

| Aspect | Monolithic In-Headset Code | Stratified Hexagonal Input Layer (Chosen) |
|---|---|---|
| **Hardware Requirement** | Meta Quest headset required for any test | Testable on Quest, physical Android phone, or headless CI |
| **CI/CD Automation** | Blocked; manual QA only | Fully automated end-to-end testing in GitHub Actions AVDs |
| **Domain Logic Purity** | Tightly coupled to Unity/Unreal vendor SDKs | Pure Rust / C# domain logic implementing typed traits |
| **Enterprise Dual-Surface** | Requires two separate apps/repos | Single unified APK runs in 3D VR or 2D companion mode |
| **Kinematics Performance** | Garbage-collected per-frame allocations | Pure Rust, zero heap allocations, 120 Hz real-time budget |

## Rationale

1. **Zero-Hardware Evaluation:** Hackathon judges and contributors can verify gesture mechanics hands-free by waving over an Android phone's IR sensor, or testing via touch fling in an emulator.
2. **Shift-Left Spatial Testing:** Regression tests on hand gestures run inside continuous integration before any code reaches physical QA headsets.
3. **Deterministic Contract:** `quest check` verifies manifest permissions (`com.oculus.permission.HAND_TRACKING`, `android.hardware.sensor.proximity`) and OpenXR extension parity (`XR_EXT_hand_tracking`) statically in sub-seconds.

## Consequences

### Positive
* 100% of spatial interaction logic can be unit-tested locally (`cargo test`) and integration-tested in CI without physical headsets.
* The showcase app (`orbit-pulse-vr`) provides a production reference implementation of native OpenXR hand tracking + Android IR proximity fallback.
* Releases score 100/100 on `quest verify`, validating ABI compliance and store submission readiness.

### Negative
* Proximity IR sensors on mobile phones detect binary proximity (near/far) rather than full 26-joint 3D spatial joint coordinates.

### Mitigations
* Full 26-joint 3D kinematics are executed whenever OpenXR `XR_EXT_hand_tracking` is active; the IR proximity sensor is purely a hardware-agnostic fallback adapter for non-headset evaluation.

## Related

- [Architecture](../architecture.md)
- [Mobile & Emulator Integration Plan](../mobile_emulator_integration.md)
- [Developer Guide](../../4-development/developer_guide.md)
- [ADR-001: Stratified Workspace Architecture](001_stratified_architecture_and_mcp_routing.md)
