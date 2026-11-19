//! Gameplay state machine and `OrbitArenaXrActions` implementation for Orbit Pulse VR.

use crate::orbit_arena_xr_scene_gen::OrbitArenaXrActions;
use crate::questforge_bootstrap_gen::{
    FOVEATION_LEVEL, HAND_TRACKING_ENABLED, PASSTHROUGH_ENABLED, SPATIAL_ANCHORS_ENABLED,
    TARGET_REFRESH_RATE_HZ,
};

/// Current lifecycle phase of an Orbit Pulse VR match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WavePhase {
    /// Waiting for the player to poke `launch_pad`.
    Lobby,
    /// Active wave with incoming energy pulses.
    Active,
    /// Match paused or completed.
    Completed,
}

/// Developer-owned VR gameplay state implementing generated `OrbitArenaXrActions`.
#[derive(Clone, Debug, PartialEq)]
pub struct OrbitArenaGame {
    /// Current wave phase.
    pub phase: WavePhase,
    /// Active wave number (starts at `1`).
    pub wave: u32,
    /// Player score.
    pub score: u32,
    /// Current combo multiplier (`1`..=`8`).
    pub combo: u32,
    /// Remaining shield energy (`0`..=`100`).
    pub shield_charge: u8,
    /// Whether Mixed Reality passthrough is currently toggled on.
    pub passthrough_active: bool,
    /// Whether the room spatial anchor has been locked.
    pub anchor_locked: bool,
    /// Elapsed wave time in seconds.
    pub elapsed_secs: f32,
    /// Count of recognized hand wave gestures.
    pub wave_gesture_count: u32,
}

impl Default for OrbitArenaGame {
    fn default() -> Self {
        Self::new()
    }
}

impl OrbitArenaGame {
    /// Initializes a fresh match using the `quest sync` generated hardware profile.
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: WavePhase::Lobby,
            wave: 1,
            score: 0,
            combo: 1,
            shield_charge: 100,
            passthrough_active: PASSTHROUGH_ENABLED,
            anchor_locked: false,
            elapsed_secs: 0.0,
            wave_gesture_count: 0,
        }
    }

    /// Returns the active Meta Quest runtime profile generated from `quest_forge.yaml`.
    #[must_use]
    pub const fn runtime_profile() -> (u32, bool, bool, bool, &'static str) {
        (
            TARGET_REFRESH_RATE_HZ,
            PASSTHROUGH_ENABLED,
            SPATIAL_ANCHORS_ENABLED,
            HAND_TRACKING_ENABLED,
            FOVEATION_LEVEL,
        )
    }

    /// Advances the VR simulation by one frame (`dt_secs`, e.g. `1.0 / 120.0` at 120 Hz).
    /// Zero heap allocations and zero blocking calls (enforced by `quest check` `QF-PERF-001`).
    pub fn step_frame(&mut self, dt_secs: f32) {
        if self.phase != WavePhase::Active || dt_secs <= 0.0 {
            return;
        }
        self.elapsed_secs += dt_secs;
        if self.elapsed_secs >= 30.0 {
            self.wave += 1;
            self.elapsed_secs = 0.0;
            self.shield_charge = self.shield_charge.saturating_add(15).min(100);
        }
    }
}

impl OrbitArenaXrActions for OrbitArenaGame {
    fn on_pulse_orb_grabbed(&mut self) {
        if self.phase != WavePhase::Active {
            return;
        }
        self.score += 100 * self.combo;
        self.combo = (self.combo + 1).min(8);
    }

    fn on_shield_activated(&mut self) {
        if self.phase != WavePhase::Active {
            return;
        }
        self.shield_charge = self.shield_charge.saturating_sub(10);
        self.score += 25;
    }

    fn on_wave_started(&mut self) {
        self.phase = WavePhase::Active;
    }

    fn on_anchor_locked(&mut self) {
        self.anchor_locked = true;
    }

    fn on_wave_gesture_detected(&mut self) {
        if self.phase != WavePhase::Active {
            return;
        }
        self.shield_charge = self.shield_charge.saturating_add(20).min(100);
        self.score += 50 * self.combo;
        self.wave_gesture_count += 1;
    }

    fn on_toggle_passthrough(&mut self) {
        self.passthrough_active = !self.passthrough_active;
    }

    fn on_reset_match(&mut self) {
        let keep_anchor = self.anchor_locked;
        *self = Self::new();
        self.anchor_locked = keep_anchor;
    }
}

/// Real-time kinematics detector for 3D hand wave gestures (OpenXR `XR_EXT_hand_tracking`).
/// Tracks palm horizontal lateral oscillations in head-relative or world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpenXrHandWaveDetector {
    last_x: f32,
    last_time: f32,
    last_direction: i8,
    direction_changes: u8,
    last_change_time: f32,
    wave_cooldown: f32,
}

impl Default for OpenXrHandWaveDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenXrHandWaveDetector {
    /// Constructs a fresh gesture detector.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_x: 0.0,
            last_time: 0.0,
            last_direction: 0,
            direction_changes: 0,
            last_change_time: 0.0,
            wave_cooldown: 0.0,
        }
    }

    /// Feeds a new palm joint position sample `(palm_x, palm_y, palm_z)` at timestamp `time_secs`.
    /// Returns `true` if a full hand wave cycle (2+ lateral direction alternations with sufficient velocity) is detected.
    pub fn update(&mut self, time_secs: f32, palm_x: f32, _palm_y: f32, _palm_z: f32) -> bool {
        if time_secs <= self.wave_cooldown {
            return false;
        }

        let dt = time_secs - self.last_time;
        if dt <= 0.001 {
            return false;
        }

        let dx = palm_x - self.last_x;
        let vx = dx / dt;

        let current_dir = if vx > 0.20 {
            1
        } else if vx < -0.20 {
            -1
        } else {
            0
        };

        let mut gesture_triggered = false;

        if current_dir != 0 && self.last_direction != 0 && current_dir != self.last_direction {
            let elapsed_since_change = time_secs - self.last_change_time;
            if elapsed_since_change <= 0.8 {
                self.direction_changes += 1;
                if self.direction_changes >= 2 {
                    gesture_triggered = true;
                    self.direction_changes = 0;
                    self.wave_cooldown = time_secs + 0.5;
                }
            } else {
                self.direction_changes = 1;
            }
            self.last_change_time = time_secs;
            self.last_direction = current_dir;
        } else if current_dir != 0 {
            if self.last_direction == 0 {
                self.last_change_time = time_secs;
            }
            self.last_direction = current_dir;
        }

        self.last_x = palm_x;
        self.last_time = time_secs;

        gesture_triggered
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orbit_arena_xr_scene_gen::OrbitArenaXrScene;

    #[test]
    fn test_orbit_pulse_vr_gameplay_loop() {
        let mut game = OrbitArenaGame::new();
        assert_eq!(
            OrbitArenaGame::runtime_profile(),
            (120, true, true, true, "high")
        );
        assert_eq!(OrbitArenaXrScene::SCENE_NAME, "OrbitArena");

        assert!(OrbitArenaXrScene::dispatch_action(
            "arena_anchor",
            &mut game
        ));
        assert!(game.anchor_locked);

        assert!(OrbitArenaXrScene::dispatch_action("launch_pad", &mut game));
        assert_eq!(game.phase, WavePhase::Active);

        assert!(OrbitArenaXrScene::dispatch_action("pulse_orb", &mut game));
        assert!(OrbitArenaXrScene::dispatch_action("pulse_orb", &mut game));
        assert_eq!(game.score, 300); // 100*1 + 100*2
        assert_eq!(game.combo, 3);

        assert!(OrbitArenaXrScene::dispatch_action(
            "shield_emitter",
            &mut game
        ));
        assert_eq!(game.shield_charge, 90);
        assert_eq!(game.score, 325);

        // Test wave gesture activation
        assert!(OrbitArenaXrScene::dispatch_action(
            "wave_gesture",
            &mut game
        ));
        assert_eq!(game.wave_gesture_count, 1);
        assert_eq!(game.shield_charge, 100); // 90 + 20 clamped to 100
        assert_eq!(game.score, 475); // 325 + 50*3

        // Test dispatch_gesture
        assert!(OrbitArenaXrScene::dispatch_gesture("wave", &mut game));
        assert_eq!(game.wave_gesture_count, 2);
        assert_eq!(game.score, 625); // 475 + 50*3

        for _ in 0..3601 {
            game.step_frame(1.0 / 120.0);
        }
        assert_eq!(game.wave, 2);
        assert_eq!(game.shield_charge, 100);

        assert!(OrbitArenaXrScene::dispatch_action(
            "toggle_passthrough",
            &mut game
        ));
        assert!(!game.passthrough_active);

        assert!(OrbitArenaXrScene::dispatch_action("reset_match", &mut game));
        assert_eq!(game.phase, WavePhase::Lobby);
        assert_eq!(game.score, 0);
        assert!(game.anchor_locked);
    }

    #[test]
    fn test_openxr_hand_wave_detector_kinematics() {
        let mut detector = OpenXrHandWaveDetector::new();

        // Initial hand position
        assert!(!detector.update(0.00, 0.0, 1.2, -0.4));

        // Move right quickly (vx > 0.20 m/s)
        assert!(!detector.update(0.10, 0.15, 1.2, -0.4));

        // Reversal 1: Move left quickly
        assert!(!detector.update(0.25, -0.10, 1.2, -0.4));

        // Reversal 2: Move right quickly again -> wave detected!
        let detected = detector.update(0.40, 0.15, 1.2, -0.4);
        assert!(detected);
    }
}
