//! Deterministic generation and toy-scale orbital simulation.

use macroquad::prelude::{Color, Vec2, vec2};
use std::f32::consts::TAU;

/// High-level categories used by generation and rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Star,
    Rocky,
    GasGiant,
}

/// A circular orbit around an earlier body in the system list.
#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub parent: usize,
    pub radius: f32,
    pub period_days: f32,
    pub phase: f32,
}

/// A generated celestial body and its current simulation position.
#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    pub kind: BodyKind,
    pub radius: f32,
    pub color: Color,
    pub orbit: Option<Orbit>,
    pub position: Vec2,
}

/// User-tunable inputs to deterministic system generation.
#[derive(Clone, Copy, Debug)]
pub struct SystemSettings {
    pub seed: u64,
    pub planet_count: usize,
    pub spacing: f32,
}

impl Default for SystemSettings {
    /// Provides a compact system that fits comfortably in the initial view.
    fn default() -> Self {
        Self {
            seed: 0x5EED_CAFE,
            planet_count: 5,
            spacing: 1.0,
        }
    }
}

/// Generated bodies and the monotonically increasing simulation clock.
pub struct System {
    pub bodies: Vec<Body>,
    pub elapsed_days: f32,
}

impl System {
    /// Builds a repeatable system from the supplied settings.
    pub fn generate(settings: SystemSettings) -> Self {
        let mut random = Random::new(settings.seed);
        let star_radius = random.range(26.0, 36.0);
        let mut bodies = vec![Body {
            name: "Solis".to_owned(),
            kind: BodyKind::Star,
            radius: star_radius,
            color: Color::new(1.0, 0.78, 0.28, 1.0),
            orbit: None,
            position: Vec2::ZERO,
        }];
        let mut orbital_radius = star_radius + 62.0;

        for index in 0..settings.planet_count {
            let kind = if random.chance(0.28) {
                BodyKind::GasGiant
            } else {
                BodyKind::Rocky
            };
            let radius = match kind {
                BodyKind::GasGiant => random.range(16.0, 24.0),
                BodyKind::Rocky => random.range(8.0, 14.0),
                BodyKind::Star => unreachable!(),
            };
            orbital_radius += random.range(52.0, 76.0) * settings.spacing + radius;
            let period_days = orbital_period(orbital_radius, 34.0);
            bodies.push(Body {
                name: format!("{}-{}", syllable(&mut random), index + 1),
                kind,
                radius,
                color: body_color(kind, &mut random),
                orbit: Some(Orbit {
                    parent: 0,
                    radius: orbital_radius,
                    period_days,
                    phase: random.range(0.0, TAU),
                }),
                position: Vec2::ZERO,
            });
        }

        let mut system = Self {
            bodies,
            elapsed_days: 0.0,
        };
        system.update_positions();
        system
    }

    /// Advances the clock at a deliberately brisk exploration-friendly rate.
    pub fn advance(&mut self, real_seconds: f32) {
        self.elapsed_days += real_seconds * 8.0;
        self.update_positions();
    }

    /// Resolves body positions in parent-before-child order.
    fn update_positions(&mut self) {
        for index in 0..self.bodies.len() {
            let Some(orbit) = self.bodies[index].orbit else {
                self.bodies[index].position = Vec2::ZERO;
                continue;
            };
            let parent_position = self.bodies[orbit.parent].position;
            let angle = orbit.phase + TAU * self.elapsed_days / orbit.period_days;
            self.bodies[index].position =
                parent_position + vec2(angle.cos(), angle.sin()) * orbit.radius;
        }
    }
}

/// Uses Kepler's third-law relationship with a compressed gravitational scale.
fn orbital_period(radius: f32, gravitational_parameter: f32) -> f32 {
    TAU * (radius.powi(3) / gravitational_parameter).sqrt()
}

/// Chooses a legible procedural palette for a body category.
fn body_color(kind: BodyKind, random: &mut Random) -> Color {
    match kind {
        BodyKind::Rocky => Color::new(
            random.range(0.32, 0.72),
            random.range(0.28, 0.62),
            random.range(0.22, 0.52),
            1.0,
        ),
        BodyKind::GasGiant => Color::new(
            random.range(0.52, 0.88),
            random.range(0.45, 0.74),
            random.range(0.42, 0.68),
            1.0,
        ),
        BodyKind::Star => Color::new(1.0, 0.78, 0.28, 1.0),
    }
}

/// Produces a short pronounceable fragment for generated labels.
fn syllable(random: &mut Random) -> &'static str {
    const PARTS: [&str; 12] = [
        "Ari", "Bel", "Cor", "Dra", "Eli", "Fae", "Gan", "Hel", "Iri", "Jun", "Kai", "Lum",
    ];
    PARTS[random.index(PARTS.len())]
}

/// Small SplitMix64 generator keeps worlds stable without an extra dependency.
struct Random {
    state: u64,
}

impl Random {
    /// Initializes the stream from a user-visible system seed.
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Returns the next uniformly mixed integer in the stream.
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// Samples a floating-point value from the half-open interval.
    fn range(&mut self, minimum: f32, maximum: f32) -> f32 {
        let unit = (self.next_u64() >> 40) as f32 / (1_u64 << 24) as f32;
        minimum + (maximum - minimum) * unit
    }

    /// Samples an array index below the provided length.
    fn index(&mut self, length: usize) -> usize {
        self.next_u64() as usize % length
    }

    /// Returns true with the supplied probability.
    fn chance(&mut self, probability: f32) -> bool {
        self.range(0.0, 1.0) < probability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generation should be stable enough for seeds to identify shared systems.
    #[test]
    fn generation_is_deterministic() {
        let settings = SystemSettings::default();
        let first = System::generate(settings);
        let second = System::generate(settings);

        assert_eq!(first.bodies.len(), second.bodies.len());
        for (left, right) in first.bodies.iter().zip(second.bodies.iter()) {
            assert_eq!(left.name, right.name);
            assert_eq!(left.kind, right.kind);
            assert_eq!(left.radius, right.radius);
            assert_eq!(left.position, right.position);
        }
    }

    /// More distant circular orbits must take longer than inner ones.
    #[test]
    fn orbital_period_increases_with_radius() {
        assert!(orbital_period(200.0, 34.0) > orbital_period(100.0, 34.0));
    }

    /// A full period returns a body to its generated starting position.
    #[test]
    fn orbit_closes_after_one_period() {
        let mut system = System::generate(SystemSettings {
            planet_count: 1,
            ..SystemSettings::default()
        });
        let start = system.bodies[1].position;
        system.elapsed_days = system.bodies[1].orbit.unwrap().period_days;
        system.update_positions();

        assert!(start.distance(system.bodies[1].position) < 0.001);
    }
}
