# orbit-pulse-vr Onboarding

**Audience**: XR developers, evaluators, and QuestForge contributors

Follow these steps to set up, build, test, and verify `orbit-pulse-vr`:

1. **Install Quest toolchain binaries**: `quest setup`
2. **Synchronize declarative XR manifests & codegen**: `quest sync`
3. **Run fast static XR & VRC source checks**: `quest check`
4. **Validate your workstation**: `quest doctor`
5. **Run the local CI gate & unit tests**: `cargo test --manifest-path Cargo.toml`
6. **Package release APK**: `quest ci --output-dir dist --release-version 1.0.0`
7. **Verify Meta Horizon Store compliance**: `quest verify dist/orbit-pulse-vr-1.0.0.apk`

---

## Evaluating Hand-Wave Interactions

`orbit-pulse-vr` implements dual-mode hand wave detection ([ADR-002](../../docs/3-design/adr/002_stratified_input_and_gesture_kinematics.md)):

* **On Meta Quest Headset:** Optical tracking evaluates palm kinematics (`OpenXrHandWaveDetector`, $v_x > 0.20\text{ m/s}$ at 120 Hz).
* **On Android Phone / Tablet:** Wave hand over the phone's top IR proximity sensor ($< 5\text{ cm}$) to trigger the wave hands-free.
* **On Headless Emulator / CI:** Swipe across the screen or trigger JNI `nativeOnGesture(1)`.
