# T4h receipt: airless horizon naming and meridian transits

**Date:** 2026-08-24
**Engine:** Turquet 0.11.0
**Scope:** eighth T4 event slice; T4 remains open

## Contract landed

`airless_rise_set_events` is a naming projection over
`airless_altitude_crossings`. An ascending caller-selected airless
center-altitude crossing is `Rise`; a descending crossing is `Set`. The result
retains its complete crossing, including the caller threshold and position,
transform, and Earth-orientation provenance, plus
`AIRLESS_RISE_SET_NAMING` revision 1.

This is not conventional civil sunrise, sunset, moonrise, or moonset. It
deliberately selects no atmospheric refraction, apparent limb, horizon dip,
terrain, obstruction, civil-day, or visibility policy. An empty result keeps
the crossing search's sampled meaning and does not prove persistent state.

`meridian_transits` is a separate provider-neutral event search. It samples
`cos(topocentric declination) * sin(local apparent hour angle)`, refines sign
changes with bisection, and classifies roots with
`cos(topocentric declination) * cos(local apparent hour angle)`: positive is
upper and negative is lower. A sign-changing result is a caller-bounded TT
interval. An exact interior sample needs signs on both sides; an exact search
boundary uses the existing altitude-crossing one-sided rule. A sampled zero
plateau and a zero classifier create no transit, so an exact celestial pole
does not acquire an invented right ascension or lower-transit label.

Each transit retains its body, upper/lower kind, interval, midpoint airless
altitude estimate, observer, transit model, position-provider identity,
topocentric transform model, and Earth-orientation authority/snapshot. The
altitude estimate is not a visibility claim or an interval-wide angular bound;
both kinds remain valid below the horizon.

## Local-meridian model

The search uses topocentric true-equator/equinox-of-date right ascension. Its
local apparent sidereal angle is IAU 2006/2000A `gst06a` plus SOFA `apio`'s
TIO-and-polar-motion-adjusted local longitude, rather than a raw geodetic
longitude. This keeps arbitrary caller-supplied UT1 and polar motion in the
same local-meridian definition as the observer transform.

The private seam matches IAU SOFA 2023 `iauApio`'s canonical nonzero-pole
validation vector: adjusted longitude
`-0.5278008060295995734` radians and local ERA
`2.617608903970400427` radians both pass a `1e-12`-radian gate.

## Independent evidence

The regenerated 867-row NASA/JPL Horizons API 1.2 / DE441 fixture contains
five-minute direct quantity-4 airless elevations, quantity-42 signed local
apparent hour angles, and quantity-49 DUT1. Direct transit references root the
quantity-42 sine and classify the cosine, including the signed-hour-angle wrap;
they do not route a Turquet transform or event solver through the oracle.

| Case and transit | Fixture provider / direct | Analytical provider / direct |
| --- | ---: | ---: |
| Boston Sun lower | 0.148 s | 0.148 s |
| Boston Sun upper | 0.077 s | 0.077 s |
| Sydney Moon upper | 0.220 s | 0.220 s |
| Sydney Moon lower | 0.205 s | 0.205 s |
| Tromso Sun upper | 0.176 s | 0.176 s |
| Tromso Sun lower | 0.125 s | 0.125 s |

All twelve comparisons meet the 1-second gate; every reported interval is at
most one second wide. Boston's lower solar transit is below the horizon.
Tromso's midsummer Sun has no named zero-degree airless rise/set crossing but
still has both meridian transits. Sydney Moon's upper transit is 213.281
seconds from its airless altitude maximum, demonstrating that transit is not an
altitude-extremum synonym.

The named Boston Sun 5-degree airless threshold control maps to `Rise` and
`Set` and stays within 0.176 and 0.380 seconds, respectively, of direct
quantity-4 references. It proves the public name follows caller threshold
rather than adopting a hidden civil convention.

Horizons applies polar motion from EOP snapshot `eop.260824.p261120`; the
fixture explicitly approximates its pole coordinates as zero and records that
approximation. The production event path still accepts and applies arbitrary
caller-supplied polar motion. Regenerate the shared fixture with:

```powershell
pwsh -File scripts/fetch_horizons_altitude_crossing_vectors.ps1 > tests/vectors/altitude_crossings_horizons.tsv
```

## Remaining T4 work

- conventional refracted/limb/horizon-dip rise-set policy;
- observer-relative solar contacts, local eclipse type, and visibility;
- general visibility and illuminated-fraction or distance extrema.

## Publication gates

- `rustfmt --edition 2015 --check src/events.rs src/observer.rs
  tests/altitude_crossings.rs`: passed.
- `git diff --check`: passed. Git reported only the repository's established
  LF-to-CRLF conversion notices, with no whitespace errors.
- `cargo test --all-targets --all-features`: passed. The direct Horizons
  altitude/transit integration target passed all 11 checks, and the complete
  target set passed.
- `cargo test --doc`: passed 3 of 3 doctests.
- `cargo package --allow-dirty`: passed after packaging and verifying 129
  source files.

The commands emitted existing warning families in unrelated legacy source and
test files; no warning or failure originated in a T4h path.
