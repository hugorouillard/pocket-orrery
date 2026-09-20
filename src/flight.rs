//! Continuous ship movement, gravity, and surface collision handling.

use crate::simulation::{DAYS_PER_SECOND, System};
use macroquad::prelude::Vec2;

pub const SHIP_RADIUS: f32 = 3.0;
const THRUST: f32 = 42.0;
const TURN_RATE: f32 = 12.0;
const GRAVITY: f32 = 360.0;
const MATCH_ACCELERATION: f32 = 64.0;
const MAX_SPEED: f32 = 240.0;

/// Per-frame pilot intent, separated from keyboard handling for testability.
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    pub thrust_direction: Vec2,
    pub brake: bool,
    pub match_velocity: Option<Vec2>,
}

/// The player's inertial state in the same world space as celestial bodies.
#[derive(Clone, Copy, Debug)]
pub struct Ship {
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub heading: f32,
}

impl Ship {
    /// Places the ship just above the first planet with matching orbital speed.
    pub fn launch(system: &System) -> Self {
        let host_index = usize::from(system.bodies.len() > 1);
        let host = &system.bodies[host_index];
        let direction = host.position.normalize_or_zero();
        let direction = if direction == Vec2::ZERO {
            Vec2::X
        } else {
            direction
        };
        Self {
            position: host.position + direction * (host.radius + 12.0),
            velocity: system.velocity_per_day(host_index) * DAYS_PER_SECOND,
            acceleration: Vec2::ZERO,
            heading: direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2,
        }
    }

    /// Integrates pilot thrust and gravity, then resolves body intersections.
    pub fn update(&mut self, system: &System, controls: Controls, delta_seconds: f32) {
        let dt = delta_seconds.min(1.0 / 20.0);
        let previous_velocity = self.velocity;
        let thrust_direction = controls.thrust_direction.normalize_or_zero();
        if thrust_direction != Vec2::ZERO {
            let target_heading = thrust_direction.y.atan2(thrust_direction.x);
            let heading_delta = (target_heading - self.heading + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.heading += heading_delta.clamp(-TURN_RATE * dt, TURN_RATE * dt);
        }
        let thrust = if thrust_direction == Vec2::ZERO {
            Vec2::ZERO
        } else {
            Vec2::new(self.heading.cos(), self.heading.sin()) * THRUST
        };
        let acceleration = thrust + self.gravity(system);
        self.velocity += acceleration * dt;

        if controls.brake {
            self.velocity *= (-2.4 * dt).exp();
        }
        if let Some(target_velocity) = controls.match_velocity {
            let correction = target_velocity - self.velocity;
            self.velocity += correction.clamp_length_max(MATCH_ACCELERATION * dt);
        }
        self.velocity = self.velocity.clamp_length_max(MAX_SPEED);
        self.position += self.velocity * dt;
        self.resolve_collisions(system);
        self.acceleration = if dt > 0.0 {
            (self.velocity - previous_velocity) / dt
        } else {
            Vec2::ZERO
        };
    }

    /// Finds the closest body by distance above its visible surface.
    pub fn nearest_body(&self, system: &System) -> (usize, f32) {
        system
            .bodies
            .iter()
            .enumerate()
            .map(|(index, body)| (index, self.position.distance(body.position) - body.radius))
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .unwrap_or((0, 0.0))
    }

    /// Sums softened inverse-square attraction from every generated body.
    fn gravity(&self, system: &System) -> Vec2 {
        system.bodies.iter().fold(Vec2::ZERO, |total, body| {
            let offset = body.position - self.position;
            let distance_squared = offset.length_squared().max(body.radius.powi(2));
            total + offset.normalize_or_zero() * GRAVITY * body.mass / distance_squared
        })
    }

    /// Pushes the ship out of surfaces and bounces only its inward velocity.
    fn resolve_collisions(&mut self, system: &System) {
        for (index, body) in system.bodies.iter().enumerate() {
            let offset = self.position - body.position;
            let minimum_distance = body.radius + SHIP_RADIUS;
            if offset.length_squared() >= minimum_distance.powi(2) {
                continue;
            }

            let normal = if offset == Vec2::ZERO {
                Vec2::X
            } else {
                offset.normalize()
            };
            self.position = body.position + normal * minimum_distance;
            let body_velocity = system.velocity_per_day(index) * DAYS_PER_SECOND;
            let relative_velocity = self.velocity - body_velocity;
            let inward_speed = relative_velocity.dot(normal);
            if inward_speed < 0.0 {
                self.velocity -= normal * inward_speed * 1.25;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::SystemSettings;
    use macroquad::prelude::vec2;

    /// Directional thrust should work equally in all eight input directions.
    #[test]
    fn thrust_accelerates_ship_in_eight_directions() {
        let system = System::generate(SystemSettings::default());
        let directions = [
            vec2(0.0, -1.0),
            vec2(1.0, -1.0),
            vec2(1.0, 0.0),
            vec2(1.0, 1.0),
            vec2(0.0, 1.0),
            vec2(-1.0, 1.0),
            vec2(-1.0, 0.0),
            vec2(-1.0, -1.0),
        ];

        for direction in directions {
            let position = vec2(10_000.0, 10_000.0);
            let target_heading = direction.y.atan2(direction.x);
            let gravity = Ship {
                position,
                velocity: Vec2::ZERO,
                acceleration: Vec2::ZERO,
                heading: target_heading,
            }
            .gravity(&system);
            let mut ship = Ship {
                position,
                velocity: Vec2::ZERO,
                acceleration: Vec2::ZERO,
                heading: target_heading,
            };
            ship.update(
                &system,
                Controls {
                    thrust_direction: direction,
                    ..Controls::default()
                },
                0.01,
            );

            let thrust_velocity = ship.velocity - gravity * 0.01;
            let expected_velocity = direction.normalize() * THRUST * 0.01;
            assert!(thrust_velocity.distance(expected_velocity) < 0.0001);
            assert!(
                ship.acceleration
                    .distance(direction.normalize() * THRUST + gravity)
                    < 0.001
            );
            assert!((ship.heading - target_heading).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn ship_turns_toward_thrust_without_snapping() {
        let system = System::generate(SystemSettings::default());
        let position = vec2(10_000.0, 10_000.0);
        let gravity = Ship {
            position,
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            heading: 0.0,
        }
        .gravity(&system);
        let mut ship = Ship {
            position,
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            heading: 0.0,
        };

        ship.update(
            &system,
            Controls {
                thrust_direction: -Vec2::Y,
                ..Controls::default()
            },
            0.01,
        );

        assert!((ship.heading + TURN_RATE * 0.01).abs() < f32::EPSILON);
        assert!(ship.heading > -std::f32::consts::FRAC_PI_2);
        let thrust_velocity = ship.velocity - gravity * 0.01;
        let facing = Vec2::new(ship.heading.cos(), ship.heading.sin());
        assert!(thrust_velocity.distance(facing * THRUST * 0.01) < 0.0001);
    }

    /// Collision resolution must leave the ship outside a body's surface.
    #[test]
    fn collision_pushes_ship_outside_body() {
        let system = System::generate(SystemSettings::default());
        let body = &system.bodies[0];
        let mut ship = Ship {
            position: body.position,
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
            heading: 0.0,
        };
        ship.resolve_collisions(&system);

        assert!(ship.position.distance(body.position) >= body.radius + SHIP_RADIUS - f32::EPSILON);
    }

    /// Velocity matching should steadily reduce speed relative to its target.
    #[test]
    fn velocity_matching_reduces_relative_speed() {
        let system = System::generate(SystemSettings::default());
        let target_velocity = vec2(18.0, -7.0);
        let mut ship = Ship {
            position: vec2(10_000.0, 10_000.0),
            velocity: vec2(-24.0, 12.0),
            acceleration: Vec2::ZERO,
            heading: 0.0,
        };
        let initial_error = ship.velocity.distance(target_velocity);
        ship.update(
            &system,
            Controls {
                match_velocity: Some(target_velocity),
                ..Controls::default()
            },
            0.05,
        );

        assert!(ship.velocity.distance(target_velocity) < initial_error);
    }
}
