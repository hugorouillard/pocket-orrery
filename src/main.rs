//! Pocket Orrery's application loop and immediate-mode renderer.

mod simulation;

use macroquad::prelude::*;
use simulation::{BodyKind, System, SystemSettings};

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

    /// Applies mouse-wheel zoom while keeping the view within useful limits.
    fn update(&mut self) {
        let (_, wheel) = mouse_wheel();
        if wheel != 0.0 {
            self.zoom = (self.zoom * 1.18_f32.powf(wheel)).clamp(0.12, 8.0);
        }

        let mut direction = Vec2::ZERO;
        direction.x = axis(is_key_down(KeyCode::D), is_key_down(KeyCode::A));
        direction.y = axis(is_key_down(KeyCode::S), is_key_down(KeyCode::W));
        if direction.length_squared() > 0.0 {
            self.center += direction.normalize() * 320.0 * get_frame_time() / self.zoom;
        }
    }
}

/// Returns a signed input axis from positive and negative buttons.
fn axis(positive: bool, negative: bool) -> f32 {
    i8::from(positive) as f32 - i8::from(negative) as f32
}

/// Draws orbit guides behind the system bodies.
fn draw_orbits(system: &System, view: &View) {
    for body in &system.bodies {
        let Some(orbit) = body.orbit else {
            continue;
        };
        let parent = system.bodies[orbit.parent].position;
        let center = view.world_to_screen(parent);
        draw_circle_lines(
            center.x,
            center.y,
            orbit.radius * view.zoom,
            1.0,
            Color::new(0.28, 0.36, 0.52, 0.35),
        );
    }
}

/// Draws generated bodies with a minimum on-screen size at distant zoom levels.
fn draw_bodies(system: &System, view: &View) {
    for body in &system.bodies {
        let position = view.world_to_screen(body.position);
        let radius = (body.radius * view.zoom).max(3.0);
        if body.kind == BodyKind::Star {
            draw_circle(
                position.x,
                position.y,
                radius * 1.8,
                Color::new(1.0, 0.65, 0.2, 0.12),
            );
        }
        draw_circle(position.x, position.y, radius, body.color);
        draw_text(
            &body.name,
            position.x + radius + 5.0,
            position.y - radius,
            16.0,
            Color::new(0.78, 0.84, 0.95, 0.85),
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
    let mut view = View {
        center: Vec2::ZERO,
        zoom: 0.75,
    };
    let mut paused = false;

    loop {
        clear_background(BACKGROUND);

        if is_key_pressed(KeyCode::Space) {
            paused = !paused;
        }
        if is_key_pressed(KeyCode::R) {
            settings.seed = settings.seed.wrapping_add(1);
            system = System::generate(settings);
        }
        view.update();
        if !paused {
            system.advance(get_frame_time());
        }

        draw_orbits(&system, &view);
        draw_bodies(&system, &view);
        draw_text(
            "WASD pan  |  wheel zoom  |  Space pause  |  R regenerate",
            22.0,
            screen_height() - 24.0,
            20.0,
            Color::new(0.72, 0.78, 0.9, 0.9),
        );
        draw_text(
            format!(
                "SEED {:08X}   DAY {:.1}",
                settings.seed, system.elapsed_days
            ),
            22.0,
            34.0,
            20.0,
            WHITE,
        );

        next_frame().await;
    }
}
