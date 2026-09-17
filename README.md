# Pocket Orrery

Pocket Orrery is a small procedural solar-system playground written in Rust. It compresses distance, time, and gravity into a toy scale while keeping the relationships that make orbits feel coherent: outer worlds take longer to orbit, moons use their parent's mass, eccentric bodies move along Kepler ellipses, and a launched ship inherits its world's velocity.

Everything exists in one continuous 2D space. There are no travel screens or world transitions.

## Features

- Deterministic systems identified by a visible seed
- Rocky planets, gas giants, moons, atmospheres, rings, and surface details
- Nested, low-eccentricity Kepler orbits with Hill-sphere-limited moons
- Inertial ship flight with thrust, gravity, braking, and surface collisions
- Smooth zoom from local terrain detail to the complete system
- Target cycling and off-screen navigation markers
- Live generator workbench for planet count, spacing, eccentricity, and moons

## Run

```sh
cargo run --release
```

The app requires a graphical Linux, macOS, or Windows session supported by Macroquad.

## Controls

| Input | Action |
| --- | --- |
| `W` / `S` | Forward / reverse thrust |
| `A` / `D` | Turn left / right |
| `Shift` | Inertial brake |
| Mouse wheel | Zoom |
| `Tab` | Select the next world |
| `Space` | Pause simulation |
| `R` | Generate the next seeded system |

The workbench can rebuild the current seed after tuning parameters, making one-variable comparisons easy.

## Physics Model

The model intentionally favors exploration over SI units. Orbital periods follow `T = 2π√(a³/GM)` with a shared toy gravitational constant. Elliptical positions solve Kepler's equation each frame. Moon orbits are generated inside a conservative fraction of the parent's Hill sphere. Ship gravity uses softened inverse-square attraction so tiny worlds remain landable at compressed radii.

This is not an n-body simulation: celestial paths are analytic and do not perturb one another. That tradeoff keeps generated systems stable, legible, and reproducible while the ship remains fully dynamic.

## Development

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

The simulation and flight modules include unit tests for deterministic generation, structural settings, orbit closure, period ordering, thrust, and collision response.
