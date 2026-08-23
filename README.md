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
[`astro-rust`](https://github.com/saurvs/astro-rust). Version `0.2.0` introduces
Turquet's typed primary API; the original 2015-era surface remains under
`turquet::compat` for migration.

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

The T1 audit names every exported calculation and distinguishes measured,
corrected, example-only, and unverified surfaces. Five confirmed inherited
coordinate and lunar defects are repaired with SOFA, NASA, and independent
Meeus vectors. The inherited suite remains compatibility evidence rather than
a general accuracy claim; see [AUDIT.md](AUDIT.md) for the exact boundary.

T2 adds two-part Julian Dates parameterized by time scale, unit-safe angles,
distances, and observers, frame-parameterized directions and rotations,
modelled states with accuracy evidence, and IAU 2006 precession with IAU 2000A
nutation. Published SOFA vectors check the orientation path. Frame and time
scale mismatches in this API are type errors.

The first Turquet-era module is `apparent` (2026-08-13): apparent geocentric
ecliptic-of-date positions for the Sun, Moon, and eight planets through
Pluto, composed from inherited analytical series with explicit light-time,
aberration, IAU 2006/2000A nutation, and Pluto frame-precession stages, plus
explicit range errors. Measured against NASA/JPL
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

See [ROADMAP.md](ROADMAP.md) for the adoption gates,
[AUDIT.md](AUDIT.md) for the public calculation boundary, and
[PROVENANCE.md](PROVENANCE.md) for the inherited source and references.

## Current use

The primary path makes the TT input and true-ecliptic-of-date output part of
the types. `ApparentSky` reuses the full nutation calculation for every body at
one epoch:

```rust
use turquet::apparent::{ApparentBody, ApparentSky};
use turquet::foundation::{JulianDate, ScaleAwareEpoch, TerrestrialTime};

let utc = ScaleAwareEpoch::from_gregorian_utc(2026, 8, 23, 12, 0, 0, 0);
let tt = JulianDate::<TerrestrialTime>::from_epoch(utc);
let sky = ApparentSky::at(tt);
let moon = sky.position(ApparentBody::Moon)?;

println!("Moon longitude: {} deg", moon.value().direction().longitude().degrees());
println!("Moon distance: {} km", moon.value().distance().kilometers());
# Ok::<(), turquet::apparent::ApparentError>(())
```

The inherited anonymous-scalar API remains available as
`turquet::compat::{lunar, sun, coords, ...}`. It is a migration surface rather
than a second primary contract.

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

Turquet's own source is distributed under the [MIT License](LICENSE.md). The
original copyright and complete Git history are retained. The pure-Rust
`sofars` dependency is MIT-licensed and contains routines derived from IAU
SOFA under the additional SOFA terms reproduced in its package. Products that
use those routines should follow those acknowledgement and redistribution
terms; see [PROVENANCE.md](PROVENANCE.md).
