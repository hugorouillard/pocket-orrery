//! Immediate-mode controls for drafting and rebuilding generated systems.

use crate::simulation::SystemSettings;
use macroquad::{
    prelude::{Vec2, screen_width, vec2},
    ui::{hash, root_ui, widgets},
};

const PANEL_WIDTH: f32 = 300.0;

/// Editable floating-point copies of discrete and continuous generator inputs.
pub struct Workbench {
    planet_count: f32,
    spacing: f32,
    eccentricity: f32,
    moon_abundance: f32,
}

impl Workbench {
    /// Starts the controls at the settings used by the current system.
    pub fn new(settings: SystemSettings) -> Self {
        Self {
            planet_count: settings.planet_count as f32,
            spacing: settings.spacing,
            eccentricity: settings.eccentricity,
            moon_abundance: settings.moon_abundance,
        }
    }

    /// Draws the panel and returns settings only when the pilot requests a build.
    pub fn draw(&mut self, current: SystemSettings) -> Option<SystemSettings> {
        let mut requested_seed = None;
        let position = vec2(screen_width() - PANEL_WIDTH - 18.0, 18.0);
        widgets::Window::new(
            hash!("generator-workbench"),
            position,
            vec2(PANEL_WIDTH, 292.0),
        )
        .label("SYSTEM WORKBENCH")
        .titlebar(true)
        .movable(false)
        .ui(&mut root_ui(), |ui| {
            ui.label(None, &format!("Seed  {:016X}", current.seed));
            ui.separator();
            ui.slider(
                hash!("planet-count"),
                "Planets",
                2.0..9.0,
                &mut self.planet_count,
            );
            ui.slider(
                hash!("spacing"),
                "Orbit spacing",
                0.72..1.36,
                &mut self.spacing,
            );
            ui.slider(
                hash!("eccentricity"),
                "Eccentricity",
                0.0..0.29,
                &mut self.eccentricity,
            );
            ui.slider(
                hash!("moon-abundance"),
                "Moon abundance",
                0.0..1.01,
                &mut self.moon_abundance,
            );
            ui.separator();
            if ui.button(Vec2::new(8.0, 220.0), "REBUILD SAME SEED") {
                requested_seed = Some(current.seed);
            }
            if ui.button(Vec2::new(158.0, 220.0), "NEW SEED") {
                requested_seed = Some(current.seed.wrapping_add(0x9E37_79B9));
            }
        });

        requested_seed.map(|seed| self.settings(seed))
    }

    /// Creates validated generation settings from the current draft controls.
    pub fn settings(&self, seed: u64) -> SystemSettings {
        SystemSettings {
            seed,
            planet_count: self.planet_count.round() as usize,
            spacing: self.spacing,
            eccentricity: self.eccentricity,
            moon_abundance: self.moon_abundance,
        }
    }
}
