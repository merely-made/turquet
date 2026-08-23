# T3 receipt: analytical ephemeris

**Date:** 2026-08-23
**Engine:** Turquet 0.3.0

## Position contract

The kernel-free production path now exposes the complete T3 position
boundary:

- typed TT geocentric apparent ecliptic positions for the Sun through Pluto;
- disclosed source-frame precession, light-time, solar deflection, annual
  aberration, and nutation stages;
- typed UT1 and caller-supplied polar motion for Earth rotation;
- WGS84 observer geometry with distinct topocentric true-equatorial and
  horizon frame markers;
- airless azimuth measured eastward from north, altitude, observer range, and
  runtime-owned Earth-orientation snapshot identity.

Atmospheric refraction remains caller policy. Event-time solving remains T4;
T3 supplies the positions and direction classification it will consume.

## Numerical evidence

The pre-existing geocentric sweep compared 112,137 samples across 1885-2099
with a DE440s-backed verifier. Its worst residual was 5 millidegrees. The
ordinary-test fixture commits 5,277 vectors and measures a worst residual of
3 millidegrees.

The observer fixture commits 90 NASA/JPL Horizons vectors using DE441:

- all ten supported bodies;
- Boston, Sydney, and Tromso;
- three epochs, including the 2024-04-08 total solar eclipse;
- apparent true-equator/equinox-of-date right ascension and declination;
- airless apparent azimuth and altitude;
- observer range and DUT1.

The observer cohort measures a worst angular residual of **0.001522 degrees**
(Moon from Tromso on 2026-08-13) and a worst range residual of
**0.000107636 AU** (Pluto from Boston on 2026-08-13), below the committed
0.010-degree and 0.001-AU gates.

Targeted tests additionally bracket the 2024 Mercury station and compare
lunar perigee and apogee samples. The high-latitude Tromso rows exercise the
observer transform away from temperate latitudes.

## Observer authority

`tests/vectors/observer_horizons.tsv` was generated from the official
Horizons API on 2026-08-23 with quantities 2, 4, 20, and 49, `AIRLESS`, and
EOP snapshot `eop.260821.p261117`. The regeneration command is:

```powershell
pwsh -File scripts/fetch_horizons_observer_vectors.ps1 > tests/vectors/observer_horizons.tsv
```

Horizons used polar motion from its EOP snapshot. The committed Turquet test
uses the supplied DUT1 and explicitly sets polar motion to zero. That
approximation is recorded rather than hidden; the measured angular residual
includes it.

## Executable gates

The completion gates are:

```powershell
cargo test --all-targets --all-features
cargo test --doc
$env:CARGO_TARGET_DIR = '<external-target>'
cargo package --allow-dirty
```

The default engine remains pure Rust, kernel-free, network-free, and free of
runtime filesystem data.
