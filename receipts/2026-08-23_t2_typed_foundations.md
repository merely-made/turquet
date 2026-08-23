# T2 typed-foundations receipt

Date: 2026-08-23

## Public contract

- `foundation::JulianDate<Scale>` is a two-part Julian Date with distinct TT
  and UT1 markers.
- angles, 0-2pi celestial longitudes, signed east-positive geographic
  longitudes, bounded latitudes, lengths, nonnegative distances, and geodetic
  observers validate construction.
- spherical directions, unit vectors, and rotations carry their reference
  frame as a type parameter.
- calculated states disclose model name/revision, bounded angular residual,
  evidence kind, epoch, frame, direction, and distance.
- the inherited anonymous-scalar catalogue is exposed through `compat`.

## IAU orientation

`orientation` calls the pure-Rust SOFARS 0.6.1 realization of IAU 2006
precession and IAU 2000A nutation. Turquet wraps it with typed TT inputs and
frame-safe outputs. Turquet is not SOFA software and is not endorsed by the
IAU SOFA Board. `PROVENANCE.md` records the dependency and applicable terms.

Published SOFA issue 2023-10-11 vectors are committed directly in
`tests/orientation.rs`:

- adjusted 2000A longitude and obliquity nutation agree within 1e-13 rad;
- all nine GCRS-to-true-equator matrix elements agree within 1e-12.

## Forcing consumer

`apparent` now accepts `JulianDate<TerrestrialTime>` and returns
`Modelled<State<TrueEclipticEquinoxOfDate>>`. `ApparentSky` evaluates the full
nutation series once per epoch and reuses it for the ten-body chart. The
committed DE440s cohort remains within its existing 10 millidegree gate.

## Verification gates

Run from a clean checkout with a dedicated target:

```text
cargo test --all-targets
cargo test --all-targets --all-features
cargo test --doc
cargo doc --no-deps --all-features
cargo package
```
