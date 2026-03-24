//! Eye Blink Animation System
//!
//! Manages random eye blinks and micro-motions for menu bar icons.
//! Features:
//! - Random blinks at configurable intervals
//! - Double-blink with 18% probability
//! - Motion effects: blink, wiggle, tilt
//! - Per-provider blink state

#![allow(dead_code)]

use rand::Rng;
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::core::ProviderId;

/// Motion effect types for icon animations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionEffect {
    /// Eye closing/opening animation
    Blink,
    /// Vertical wiggle of appendages (Claude crab)
    Wiggle,
    /// Rotation of hat/cap (Codex)
    Tilt,
}

impl MotionEffect {
    /// Get a random motion effect appropriate for a provider
    pub fn random_for_provider(provider: ProviderId) -> Self {
        let mut rng = rand::rng();
        match provider {
            ProviderId::Claude => {
                // Claude can blink or wiggle
                if rng.random_bool(0.5) {
                    MotionEffect::Blink
                } else {
                    MotionEffect::Wiggle
                }
            }
            ProviderId::Codex => {
                // Codex can blink or tilt hat
                if rng.random_bool(0.5) {
                    MotionEffect::Blink
                } else {
                    MotionEffect::Tilt
                }
            }
            _ => {
                // Other providers just blink
                MotionEffect::Blink
            }
        }
    }
}

/// State for a single provider's blink animation
#[derive(Debug, Clone)]
pub struct BlinkState {
    /// When the next blink should start
    pub next_blink: Instant,
    /// When the current blink started (None if not blinking)
    pub blink_start: Option<Instant>,
    /// Pending second blink for double-blink
    pub pending_second_start: Option<Instant>,
    /// Current motion effect
    pub effect: MotionEffect,
}

impl BlinkState {
    /// Create a new blink state with a random delay
    pub fn new() -> Self {
        Self {
            next_blink: Instant::now() + Self::random_delay(),
            blink_start: None,
            pending_second_start: None,
            effect: MotionEffect::Blink,
        }
    }

    /// Generate a random delay between blinks (2-6 seconds)
    pub fn random_delay() -> Duration {
        let mut rng = rand::rng();
        let seconds: f64 = rng.random_range(2.0..6.0);
        Duration::from_secs_f64(seconds)
    }

    /// Generate a random delay for double-blink (0.22-0.34 seconds)
    pub fn double_blink_delay() -> Duration {
        let mut rng = rand::rng();
        let seconds: f64 = rng.random_range(0.22..0.34);
        Duration::from_secs_f64(seconds)
    }
}

impl Default for BlinkState {
    fn default() -> Self {
        Self::new()
    }
}

/// Blink animation system configuration
#[derive(Debug, Clone)]
pub struct BlinkConfig {
    /// Duration of a single blink animation
    pub blink_duration: Duration,
    /// Probability of a double-blink (0.0-1.0)
    pub double_blink_chance: f64,
    /// Tick interval for the animation loop
    pub tick_interval: Duration,
    /// Maximum tilt angle in radians
    pub max_tilt: f32,
}

impl Default for BlinkConfig {
    fn default() -> Self {
        Self {
            blink_duration: Duration::from_millis(360),
            double_blink_chance: 0.18,
            tick_interval: Duration::from_millis(75),
            max_tilt: std::f32::consts::PI / 28.0, // ~6.4 degrees
        }
    }
}

/// Animation output from a blink tick
#[derive(Debug, Clone, Default)]
pub struct BlinkOutput {
    /// Blink amount (0.0 = open, 1.0 = closed)
    pub blink: f32,
    /// Wiggle amount (-1.0 to 1.0)
    pub wiggle: f32,
    /// Tilt amount in radians
    pub tilt: f32,
}

impl BlinkOutput {
    /// Whether any motion is active
    pub fn has_motion(&self) -> bool {
        self.blink > 0.001 || self.wiggle.abs() > 0.001 || self.tilt.abs() > 0.001
    }
}

/// Eye blink animation system
pub struct EyeBlinkSystem {
    config: BlinkConfig,
    states: HashMap<ProviderId, BlinkState>,
    outputs: HashMap<ProviderId, BlinkOutput>,
    enabled: bool,
    force_until: Option<Instant>,
}

impl EyeBlinkSystem {
    /// Create a new blink system with default config
    pub fn new() -> Self {
        Self::with_config(BlinkConfig::default())
    }

    /// Create a new blink system with custom config
    pub fn with_config(config: BlinkConfig) -> Self {
        Self {
            config,
            states: HashMap::new(),
            outputs: HashMap::new(),
            enabled: true,
            force_until: None,
        }
    }

    /// Enable or disable the blink system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.outputs.clear();
        }
    }

    /// Check if blinking is currently allowed
    pub fn is_blinking_allowed(&self) -> bool {
        if self.enabled {
            return true;
        }
        if let Some(until) = self.force_until {
            if until > Instant::now() {
                return true;
            }
        }
        false
    }

    /// Force a blink to happen now
    pub fn force_blink(&mut self, provider: ProviderId) {
        let now = Instant::now();
        self.force_until = Some(now + Duration::from_millis(600));

        let state = self.states.entry(provider).or_insert_with(BlinkState::new);
        state.blink_start = Some(now);
        state.pending_second_start = None;
        state.effect = MotionEffect::random_for_provider(provider);
        state.next_blink = now + BlinkState::random_delay();
    }

    /// Seed blink state for a provider if not already present
    pub fn seed_provider(&mut self, provider: ProviderId) {
        self.states.entry(provider).or_insert_with(BlinkState::new);
    }

    /// Tick the animation system for a provider
    ///
    /// Returns the current animation output for the provider
    pub fn tick(&mut self, provider: ProviderId) -> BlinkOutput {
        if !self.is_blinking_allowed() {
            return BlinkOutput::default();
        }

        let now = Instant::now();
        let config = &self.config;
        let mut rng = rand::rng();

        // Get or create state
        let state = self.states.entry(provider).or_insert_with(BlinkState::new);

        // Check for pending double-blink
        if let Some(pending) = state.pending_second_start {
            if now >= pending {
                state.blink_start = Some(now);
                state.pending_second_start = None;
            }
        }

        let output = if let Some(start) = state.blink_start {
            let elapsed = now.duration_since(start);

            if elapsed >= config.blink_duration {
                // Blink finished
                state.blink_start = None;
                if state.pending_second_start.is_none() {
                    state.next_blink = now + BlinkState::random_delay();
                }
                BlinkOutput::default()
            } else {
                // Blink in progress
                let progress = elapsed.as_secs_f32() / config.blink_duration.as_secs_f32();
                let progress = progress.clamp(0.0, 1.0);

                // Symmetric curve: 0->1->0
                let symmetric = if progress < 0.5 {
                    progress * 2.0
                } else {
                    (1.0 - progress) * 2.0
                };

                // Slightly punchier easing
                let eased = symmetric.powf(2.2);

                let mut output = BlinkOutput::default();
                match state.effect {
                    MotionEffect::Blink => output.blink = eased,
                    MotionEffect::Wiggle => output.wiggle = eased,
                    MotionEffect::Tilt => output.tilt = eased * config.max_tilt,
                }
                output
            }
        } else if now >= state.next_blink {
            // Start a new blink
            state.blink_start = Some(now);
            state.effect = MotionEffect::random_for_provider(provider);

            // Maybe schedule a double-blink
            if state.effect == MotionEffect::Blink
                && rng.random_bool(config.double_blink_chance)
            {
                state.pending_second_start = Some(now + BlinkState::double_blink_delay());
            }

            BlinkOutput::default()
        } else {
            BlinkOutput::default()
        };

        self.outputs.insert(provider, output.clone());
        output
    }

    /// Get the current output for a provider without updating
    pub fn get_output(&self, provider: ProviderId) -> BlinkOutput {
        self.outputs.get(&provider).cloned().unwrap_or_default()
    }

    /// Clear all motion for a provider
    pub fn clear_motion(&mut self, provider: ProviderId) {
        self.outputs.remove(&provider);
    }

    /// Clear all states and outputs
    pub fn reset(&mut self) {
        self.states.clear();
        self.outputs.clear();
        self.force_until = None;
    }
}

impl Default for EyeBlinkSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blink_state_new() {
        let state = BlinkState::new();
        assert!(state.blink_start.is_none());
        assert!(state.pending_second_start.is_none());
        assert!(state.next_blink > Instant::now() - Duration::from_secs(1));
    }

    #[test]
    fn test_random_delay_range() {
        for _ in 0..100 {
            let delay = BlinkState::random_delay();
            assert!(delay >= Duration::from_secs(2));
            assert!(delay <= Duration::from_secs(6));
        }
    }

    #[test]
    fn test_motion_effect_for_provider() {
        // Run multiple times to test randomness
        for _ in 0..20 {
            let claude_effect = MotionEffect::random_for_provider(ProviderId::Claude);
            assert!(matches!(
                claude_effect,
                MotionEffect::Blink | MotionEffect::Wiggle
            ));

            let codex_effect = MotionEffect::random_for_provider(ProviderId::Codex);
            assert!(matches!(
                codex_effect,
                MotionEffect::Blink | MotionEffect::Tilt
            ));

            let gemini_effect = MotionEffect::random_for_provider(ProviderId::Gemini);
            assert_eq!(gemini_effect, MotionEffect::Blink);
        }
    }

    #[test]
    fn test_blink_system_new() {
        let system = EyeBlinkSystem::new();
        assert!(system.enabled);
        assert!(system.is_blinking_allowed());
    }

    #[test]
    fn test_blink_system_disable() {
        let mut system = EyeBlinkSystem::new();
        system.set_enabled(false);
        assert!(!system.is_blinking_allowed());
    }

    #[test]
    fn test_force_blink() {
        let mut system = EyeBlinkSystem::new();
        system.set_enabled(false);

        system.force_blink(ProviderId::Claude);
        assert!(system.is_blinking_allowed());
        assert!(system.states.contains_key(&ProviderId::Claude));
    }

    #[test]
    fn test_blink_output_has_motion() {
        let none = BlinkOutput::default();
        assert!(!none.has_motion());

        let blink = BlinkOutput {
            blink: 0.5,
            ..Default::default()
        };
        assert!(blink.has_motion());

        let wiggle = BlinkOutput {
            wiggle: 0.3,
            ..Default::default()
        };
        assert!(wiggle.has_motion());
    }

    #[test]
    fn test_blink_config_defaults() {
        let config = BlinkConfig::default();
        assert_eq!(config.blink_duration, Duration::from_millis(360));
        assert!((config.double_blink_chance - 0.18).abs() < 0.01);
    }
}
