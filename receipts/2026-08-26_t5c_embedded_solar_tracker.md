# T5c embedded solar-tracker receipt

Date: 2026-08-26

## Scope

`embedded/` is the standalone `turquet-embedded` package. It is a
dependency-free `#![no_std]` vector profile, not a compact ephemeris or a
hardware controller. It accepts finite, nonzero Cartesian directions,
normalizes them, and returns a desired Sun direction plus signed geometric
panel-incidence cosine.

The profile's frame is supplied by its host. The bridge uses Turquet's public
airless topocentric horizon direction, whose components are local north, east,
and up. The profile does not recalculate time scales, EOP, observer geometry,
or solar position.

## Gates

```powershell
$env:CARGO_HOME='C:\t\turquet-t5c-cargo-home'
$env:CARGO_TARGET_DIR='C:\t\turquet-t5c-target'
cargo check --manifest-path embedded/Cargo.toml -j 1
cargo test --manifest-path embedded/Cargo.toml -j 1
```

Results:

- the core `#![no_std]` check passed;
- three unit tests passed: normalization and invalid input, aligned/orthogonal/
  opposed incidence;
- one host-side bridge test passed: public `ObserverSky` analytical Sun output
  at explicit TT, UT1, WGS84 observer, and zero polar motion feeds the embedded
  profile; its frame conversion and dot product independently agree;
- no profile warnings occurred. The parent Turquet library emitted six
  pre-existing inherited-parenthesis warnings.

## Exclusions

The receipt does not claim sensor fusion, calibration, actuator commands,
travel limits, scheduling, control loops, weather, terrain, shading,
visibility, eye safety, or any decision to move a panel. Those policies stay
above the reusable embedded geometry boundary.
