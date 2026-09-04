# T5c: embedded solar-tracker profile

**Date:** 2026-08-26
**Status:** landed 2026-08-26

## Scope

T5c proves that Turquet's canonical observer-relative Sun direction can feed
a small embedded calculation without copying an approximate solar ephemeris
into a second engine. The new `turquet-embedded` package is a separate,
dependency-free `no_std` profile. It accepts a normalized Sun direction and a
normalized panel normal in the same local north-east-up frame, then returns:

- the requested Sun-pointing unit direction;
- the signed geometric incidence cosine between the two directions.

The profile has no control loop, motors, limits, hysteresis, safety policy,
time conversion, observer/EOP calculation, weather, terrain, shading, or
visibility claim. Those remain caller or hardware concerns.

## Contract

`UnitVector::new([x, y, z])` validates finite, nonzero input and normalizes it.
`solve(sun_direction, panel_normal)` preserves the canonical Sun direction and
reports the clamped signed dot product in `[-1, 1]`. It owns only vector
geometry, so a host application chooses when and whether to move hardware.

The host-only bridge test obtains an airless north-zero/east-positive Sun
horizon direction through `ObserverSky`, converts its public unit-vector
components to the embedded type, and independently checks the dot product.
The embedded crate itself never depends on Turquet or `std`.

## Phases

### 1. Core-only profile

**Done when:** `embedded/` declares `#![no_std]`, no dependencies, validated
unit vectors, a signed-incidence result, and direct unit tests for alignment,
orthogonality, opposition, normalization, and invalid inputs.

### 2. Turquet bridge

**Done when:** a host-side integration test feeds a real public
`ObserverSky` Sun result into the embedded profile without reimplementing any
ephemeris or observer transform, and agrees with an independent dot product.

### 3. Acceptance and disclosure

**Done when:** the package passes a `no_std` check and its bridge test, the
roadmap and README distinguish the profile from control policy, and a dated
receipt names the input frame, model/provenance boundary, and exclusions.

## Findings

- **2026-08-26:** `ObserverSky::position(ApparentBody::Sun)` returns a public
  `Observation` with north-zero/east-positive airless horizon direction;
  `Direction::to_unit_vector().components()` exposes the required canonical
  local vector (`src/observer.rs`).
- **2026-08-26:** Turquet's main crate uses `std`, allocation, `hifitime`, and
  `sofars`, so a whole-engine `no_std` feature would misstate its actual
  boundary. The profile must be a separate core-only package.

## Progress

- **2026-08-26:** Started from clean `origin/main` worktree
  `C:\\t\\turquet-t5c` because the shared Turquet checkout contains unrelated
  concurrent edits.
- **2026-08-26:** Added the dependency-free `turquet-embedded` profile. Its
  `UnitVector` constructor uses scale-first normalization and a bounded
  no-`std` Newton square root; alignment, orthogonality, opposition, and
  invalid-input tests are all explicit.
- **2026-08-26:** `cargo check --manifest-path embedded/Cargo.toml -j 1`
  passed with `#![no_std]`. `cargo test --manifest-path embedded/Cargo.toml -j 1`
  passed 3 core tests and 1 host-side `ObserverSky` bridge test. The parent
  Turquet library emitted only pre-existing inherited-parenthesis warnings.

## Stop rule

Do not add a low-precision ephemeris, persistent state, sensor fusion,
calibration, actuator commands, scheduling, local time, visibility decisions,
weather or terrain terms, panel mechanics, or product UI. A future device
integration may choose those policies above this profile.
