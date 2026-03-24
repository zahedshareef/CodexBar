//! Provider-Specific Icon Twists
//!
//! Unique visual modifications for different AI provider icons.
//! - Claude: Crab-style with arms, legs, and vertical eyes
//! - Gemini: Sparkle-inspired with 4-pointed star eyes
//! - Factory: Gear/droid-like with asterisk eyes and cog teeth
//! - Antigravity: Gemini sparkle eyes with orbiting dot

#![allow(dead_code)]

use crate::core::ProviderId;

/// Icon style/twist to apply when rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconTwist {
    /// Default simple bar with face (Codex-style)
    #[default]
    Simple,
    /// Crab-style with arms, legs, vertical eyes (Claude)
    Crab,
    /// 4-pointed sparkle star eyes with decorative points (Gemini)
    Sparkle,
    /// Sparkle eyes with orbiting dot (Antigravity)
    SparkleOrbit,
    /// 8-pointed asterisk/gear eyes with cog teeth (Factory/Windsurf)
    Gear,
    /// Just a capsule bar with no face
    Plain,
}

impl IconTwist {
    /// Get the icon twist for a provider
    pub fn for_provider(provider: ProviderId) -> Self {
        match provider {
            ProviderId::Claude => IconTwist::Crab,
            ProviderId::Gemini | ProviderId::VertexAI => IconTwist::Sparkle,
            ProviderId::Antigravity => IconTwist::SparkleOrbit,
            ProviderId::Factory => IconTwist::Gear,
            ProviderId::Codex => IconTwist::Simple,
            _ => IconTwist::Plain,
        }
    }

    /// Whether this twist adds notches (blocky corners instead of rounded)
    pub fn has_notches(&self) -> bool {
        matches!(self, IconTwist::Crab)
    }

    /// Whether this twist has a face (eyes)
    pub fn has_face(&self) -> bool {
        !matches!(self, IconTwist::Plain)
    }

    /// Whether this twist has arms/legs (Claude crab)
    pub fn has_appendages(&self) -> bool {
        matches!(self, IconTwist::Crab)
    }

    /// Whether this twist has sparkle decorations
    pub fn has_sparkles(&self) -> bool {
        matches!(self, IconTwist::Sparkle | IconTwist::SparkleOrbit)
    }

    /// Whether this twist has an orbiting dot
    pub fn has_orbit(&self) -> bool {
        matches!(self, IconTwist::SparkleOrbit)
    }

    /// Whether this twist has gear teeth
    pub fn has_gear_teeth(&self) -> bool {
        matches!(self, IconTwist::Gear)
    }
}

/// Parameters for rendering icon features
#[derive(Debug, Clone, Default)]
pub struct IconFeatures {
    /// The icon twist style to use
    pub twist: IconTwist,
    /// Blink amount (0.0 = eyes open, 1.0 = eyes closed)
    pub blink: f32,
    /// Wiggle amount for Claude crab appendages
    pub wiggle: f32,
    /// Tilt amount for hat/cap rotation (radians)
    pub tilt: f32,
}

impl IconFeatures {
    pub fn new(twist: IconTwist) -> Self {
        Self {
            twist,
            ..Default::default()
        }
    }

    pub fn with_blink(mut self, blink: f32) -> Self {
        self.blink = blink.clamp(0.0, 1.0);
        self
    }

    pub fn with_wiggle(mut self, wiggle: f32) -> Self {
        self.wiggle = wiggle.clamp(-1.0, 1.0);
        self
    }

    pub fn with_tilt(mut self, tilt: f32) -> Self {
        self.tilt = tilt;
        self
    }

    /// Create features for a provider
    pub fn for_provider(provider: ProviderId) -> Self {
        Self::new(IconTwist::for_provider(provider))
    }
}

/// Eye shape for the icon
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EyeShape {
    /// Square/rectangular eyes (Simple/Codex)
    Square,
    /// Tall vertical rectangles (Crab/Claude)
    Vertical,
    /// 4-pointed star (Sparkle/Gemini)
    Star4,
    /// 8-pointed asterisk (Gear/Factory)
    Star8,
}

impl IconTwist {
    /// Get the eye shape for this twist
    pub fn eye_shape(&self) -> Option<EyeShape> {
        match self {
            IconTwist::Simple => Some(EyeShape::Square),
            IconTwist::Crab => Some(EyeShape::Vertical),
            IconTwist::Sparkle | IconTwist::SparkleOrbit => Some(EyeShape::Star4),
            IconTwist::Gear => Some(EyeShape::Star8),
            IconTwist::Plain => None,
        }
    }
}

/// Decoration element for icon embellishments
#[derive(Debug, Clone)]
pub struct Decoration {
    /// Type of decoration
    pub kind: DecorationKind,
    /// X offset from bar center (pixels)
    pub x_offset: i32,
    /// Y offset from bar center (pixels)
    pub y_offset: i32,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

/// Types of decorative elements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecorationKind {
    /// Rectangular arm (Claude crab)
    Arm,
    /// Small rectangular leg (Claude crab)
    Leg,
    /// Triangular point (Gemini sparkle)
    Point,
    /// Rectangular tooth (Factory gear)
    Tooth,
    /// Circular dot (Antigravity orbit)
    Dot,
    /// Rectangular hat/cap (Codex)
    Hat,
}

impl IconTwist {
    /// Get the decorations for this twist
    pub fn decorations(&self, bar_width: u32, bar_height: u32) -> Vec<Decoration> {
        let mut decorations = Vec::new();
        let bar_w = bar_width as i32;
        let bar_h = bar_height as i32;

        match self {
            IconTwist::Crab => {
                // Left and right arms
                let arm_width = 3;
                let arm_height = (bar_h - 6).max(2) as u32;
                decorations.push(Decoration {
                    kind: DecorationKind::Arm,
                    x_offset: -(bar_w / 2) - arm_width,
                    y_offset: 0,
                    width: arm_width as u32,
                    height: arm_height,
                });
                decorations.push(Decoration {
                    kind: DecorationKind::Arm,
                    x_offset: bar_w / 2,
                    y_offset: 0,
                    width: arm_width as u32,
                    height: arm_height,
                });

                // Four legs underneath
                let leg_width = 2;
                let leg_height = 3;
                let step = bar_w / 5;
                for i in 0..4 {
                    let x = -(bar_w / 2) + step * (i + 1) - leg_width / 2;
                    decorations.push(Decoration {
                        kind: DecorationKind::Leg,
                        x_offset: x,
                        y_offset: -(bar_h / 2) - leg_height,
                        width: leg_width as u32,
                        height: leg_height as u32,
                    });
                }
            }
            IconTwist::Sparkle | IconTwist::SparkleOrbit => {
                // Top and bottom points
                let point_size = 4;
                decorations.push(Decoration {
                    kind: DecorationKind::Point,
                    x_offset: 0,
                    y_offset: bar_h / 2,
                    width: point_size,
                    height: point_size,
                });
                decorations.push(Decoration {
                    kind: DecorationKind::Point,
                    x_offset: 0,
                    y_offset: -(bar_h / 2) - point_size as i32,
                    width: point_size,
                    height: point_size,
                });

                // Side points
                let side_point = 3;
                decorations.push(Decoration {
                    kind: DecorationKind::Point,
                    x_offset: -(bar_w / 2) - side_point as i32,
                    y_offset: 0,
                    width: side_point,
                    height: side_point,
                });
                decorations.push(Decoration {
                    kind: DecorationKind::Point,
                    x_offset: bar_w / 2,
                    y_offset: 0,
                    width: side_point,
                    height: side_point,
                });

                // Orbiting dot for Antigravity
                if *self == IconTwist::SparkleOrbit {
                    decorations.push(Decoration {
                        kind: DecorationKind::Dot,
                        x_offset: bar_w / 2 + 2,
                        y_offset: bar_h / 2 - 2,
                        width: 3,
                        height: 3,
                    });
                }
            }
            IconTwist::Gear => {
                // Top and bottom gear teeth
                let tooth_width = 3;
                let tooth_height = 2;
                let tooth_spacing = 5;

                for offset in [-tooth_spacing, tooth_spacing] {
                    // Top teeth
                    decorations.push(Decoration {
                        kind: DecorationKind::Tooth,
                        x_offset: offset - tooth_width as i32 / 2,
                        y_offset: bar_h / 2,
                        width: tooth_width,
                        height: tooth_height,
                    });
                    // Bottom teeth
                    decorations.push(Decoration {
                        kind: DecorationKind::Tooth,
                        x_offset: offset - tooth_width as i32 / 2,
                        y_offset: -(bar_h / 2) - tooth_height as i32,
                        width: tooth_width,
                        height: tooth_height,
                    });
                }
            }
            IconTwist::Simple => {
                // Hat for Codex
                let hat_width = 18;
                let hat_height = 4;
                decorations.push(Decoration {
                    kind: DecorationKind::Hat,
                    x_offset: -(hat_width as i32 / 2),
                    y_offset: bar_h / 2 - hat_height as i32,
                    width: hat_width,
                    height: hat_height,
                });
            }
            IconTwist::Plain => {}
        }

        decorations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_twist_for_provider() {
        assert_eq!(IconTwist::for_provider(ProviderId::Claude), IconTwist::Crab);
        assert_eq!(
            IconTwist::for_provider(ProviderId::Gemini),
            IconTwist::Sparkle
        );
        assert_eq!(
            IconTwist::for_provider(ProviderId::Factory),
            IconTwist::Gear
        );
        assert_eq!(
            IconTwist::for_provider(ProviderId::Antigravity),
            IconTwist::SparkleOrbit
        );
    }

    #[test]
    fn test_twist_properties() {
        assert!(IconTwist::Crab.has_notches());
        assert!(!IconTwist::Sparkle.has_notches());

        assert!(IconTwist::Crab.has_appendages());
        assert!(!IconTwist::Gear.has_appendages());

        assert!(IconTwist::SparkleOrbit.has_orbit());
        assert!(!IconTwist::Sparkle.has_orbit());

        assert!(IconTwist::Gear.has_gear_teeth());
    }

    #[test]
    fn test_crab_decorations() {
        let decorations = IconTwist::Crab.decorations(30, 12);
        // Should have 2 arms + 4 legs = 6 decorations
        assert_eq!(decorations.len(), 6);

        let arms = decorations
            .iter()
            .filter(|d| d.kind == DecorationKind::Arm)
            .count();
        let legs = decorations
            .iter()
            .filter(|d| d.kind == DecorationKind::Leg)
            .count();
        assert_eq!(arms, 2);
        assert_eq!(legs, 4);
    }

    #[test]
    fn test_eye_shapes() {
        assert_eq!(IconTwist::Simple.eye_shape(), Some(EyeShape::Square));
        assert_eq!(IconTwist::Crab.eye_shape(), Some(EyeShape::Vertical));
        assert_eq!(IconTwist::Sparkle.eye_shape(), Some(EyeShape::Star4));
        assert_eq!(IconTwist::Gear.eye_shape(), Some(EyeShape::Star8));
        assert_eq!(IconTwist::Plain.eye_shape(), None);
    }
}
