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
[`astro-rust`](https://github.com/saurvs/astro-rust). Version `0.15.0` provides
Turquet's typed geocentric and observer-relative analytical ephemeris and
its first provider-neutral event searches: bounded conjunctions, apparent
ecliptic-longitude stations, lunar quarter phases, eclipse candidates, lunar
eclipse circumstances, and local solar-eclipse contacts, plus observer-relative
airless altitude crossings, named caller-threshold rise/set facts, extrema,
sampled threshold circumstances, and separate upper/lower meridian transits.
It also provides one provider-neutral, geocentric apparent lunar-illumination
fact for a selected TT epoch.
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
ecliptic-longitude conjunctions, stationary points, lunar quarter phases,
eclipses, and observer-relative altitude events. Altitude searches compose
that provider with an epoch-indexed `EarthOrientationProvider` and the same
WGS84 airless transform used by `ObserverSky`. They return sign-changing
crossings plus sampled, bracketed roots of an explicit central-difference
altitude derivative. A combined circumstance result distinguishes crossings,
tolerance-based grazing candidates, sampled above/below states, and unresolved
near-threshold cases without claiming continuous proof between samples.
Search controls are explicit, and results are bounded TT intervals carrying
position, transform, and Earth-orientation identities. Eclipse results retain
a named spherical geometry revision and the terms that decide their class.
Solar candidates mean a global alignment is possible after observer parallax.
`local_solar_eclipse_circumstances` separately composes one observer transform
for the Sun and Moon at each TT sample, then returns local partial, annular, or
total fixed-spherical-limb geometry with C1--C4 contacts and an airless upper
solar-limb horizon state at greatest eclipse. It is not a visibility-window
claim: lunar limb relief, refraction, terrain, obstruction, weather, eye
safety, and civil policy remain outside the result. Lunar searches refine greatest
eclipse and the P1/P4, U1/U4, and U2/U3 contacts appropriate to penumbral,
partial, and total events. Every landed algorithm is exercised with the
analytical engine and committed NASA/JPL Horizons facts; geocentric families
also run through a live caller-supplied JPL SPK kernel in the opt-in verifier.
`lunar_illumination_at` separately composes same-epoch geocentric apparent Sun
and Moon states into a named Sun-Moon-Earth triangle. It retains fraction,
elongation, phase angle, all three distances, and provider provenance, and
rejects a returned state with the wrong TT epoch. It is not a topocentric
illumination, lunar-limb, atmospheric, or visibility result.
Airless `Rise` and `Set` name only a caller-selected center-altitude crossing:
they do not select refraction, limb, horizon, terrain, civil, or visibility
policy. Meridian transits are separate roots of topocentric apparent local
hour angle, using the polar-motion-adjusted local meridian; an upper or lower
transit can occur below the horizon.
`conventional_rise_set_events` is a separate caller-composed contract: it
solves airless center altitude plus a fixed target refraction, selected upper
limb, and selected horizon dip. Its result retains the complete policy. The
USNO standard helpers select fixed 34-arcminute refraction and a fixed
16-arcminute solar upper limb; a physical-radius limb instead uses the
topocentric range and therefore adds no second lunar horizontal parallax.
Altitude-dependent meteorological refraction, terrain, obstruction, civil-day,
and visibility policy remain open.

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
let _epoch_for_civil_display = tt.to_epoch();
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
motion are observed facts rather than timeless constants. Provider-neutral
searches use `EarthOrientationProvider` to obtain those facts at every TT
sample. Outputs retain the data identity and expose topocentric right
ascension/declination, azimuth/altitude, and observer range. Atmospheric
refraction remains an application-selected policy rather than an implicit
correction.

Event searches use `provider::AnalyticalEphemeris` by default and accept any
implementation of `GeocentricPositionProvider`. The opt-in `JplVerifier`
implements the same contract when the `verify` feature is enabled. Providers
may disclose a shared measured `Accuracy`; the analytical provider reports
its external angular cohort while the default is honestly undisclosed. Typed
TT results convert back to a scale-aware epoch through
`JulianDate<TerrestrialTime>::to_epoch()` for consumer-side civil rendering.
T4 still
has bounded open slices for observer-relative solar contacts and local eclipse
visibility, illuminated-fraction and distance facts/extrema, named twilight,
and consumer-forced visibility windows. The first acceptance consumer is a
Sky-home daily timeline; Cleromancy and an embedded solar-tracker profile
follow the remaining event slices. That cross-repository consumer work is
owned by `turnstone/design_docs/2026-08-26_sky_home_timeline_plan.md`.

## Verification

Turquet will use independent authorities according to the calculation:

- official IAU SOFA vectors for time and reference-frame transformations;
- JPL Development Ephemerides and Horizons for solar-system comparisons;
- published examples only as local regression fixtures, not sole proof;
- property and boundary tests for coordinate wraps, stations, eclipses, lunar
  distance extremes, and high-latitude observers.

JPL kernels and external implementations are verification inputs. The default
engine remains Rust-only and usable without a runtime kernel download.

Maintainers can execute every landed event family through a caller-owned SPK
kernel and compare it with the analytical provider:

```text
cargo run --features verify --bin verify_events -- <kernel.bsp>
```

The command prints the provider revision and kernel SHA-256 and fails on a
partial cohort, class/contact disagreement, wide result interval, or exceeded
measured provider-difference gate.

## License

Turquet's own source is distributed under the [MIT License](LICENSE.md). The
original copyright and complete Git history are retained. The pure-Rust
`sofars` dependency is MIT-licensed and contains routines derived from IAU
SOFA under the additional SOFA terms reproduced in its package. Products that
use those routines should follow those acknowledgement and redistribution
terms; see [PROVENANCE.md](PROVENANCE.md).
