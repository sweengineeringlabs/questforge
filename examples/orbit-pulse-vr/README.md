# Orbit Pulse VR (`orbit-pulse-vr`)

**Showcase Native Rust OpenXR & Mixed Reality Application for Meta Quest & Android**

`orbit-pulse-vr` is the official showcase project demonstrating QuestForge's declarative workflow, sub-second static verification, zero-allocation Rust kinematics, and **stratified dual-mode hand wave interaction** ([ADR-002](../../docs/3-design/adr/002_stratified_input_and_gesture_kinematics.md)).

---

## Key Highlights

* **Pure Rust OpenXR Core (`cdylib`):** Compiled for `aarch64-linux-android` with zero heap allocations at 120 Hz.
* **Declarative Interaction (`specs/orbit_arena_xr.yaml`):** Declares grabbable objects, poke buttons, spatial anchors, and the `wave_gesture`.
* **Zero-Hardware Evaluation:** Runs as a native 6DoF spatial VR game on Meta Quest, or as an interactive application on standard Android phones and headless emulators.
* **100/100 VRC Verification:** Fully compliant with Meta Horizon Store policies (64-bit ARM64, v2+ signing, non-debuggable, clean permissions).
* **Live Meta Horizon Store Release:** Uploaded to **ALPHA** channel (Build ID `1440281285843564`, App ID `1439257382612621`).
* **GitHub Public Release:** Downloadable at [GitHub Releases v0.1.2](https://github.com/sweengineeringlabs/questforge/releases/tag/v0.1.2).

---

## How to Test the Hand-Wave Gesture

QuestForge decouples core spatial interaction logic from physical optical camera sensors, giving judges and developers three distinct ways to test the hand wave:

### 1. On Meta Quest (Optical 3D Hand Tracking)
When deployed to a Quest 2, Quest 3, Quest 3S, or Quest Pro:
* OpenXR optical hand tracking (`XR_EXT_hand_tracking`) tracks 26 joints per hand.
* The native Rust kinematics engine (`OpenXrHandWaveDetector`) analyzes palm lateral velocity ($v_x > 0.20\text{ m/s}$) and direction alternations in real time.
* Waving your hand horizontally activates the **Kinetic Shield Boost (+20% energy)** and awards $+50\times\text{combo}$ points.

### 2. On Any Android Phone / Tablet (Hands-Free IR Proximity Sensor)
You do **not** need a VR headset to experience physical hand-wave interaction:
1. Install and launch `orbit-pulse-vr-1.0.0.apk` on any standard Android phone.
2. Hold the phone in front of you.
3. **Wave your physical hand across the top front screen area (< 5 cm above the front camera / IR proximity sensor).**
4. The IR proximity sensor (`Sensor.TYPE_PROXIMITY`) detects your hand entering and leaving the IR field, bridges the event into native Rust via JNI (`nativeOnGesture(1)`), updates the live HUD, and displays `WAVE DETECTED via IR Proximity Sensor Wave! Kinetic Shield Boost +20%`.

### 3. On Android Emulator or Touch Screen
If testing in a headless CI environment or emulator without an IR proximity sensor:
* **Fling / Swipe:** Rapidly swipe horizontally across the screen ($|v_x| > 500\text{ px/s}$).
* **Button Trigger:** Tap **"Wave Hand Gesture (Shield Pulse +20%)"** to simulate the gesture dispatch directly.

---

## Build & Verification Commands

```pwsh
# 1. Run sub-second static checks (manifest, openxr parity, frame budget)
quest check --project-dir .

# 2. Synchronize declarative specs into code and AndroidManifest.xml
quest sync --project-dir .

# 3. Run unit & kinematics test suite
cargo test --manifest-path Cargo.toml

# 4. Package release APK with ARM64 native library and DEX launcher
quest ci --project-dir . --output-dir dist --release-version 1.0.0

# 5. Verify Meta Horizon Store compliance (asserts 100/100 score)
quest verify dist/orbit-pulse-vr-1.0.0.apk --project-dir .

# 6. Upload directly to Meta Horizon Store Alpha channel
quest ops store-upload --app-id 1439257382612621 --apk dist/orbit-pulse-vr-1.0.0.apk --channel ALPHA --notes "Release 1.0.0"
```
