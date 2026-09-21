//! Deterministic generation and toy-scale orbital simulation.

use macroquad::prelude::{Color, Vec2, vec2};
use std::f32::consts::TAU;

/// Simulation days advanced for each real-time second.
pub const DAYS_PER_SECOND: f32 = 3.0;

/// High-level categories used by generation and rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyKind {
    Star,
    Rocky,
    GasGiant,
    Moon,
}

/// A mildly eccentric Kepler orbit around an earlier body in the system list.
#[derive(Clone, Copy, Debug)]
pub struct Orbit {
    pub parent: usize,
    pub semi_major_axis: f32,
    pub eccentricity: f32,
    pub argument: f32,
    pub period_days: f32,
    pub phase: f32,
}

impl Orbit {
    /// Resolves a relative position by iterating Kepler's equation.
    pub fn relative_position(&self, elapsed_days: f32) -> Vec2 {
        let mean_anomaly = self.phase + TAU * elapsed_days / self.period_days;
        let mut eccentric_anomaly = mean_anomaly;
        for _ in 0..4 {
            eccentric_anomaly -=
                (eccentric_anomaly - self.eccentricity * eccentric_anomaly.sin() - mean_anomaly)
                    / (1.0 - self.eccentricity * eccentric_anomaly.cos());
        }

        let local = vec2(
            self.semi_major_axis * (eccentric_anomaly.cos() - self.eccentricity),
            self.semi_major_axis
                * (1.0 - self.eccentricity.powi(2)).sqrt()
                * eccentric_anomaly.sin(),
        );
        rotate(local, self.argument)
    }
}

/// A thin visual atmosphere with density controlling its apparent depth.
#[derive(Clone, Copy, Debug)]
pub struct Atmosphere {
    pub color: Color,
    pub density: f32,
}

/// A generated celestial body and its current simulation position.
#[derive(Clone, Debug)]
pub struct Body {
    pub name: String,
    pub kind: BodyKind,
    pub radius: f32,
    pub mass: f32,
    pub color: Color,
    pub accent: Color,
    pub atmosphere: Option<Atmosphere>,
    pub terrain_seed: u64,
    pub has_rings: bool,
    pub orbit: Option<Orbit>,
    pub position: Vec2,
}

/// User-tunable inputs to deterministic system generation.
#[derive(Clone, Copy, Debug)]
pub struct SystemSettings {
    pub seed: u64,
    pub planet_count: usize,
    pub spacing: f32,
    pub eccentricity: f32,
    pub moon_abundance: f32,
}

impl Default for SystemSettings {
    /// Provides a compact system that fits comfortably in the initial view.
    fn default() -> Self {
        Self {
            seed: 0x5EED_CAFE,
            planet_count: 5,
            spacing: 1.0,
            eccentricity: 0.16,
            moon_abundance: 0.65,
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
        let star_mass = random.range(400.0, 560.0);
        let mut bodies = vec![Body {
            name: "Solis".to_owned(),
            kind: BodyKind::Star,
            radius: star_radius,
            mass: star_mass,
            color: Color::new(1.0, 0.78, 0.28, 1.0),
            accent: Color::new(1.0, 0.94, 0.64, 1.0),
            atmosphere: None,
            terrain_seed: random.next_u64(),
            has_rings: false,
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
                BodyKind::Star | BodyKind::Moon => unreachable!(),
            };
            let mass = match kind {
                BodyKind::GasGiant => radius.powi(2) * random.range(0.1, 0.16),
                BodyKind::Rocky => radius.powi(3) * random.range(0.0025, 0.0045),
                BodyKind::Star | BodyKind::Moon => unreachable!(),
            };
            orbital_radius += random.range(52.0, 76.0) * settings.spacing + radius;
            let period_days = orbital_period(orbital_radius, star_mass);
            let planet_name = format!("{}-{}", syllable(&mut random), index + 1);
            let color = body_color(kind, &mut random);
            let atmosphere = generate_atmosphere(kind, &mut random);
            let planet_index = bodies.len();
            bodies.push(Body {
                name: planet_name.clone(),
                kind,
                radius,
                mass,
                color,
                accent: vary_color(color, random.range(0.75, 1.25)),
                atmosphere,
                terrain_seed: random.next_u64(),
                has_rings: kind == BodyKind::GasGiant && random.chance(0.36),
                orbit: Some(Orbit {
                    parent: 0,
                    semi_major_axis: orbital_radius,
                    eccentricity: random.range(0.0, settings.eccentricity),
                    argument: random.range(0.0, TAU),
                    period_days,
                    phase: random.range(0.0, TAU),
                }),
                position: Vec2::ZERO,
            });

            let hill_radius = orbital_radius * (mass / (3.0 * star_mass)).cbrt();
            let natural_moon_limit = if kind == BodyKind::GasGiant { 3 } else { 2 };
            let moon_limit = (natural_moon_limit as f32 * settings.moon_abundance).round() as usize;
            let moon_count = if moon_limit == 0 {
                0
            } else {
                random.index(moon_limit + 1)
            };
            let mut moon_axis = radius + random.range(10.0, 15.0);
            for moon_index in 0..moon_count {
                if moon_axis + 6.0 >= hill_radius * 0.72 {
                    break;
                }
                let moon_radius = random.range(3.2, (radius * 0.38).max(4.0));
                let moon_color = body_color(BodyKind::Moon, &mut random);
                bodies.push(Body {
                    name: format!("{} {}", planet_name, roman_numeral(moon_index)),
                    kind: BodyKind::Moon,
                    radius: moon_radius,
                    mass: moon_radius.powi(3) * 0.003,
                    color: moon_color,
                    accent: vary_color(moon_color, 0.72),
                    atmosphere: if moon_radius > 4.5 && random.chance(0.15) {
                        generate_atmosphere(BodyKind::Rocky, &mut random)
                    } else {
                        None
                    },
                    terrain_seed: random.next_u64(),
                    has_rings: false,
                    orbit: Some(Orbit {
                        parent: planet_index,
                        semi_major_axis: moon_axis,
                        eccentricity: random.range(0.0, 0.08),
                        argument: random.range(0.0, TAU),
                        period_days: orbital_period(moon_axis, mass),
                        phase: random.range(0.0, TAU),
                    }),
                    position: Vec2::ZERO,
                });
                moon_axis += moon_radius + random.range(10.0, 16.0);
            }
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
        self.elapsed_days += real_seconds * DAYS_PER_SECOND;
        self.update_positions();
    }

    /// Estimates a body's world-space velocity in simulation units per day.
    pub fn velocity_per_day(&self, index: usize) -> Vec2 {
        let Some(orbit) = self.bodies[index].orbit else {
            return Vec2::ZERO;
        };
        const SAMPLE_DAYS: f32 = 0.01;
        let local_velocity = (orbit.relative_position(self.elapsed_days + SAMPLE_DAYS)
            - orbit.relative_position(self.elapsed_days - SAMPLE_DAYS))
            / (2.0 * SAMPLE_DAYS);
        self.velocity_per_day(orbit.parent) + local_velocity
    }

    /// Resolves body positions in parent-before-child order.
    fn update_positions(&mut self) {
        for index in 0..self.bodies.len() {
            let Some(orbit) = self.bodies[index].orbit else {
                self.bodies[index].position = Vec2::ZERO;
                continue;
            };
            let parent_position = self.bodies[orbit.parent].position;
            self.bodies[index].position =
                parent_position + orbit.relative_position(self.elapsed_days);
        }
    }
}

/// Uses Kepler's third-law relationship with a compressed gravitational scale.
fn orbital_period(radius: f32, parent_mass: f32) -> f32 {
    const TOY_GRAVITY: f32 = 40.0;
    TAU * (radius.powi(3) / (TOY_GRAVITY * parent_mass)).sqrt()
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
        BodyKind::Moon => {
            let value = random.range(0.38, 0.68);
            Color::new(value * 1.04, value, value * 0.92, 1.0)
        }
        BodyKind::Star => Color::new(1.0, 0.78, 0.28, 1.0),
    }
}

/// Optionally gives rocky worlds air and always gives giants deep clouds.
fn generate_atmosphere(kind: BodyKind, random: &mut Random) -> Option<Atmosphere> {
    if kind == BodyKind::Rocky && !random.chance(0.58) {
        return None;
    }
    let color = if random.chance(0.65) {
        Color::new(0.28, 0.68, 0.96, 1.0)
    } else {
        Color::new(0.92, 0.58, 0.28, 1.0)
    };
    Some(Atmosphere {
        color,
        density: random.range(0.35, 1.0),
    })
}

/// Scales RGB channels while preserving a fully opaque color.
fn vary_color(color: Color, amount: f32) -> Color {
    Color::new(
        (color.r * amount).min(1.0),
        (color.g * amount).min(1.0),
        (color.b * amount).min(1.0),
        1.0,
    )
}

/// Rotates a vector without coupling the simulation to a scene transform.
fn rotate(point: Vec2, angle: f32) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    vec2(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

/// Names the small generated moon sets without carrying a formatter.
fn roman_numeral(index: usize) -> &'static str {
    ["I", "II", "III", "IV"][index]
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
        (self.next_u64() % length as u64) as usize
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

    /// Seeded choices must not change when usize is 32-bit in WebAssembly.
    #[test]
    fn random_index_uses_the_full_sample() {
        assert_eq!(Random::new(123).index(12), 7);
    }

    /// Maximum moon abundance can produce a fourth moon around a gas giant.
    #[test]
    fn fourth_moon_has_a_name() {
        assert_eq!(roman_numeral(3), "IV");
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

    /// Tunable generation must honor planet count and preserve hierarchy order.
    #[test]
    fn generation_honors_structural_settings() {
        let settings = SystemSettings {
            planet_count: 8,
            eccentricity: 0.05,
            moon_abundance: 0.0,
            ..SystemSettings::default()
        };
        let system = System::generate(settings);
        let planet_count = system
            .bodies
            .iter()
            .filter(|body| matches!(body.kind, BodyKind::Rocky | BodyKind::GasGiant))
            .count();

        assert_eq!(planet_count, 8);
        assert!(!system.bodies.iter().any(|body| body.kind == BodyKind::Moon));
        for (index, body) in system.bodies.iter().enumerate() {
            if let Some(orbit) = body.orbit {
                assert!(orbit.parent < index);
                assert!(orbit.eccentricity <= settings.eccentricity);
            }
        }
    }
}
