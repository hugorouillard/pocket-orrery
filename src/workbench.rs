//! Immediate-mode controls for drafting and rebuilding generated systems.

use crate::simulation::SystemSettings;
use macroquad::{
    prelude::{Vec2, screen_width, vec2},
    ui::{hash, root_ui, widgets},
};

const PANEL_WIDTH: f32 = 300.0;
const MIN_PLANETS: u32 = 1;
const MAX_PLANETS: u32 = 24;
const MIN_SPACING: f32 = 0.5;
const MAX_SPACING: f32 = 1.5;
const MIN_ECCENTRICITY: f32 = 0.0;
const MAX_ECCENTRICITY: f32 = 0.32;
const MIN_MOON_ABUNDANCE: f32 = 0.0;
const MAX_MOON_ABUNDANCE: f32 = 1.3;

/// Editable copies of discrete and continuous generator inputs.
pub struct Workbench {
    planet_count: u32,
    spacing: f32,
    eccentricity: f32,
    moon_abundance: f32,
}

impl Workbench {
    /// Starts the controls at the settings used by the current system.
    pub fn new(settings: SystemSettings) -> Self {
        Self {
            planet_count: settings
                .planet_count
                .clamp(MIN_PLANETS as usize, MAX_PLANETS as usize) as u32,
            spacing: settings.spacing.clamp(MIN_SPACING, MAX_SPACING),
            eccentricity: settings
                .eccentricity
                .clamp(MIN_ECCENTRICITY, MAX_ECCENTRICITY),
            moon_abundance: settings
                .moon_abundance
                .clamp(MIN_MOON_ABUNDANCE, MAX_MOON_ABUNDANCE),
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
            let mut planet_count = self.planet_count as f32;
            ui.slider(
                hash!("planet-count"),
                "Planets",
                MIN_PLANETS as f32..MAX_PLANETS as f32,
                &mut planet_count,
            );
            self.planet_count = planet_count.round() as u32;
            ui.slider(
                hash!("spacing"),
                "Orbit spacing",
                MIN_SPACING..MAX_SPACING,
                &mut self.spacing,
            );
            ui.slider(
                hash!("eccentricity"),
                "Eccentricity",
                MIN_ECCENTRICITY..MAX_ECCENTRICITY,
                &mut self.eccentricity,
            );
            ui.slider(
                hash!("moon-abundance"),
                "Moon abundance",
                MIN_MOON_ABUNDANCE..MAX_MOON_ABUNDANCE,
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
            planet_count: self.planet_count.clamp(MIN_PLANETS, MAX_PLANETS) as usize,
            spacing: self.spacing.clamp(MIN_SPACING, MAX_SPACING),
            eccentricity: self.eccentricity.clamp(MIN_ECCENTRICITY, MAX_ECCENTRICITY),
            moon_abundance: self
                .moon_abundance
                .clamp(MIN_MOON_ABUNDANCE, MAX_MOON_ABUNDANCE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_preserve_integer_planet_counts() {
        let mut workbench = Workbench::new(SystemSettings::default());
        workbench.planet_count = 17;

        assert_eq!(workbench.settings(42).planet_count, 17);
    }

    #[test]
    fn settings_clamp_inputs_to_supported_ranges() {
        let workbench = Workbench {
            planet_count: u32::MAX,
            spacing: f32::MAX,
            eccentricity: f32::MAX,
            moon_abundance: f32::MAX,
        };
        let settings = workbench.settings(42);

        assert_eq!(settings.planet_count, MAX_PLANETS as usize);
        assert_eq!(settings.spacing, MAX_SPACING);
        assert_eq!(settings.eccentricity, MAX_ECCENTRICITY);
        assert_eq!(settings.moon_abundance, MAX_MOON_ABUNDANCE);
    }

    #[test]
    fn continuous_defaults_are_centered_in_their_ranges() {
        let defaults = SystemSettings::default();
        let tolerance = f32::EPSILON;

        assert!((defaults.spacing - (MIN_SPACING + MAX_SPACING) / 2.0).abs() <= tolerance);
        assert!(
            (defaults.eccentricity - (MIN_ECCENTRICITY + MAX_ECCENTRICITY) / 2.0).abs()
                <= tolerance
        );
        assert!(
            (defaults.moon_abundance - (MIN_MOON_ABUNDANCE + MAX_MOON_ABUNDANCE) / 2.0).abs()
                <= tolerance
        );
    }
}
