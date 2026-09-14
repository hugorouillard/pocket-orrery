//! Pocket Orrery's application loop and immediate-mode renderer.

mod flight;
mod simulation;

use flight::{Controls, SHIP_RADIUS, Ship};
use macroquad::prelude::*;
use simulation::{BodyKind, System, SystemSettings};
use std::collections::VecDeque;

const BACKGROUND: Color = Color::new(0.025, 0.035, 0.075, 1.0);

/// Camera state measured in simulation units and pixels per unit.
struct View {
    center: Vec2,
    zoom: f32,
}

impl View {
    /// Converts a simulation-space point to screen coordinates.
    fn world_to_screen(&self, point: Vec2) -> Vec2 {
        (point - self.center) * self.zoom + vec2(screen_width(), screen_height()) * 0.5
    }

    /// Follows the ship smoothly and applies bounded mouse-wheel zoom.
    fn update(&mut self, target: Vec2) {
        let (_, wheel) = mouse_wheel();
        if wheel != 0.0 {
            self.zoom = (self.zoom * 1.18_f32.powf(wheel)).clamp(0.1, 8.0);
        }
        let follow = 1.0 - (-6.0 * get_frame_time()).exp();
        self.center = self.center.lerp(target, follow);
    }
}

/// Returns a signed input axis from positive and negative buttons.
fn axis(positive: bool, negative: bool) -> f32 {
    i8::from(positive) as f32 - i8::from(negative) as f32
}

/// Reads keyboard input into a frame-independent pilot command.
fn pilot_controls() -> Controls {
    Controls {
        thrust: axis(is_key_down(KeyCode::W), is_key_down(KeyCode::S)),
        turn: axis(is_key_down(KeyCode::D), is_key_down(KeyCode::A)),
        brake: is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift),
    }
}

/// Draws orbit guides behind the system bodies.
fn draw_orbits(system: &System, view: &View) {
    for body in &system.bodies {
        let Some(orbit) = body.orbit else {
            continue;
        };
        let parent = system.bodies[orbit.parent].position;
        let segments = 72;
        let mut previous = view.world_to_screen(parent + orbit.relative_position(0.0));
        for step in 1..=segments {
            let sample_time = orbit.period_days * step as f32 / segments as f32;
            let next = view.world_to_screen(parent + orbit.relative_position(sample_time));
            draw_line(
                previous.x,
                previous.y,
                next.x,
                next.y,
                1.0,
                Color::new(0.28, 0.36, 0.52, 0.28),
            );
            previous = next;
        }
    }
}

/// Draws generated bodies with atmosphere, terrain, and ring details.
fn draw_bodies(system: &System, view: &View) {
    for body in &system.bodies {
        let position = view.world_to_screen(body.position);
        let radius = (body.radius * view.zoom).max(3.0);
        if body.kind == BodyKind::Star {
            draw_circle(
                position.x,
                position.y,
                radius * (1.3 + body.mass / 1_000.0),
                Color::new(1.0, 0.65, 0.2, 0.12),
            );
        }
        if let Some(atmosphere) = body.atmosphere {
            let air = Color::new(
                atmosphere.color.r,
                atmosphere.color.g,
                atmosphere.color.b,
                0.16 + atmosphere.density * 0.22,
            );
            draw_circle(position.x, position.y, radius * 1.16, air);
        }
        if body.has_rings {
            draw_ellipse_lines(
                position.x,
                position.y,
                radius * 1.75,
                radius * 0.58,
                0.18,
                2.0,
                Color::new(0.82, 0.75, 0.62, 0.72),
            );
        }
        draw_circle(position.x, position.y, radius, body.color);
        draw_surface_details(body, position, radius);
        draw_text(
            &body.name,
            position.x + radius + 5.0,
            position.y - radius,
            16.0,
            Color::new(0.78, 0.84, 0.95, 0.85),
        );
    }
}

/// Adds seeded continents, craters, or cloud bands when a body is large enough.
fn draw_surface_details(body: &simulation::Body, position: Vec2, radius: f32) {
    if radius < 7.0 || body.kind == BodyKind::Star {
        return;
    }

    if body.kind == BodyKind::GasGiant {
        for band in -2..=2 {
            let y = position.y + band as f32 * radius * 0.28;
            let half_width = (radius.powi(2) - (y - position.y).powi(2)).sqrt();
            draw_line(
                position.x - half_width,
                y,
                position.x + half_width,
                y,
                (radius * 0.08).max(1.0),
                body.accent,
            );
        }
        return;
    }

    let mut state = body.terrain_seed;
    for _ in 0..7 {
        let angle = hash_unit(&mut state) * std::f32::consts::TAU;
        let distance = hash_unit(&mut state).sqrt() * radius * 0.62;
        let patch_radius = radius * (0.08 + hash_unit(&mut state) * 0.16);
        let offset = vec2(angle.cos(), angle.sin()) * distance;
        draw_circle(
            position.x + offset.x,
            position.y + offset.y,
            patch_radius,
            body.accent,
        );
    }
}

/// Produces a deterministic unit value for procedural rendering details.
fn hash_unit(state: &mut u64) -> f32 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut value = *state;
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    ((value ^ (value >> 31)) >> 40) as f32 / (1_u64 << 24) as f32
}

/// Draws a fading inertial trail and a heading-oriented triangular craft.
fn draw_ship(ship: &Ship, trail: &VecDeque<Vec2>, view: &View) {
    let mut previous: Option<Vec2> = None;
    for (index, point) in trail.iter().enumerate() {
        let screen = view.world_to_screen(*point);
        if let Some(from) = previous {
            let alpha = index as f32 / trail.len().max(1) as f32 * 0.42;
            draw_line(
                from.x,
                from.y,
                screen.x,
                screen.y,
                1.5,
                Color::new(0.3, 0.85, 1.0, alpha),
            );
        }
        previous = Some(screen);
    }

    let center = view.world_to_screen(ship.position);
    let size = (SHIP_RADIUS * view.zoom).clamp(6.0, 16.0);
    let forward = vec2(ship.heading.cos(), ship.heading.sin());
    let side = vec2(-forward.y, forward.x);
    draw_triangle(
        center + forward * size,
        center - forward * size * 0.7 + side * size * 0.62,
        center - forward * size * 0.7 - side * size * 0.62,
        Color::new(0.86, 0.95, 1.0, 1.0),
    );
    if is_key_down(KeyCode::W) {
        draw_triangle(
            center - forward * size * 1.35,
            center - forward * size * 0.65 + side * size * 0.3,
            center - forward * size * 0.65 - side * size * 0.3,
            ORANGE,
        );
    }
}

/// Configures a resizable antialiased desktop window.
fn window_conf() -> Conf {
    Conf {
        window_title: "Pocket Orrery".to_owned(),
        window_width: 1280,
        window_height: 800,
        window_resizable: true,
        sample_count: 4,
        ..Default::default()
    }
}

/// Runs the simulation and redraws the generated system each frame.
#[macroquad::main(window_conf)]
async fn main() {
    let mut settings = SystemSettings::default();
    let mut system = System::generate(settings);
    let mut ship = Ship::launch(&system);
    let mut view = View {
        center: ship.position,
        zoom: 1.5,
    };
    let mut trail: VecDeque<Vec2> = VecDeque::with_capacity(100);
    let mut paused = false;

    loop {
        clear_background(BACKGROUND);

        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
        }
        if is_key_pressed(KeyCode::R) {
            settings.seed = settings.seed.wrapping_add(1);
            system = System::generate(settings);
            ship = Ship::launch(&system);
            trail.clear();
        }
        if !paused {
            let delta = get_frame_time();
            system.advance(delta);
            ship.update(&system, pilot_controls(), delta);
            if trail
                .back()
                .is_none_or(|point| point.distance(ship.position) > 2.0)
            {
                if trail.len() == trail.capacity() {
                    trail.pop_front();
                }
                trail.push_back(ship.position);
            }
        }
        view.update(ship.position);

        draw_orbits(&system, &view);
        draw_bodies(&system, &view);
        draw_ship(&ship, &trail, &view);
        let (nearest_index, altitude) = ship.nearest_body(&system);
        draw_text(
            "W/S thrust  A/D turn  Shift brake  |  wheel zoom  Space pause  R regenerate",
            22.0,
            screen_height() - 24.0,
            20.0,
            Color::new(0.72, 0.78, 0.9, 0.9),
        );
        draw_text(
            format!(
                "SEED {:08X}   DAY {:.1}   SPEED {:>5.1}   {} +{:.0}",
                settings.seed,
                system.elapsed_days,
                ship.velocity.length(),
                system.bodies[nearest_index].name,
                altitude.max(0.0),
            ),
            22.0,
            34.0,
            20.0,
            WHITE,
        );

        next_frame().await;
    }
}
