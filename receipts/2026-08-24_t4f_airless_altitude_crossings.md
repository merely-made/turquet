# T4f receipt: airless altitude crossings

**Date:** 2026-08-24
**Engine:** Turquet 0.9.0
**Scope:** sixth T4 event slice; T4 remains open

## Contract landed

`airless_altitude_crossings` composes a `GeocentricPositionProvider`, an
epoch-indexed `EarthOrientationProvider`, a WGS84 observer, and a physical
altitude threshold. It returns ascending or descending sampled sign changes
as TT intervals bounded by the caller's tolerance.

Every result retains the observer, threshold, position-provider model and
snapshot, revision-1 airless topocentric transform, and Earth-orientation
authority and snapshot. Position, Earth-orientation, identity, and observer
transform failures remain distinct typed errors. The search accepts at most a
one-hour sampling step.

An empty vector means only that the selected samples contain no sign change.
It does not claim that a body is always above or below the threshold, that it
is circumpolar, or that a grazing event is absent. Atmospheric refraction,
horizon dip, terrain, apparent limb, and named civil rise/set policy are not
part of this event.

`ObserverTransform` is the provider-neutral projection extracted from
`ObserverSky`. The existing analytical wrapper and its 90-vector observer
cohort remain unchanged. `ConstantOffsetEarthOrientation` provides a disclosed
bounded-search approximation: UT1 advances with TT while UT1-minus-TT and
polar motion remain fixed.

## Independent evidence

The committed NASA/JPL Horizons fixture contains 867 five-minute rows from
API 1.2 and DE441. Each row retains apparent geocentric ecliptic coordinates,
range, DUT1, and direct quantity-4 topocentric elevation with
`APPARENT=AIRLESS`.

| Case | Expected result | Worst root residual against direct Horizons |
| --- | --- | ---: |
| Boston Sun, 2024-04-08 | ascending and descending | 0.232 s |
| Sydney Moon, 2024-04-08 | descending and ascending | 0.157 s |
| Tromso Sun, 2024-06-21 | empty midsummer control | no sampled crossing |

Both Turquet's analytical position provider and the committed DE441/Horizons
position provider stay inside the 2-second gate. Every returned interval is
at most one second wide. Horizons applies polar motion from EOP snapshot
`eop.260824.p261120`; Turquet's test records its explicit zero-polar-motion
approximation and interpolates the supplied DUT1 at every TT sample.

Regenerate the fixture with:

```powershell
pwsh -File scripts/fetch_horizons_altitude_crossing_vectors.ps1 > tests/vectors/altitude_crossings_horizons.tsv
```

## Remaining T4 work

- named rise/set and transit policy;
- grazing roots, altitude extrema, and persistent above/below classification;
- observer-relative solar contacts, local eclipse type, and visibility;
- general visibility and illuminated-fraction or distance extrema.

## Publication gates

```powershell
cargo test --all-targets --all-features
cargo test --doc
$env:CARGO_TARGET_DIR = '<external-target>'
cargo package --allow-dirty
```
