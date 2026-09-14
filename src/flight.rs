//! Continuous ship movement, gravity, and surface collision handling.

use crate::simulation::{DAYS_PER_SECOND, System};
use macroquad::prelude::{Vec2, vec2};

pub const SHIP_RADIUS: f32 = 3.0;
const THRUST: f32 = 42.0;
const TURN_RATE: f32 = 2.7;
const GRAVITY: f32 = 360.0;
const MAX_SPEED: f32 = 240.0;

/// Per-frame pilot intent, separated from keyboard handling for testability.
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    pub thrust: f32,
    pub turn: f32,
    pub brake: bool,
}

/// The player's inertial state in the same world space as celestial bodies.
#[derive(Clone, Copy, Debug)]
pub struct Ship {
    pub position: Vec2,
    pub velocity: Vec2,
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
            heading: direction.y.atan2(direction.x) - std::f32::consts::FRAC_PI_2,
        }
    }

    /// Integrates pilot thrust and gravity, then resolves body intersections.
    pub fn update(&mut self, system: &System, controls: Controls, delta_seconds: f32) {
        let dt = delta_seconds.min(1.0 / 20.0);
        self.heading += controls.turn * TURN_RATE * dt;
        let forward = vec2(self.heading.cos(), self.heading.sin());
        let acceleration = forward * controls.thrust * THRUST + self.gravity(system);
        self.velocity += acceleration * dt;

        if controls.brake {
            self.velocity *= (-2.4 * dt).exp();
        }
        self.velocity = self.velocity.clamp_length_max(MAX_SPEED);
        self.position += self.velocity * dt;
        self.resolve_collisions(system);
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

    /// Forward thrust should increase velocity along the ship's heading.
    #[test]
    fn thrust_accelerates_ship_forward() {
        let system = System::generate(SystemSettings::default());
        let mut ship = Ship {
            position: vec2(10_000.0, 10_000.0),
            velocity: Vec2::ZERO,
            heading: 0.0,
        };
        ship.update(
            &system,
            Controls {
                thrust: 1.0,
                ..Controls::default()
            },
            0.01,
        );

        assert!(ship.velocity.x > 0.0);
    }

    /// Collision resolution must leave the ship outside a body's surface.
    #[test]
    fn collision_pushes_ship_outside_body() {
        let system = System::generate(SystemSettings::default());
        let body = &system.bodies[0];
        let mut ship = Ship {
            position: body.position,
            velocity: Vec2::ZERO,
            heading: 0.0,
        };
        ship.resolve_collisions(&system);

        assert!(ship.position.distance(body.position) >= body.radius + SHIP_RADIUS - f32::EPSILON);
    }
}
