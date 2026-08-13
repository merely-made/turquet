# Turquet

**Many views of one sky.**

Turquet is a pure-Rust celestial fact engine. Given a time, observer, model,
and requested frame, it aims to produce positions, relations, events, and
explanations that can be inspected and reproduced.

The name is a historical variant of *torquetum*: an astronomical instrument
used to observe and convert between horizon, equatorial, and ecliptic
coordinates.

## Status

Turquet was founded in 2026 as a history-preserving adoption of Saurav
Sachidanand's MIT-licensed
[`astro-rust`](https://github.com/saurvs/astro-rust). The inherited algorithms
are useful, but the public API and verification surface are still those of the
original 2015-era library. Treat `0.1.x` as an audit and modernization series.

The inherited implementation currently includes:

- complete VSOP87D coefficient tables for the eight planets;
- a partial Chapront ELP-2000/82 lunar solution;
- analytical solar and Pluto positions;
- Julian dates, sidereal time, and delta-T approximations;
- coordinate transformations, precession, nutation, parallax, aberration,
  and atmospheric refraction;
- lunar phases, nodes, libration, planetary magnitudes, and selected physical
  ephemerides;
- selected satellite calculations for Jupiter and Saturn.

Existing tests pass on current stable Rust. That is a compatibility receipt,
not a general accuracy claim. Known upstream correctness reports and every
public calculation still require a systematic numerical audit.

The first Turquet-era module is `apparent` (2026-08-13): apparent geocentric
ecliptic-of-date positions for the Sun, Moon, and eight planets through
Pluto, composed entirely from inherited code with explicit light-time,
aberration, nutation, and Pluto frame-precession stages, plus a leap-second
UTC-to-TT conversion and explicit range errors. Measured against NASA/JPL
Horizons at J2000, the 2024 total solar eclipse, and 2026-08-13, every body
lands within 2 millidegrees, most exactly (`tests/apparent.rs`). That is a
T3 down payment measured at chart precision, not yet the broad-cohort T3
gate.

## Direction

Turquet is intended to serve several consumers without embedding their policy:

- legitimate astronomical and observational tools;
- local-first astrology applications;
- embedded devices such as solar trackers and celestial displays;
- deterministic simulations and procedural systems;
- signed, reproducible calculations shared between peers.

The engine owns celestial facts. Applications own interpretation, control
policy, secrets, social membership, and presentation.

Each mature calculation should disclose:

- input time scale and observer;
- output reference frame and units;
- model, coefficient, and data revision;
- supported date range and expected accuracy;
- the derivation required to explain and reproduce the result.

See [ROADMAP.md](ROADMAP.md) for the adoption gates and
[PROVENANCE.md](PROVENANCE.md) for the inherited source and references.

## Current use

The legacy API remains available while typed replacements are built:

```rust
use turquet::{lunar, sun};

let jde = 2_451_545.0;
let (sun_position, sun_distance_au) = sun::geocent_ecl_pos(jde);
let (moon_position, moon_distance_km) = lunar::geocent_ecl_pos(jde);

println!("Sun longitude: {} rad", sun_position.long);
println!("Sun distance: {sun_distance_au} AU");
println!("Moon longitude: {} rad", moon_position.long);
println!("Moon distance: {moon_distance_km} km");
```

The anonymous `f64` epoch and coordinate values above are legacy interfaces.
Typed epochs, frames, angles, distances, states, and uncertainty are adoption
work rather than compatibility aliases.

## Verification

Turquet will use independent authorities according to the calculation:

- official IAU SOFA vectors for time and reference-frame transformations;
- JPL Development Ephemerides and Horizons for solar-system comparisons;
- published examples only as local regression fixtures, not sole proof;
- property and boundary tests for coordinate wraps, stations, eclipses, and
  high-latitude observers.

JPL kernels and external implementations are verification inputs. The default
engine remains Rust-only and usable without a runtime kernel download.

## License

Turquet is distributed under the [MIT License](LICENSE.md). The original
copyright and complete Git history are retained.
