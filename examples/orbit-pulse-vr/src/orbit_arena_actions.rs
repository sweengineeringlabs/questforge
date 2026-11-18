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

    fn on_toggle_passthrough(&mut self) {
        self.passthrough_active = !self.passthrough_active;
    }

    fn on_reset_match(&mut self) {
        let keep_anchor = self.anchor_locked;
        *self = Self::new();
        self.anchor_locked = keep_anchor;
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

        assert!(OrbitArenaXrScene::dispatch_action("arena_anchor", &mut game));
        assert!(game.anchor_locked);

        assert!(OrbitArenaXrScene::dispatch_action("launch_pad", &mut game));
        assert_eq!(game.phase, WavePhase::Active);

        assert!(OrbitArenaXrScene::dispatch_action("pulse_orb", &mut game));
        assert!(OrbitArenaXrScene::dispatch_action("pulse_orb", &mut game));
        assert_eq!(game.score, 300); // 100*1 + 100*2
        assert_eq!(game.combo, 3);

        assert!(OrbitArenaXrScene::dispatch_action("shield_emitter", &mut game));
        assert_eq!(game.shield_charge, 90);
        assert_eq!(game.score, 325);

        for _ in 0..3601 {
            game.step_frame(1.0 / 120.0);
        }
        assert_eq!(game.wave, 2);
        assert_eq!(game.shield_charge, 100);

        assert!(OrbitArenaXrScene::dispatch_action("toggle_passthrough", &mut game));
        assert!(!game.passthrough_active);

        assert!(OrbitArenaXrScene::dispatch_action("reset_match", &mut game));
        assert_eq!(game.phase, WavePhase::Lobby);
        assert_eq!(game.score, 0);
        assert!(game.anchor_locked);
    }
}
