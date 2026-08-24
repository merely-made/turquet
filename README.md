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
[`astro-rust`](https://github.com/saurvs/astro-rust). Version `0.7.0` provides
Turquet's typed geocentric and observer-relative analytical ephemeris and
its first provider-neutral event searches: bounded conjunctions, apparent
ecliptic-longitude stations, lunar quarter phases, and eclipse candidates.
The original 2015-era surface remains under `turquet::compat` for migration.

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

The `apparent` module provides apparent geocentric
ecliptic-of-date positions for the Sun, Moon, and eight planets through
Pluto, composed from inherited analytical series with explicit light-time,
solar deflection, aberration, IAU 2006 precession, and IAU 2000A nutation,
plus explicit range errors. The `observer` module adds WGS84 parallax,
separate typed TT and UT1 epochs, caller-supplied polar motion and snapshot
identity, topocentric true-equatorial output, and airless north-zero horizon
coordinates.

T3 is measured against 112,137 DE440s geocentric samples across 1885-2099 and
90 DE441/Horizons observer samples across Boston, Sydney, and Tromso. The
committed observer cohort's worst angular residual is 0.001522 degrees. Tests
also cover an eclipse, a Mercury station bracket, lunar perigee and apogee,
and a high-latitude site.

The T4 event API uses `GeocentricPositionProvider` for apparent
ecliptic-longitude conjunctions, stationary points, lunar quarter phases, and
eclipse candidates. Search step, time tolerance, and the station
velocity-difference span are explicit. Results are TT intervals carrying
provider model and runtime snapshot identity. Position failures remain
errors. Eclipse results also retain a named spherical geometry revision and
the angular terms that decided the candidate. Solar candidates mean a global
alignment is possible after observer parallax; local type and visibility are
not inferred. Lunar candidates are classified as penumbral, partial, or total
against an atmosphere-free spherical Earth shadow. Every algorithm is
exercised with the analytical engine and committed NASA/JPL Horizons facts.

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

Observer calculations use `ObserverSky` in the same epoch-scoped pattern.
The caller supplies an `EarthOrientation` snapshot because UT1 and polar
motion are observed facts rather than timeless constants. Outputs retain that
snapshot and expose topocentric right ascension/declination, azimuth/altitude,
and observer range. Atmospheric refraction remains an application-selected
policy rather than an implicit correction.

Event searches use `provider::AnalyticalEphemeris` by default and accept any
implementation of `GeocentricPositionProvider`. The opt-in `JplVerifier`
implements the same contract when the `verify` feature is enabled. T4 still
has open slices for eclipse contacts and local visibility, rise/set, general
visibility, and extrema.

## Verification

Turquet will use independent authorities according to the calculation:

- official IAU SOFA vectors for time and reference-frame transformations;
- JPL Development Ephemerides and Horizons for solar-system comparisons;
- published examples only as local regression fixtures, not sole proof;
- property and boundary tests for coordinate wraps, stations, eclipses, lunar
  distance extremes, and high-latitude observers.

JPL kernels and external implementations are verification inputs. The default
engine remains Rust-only and usable without a runtime kernel download.

## License

Turquet's own source is distributed under the [MIT License](LICENSE.md). The
original copyright and complete Git history are retained. The pure-Rust
`sofars` dependency is MIT-licensed and contains routines derived from IAU
SOFA under the additional SOFA terms reproduced in its package. Products that
use those routines should follow those acknowledgement and redistribution
terms; see [PROVENANCE.md](PROVENANCE.md).
