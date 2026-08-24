# T4a receipt: provider-neutral conjunction search

**Date:** 2026-08-23
**Engine:** Turquet 0.4.0
**Scope:** first T4 event slice; T4 remains open

## Contract landed

`GeocentricPositionProvider` is the shared TT-to-apparent-state seam. The
kernel-free `AnalyticalEphemeris` and opt-in `JplVerifier` implement it.
Provider errors propagate as event errors, so an unsupported epoch or missing
reference fact cannot become an empty search result.

Providers may also expose a runtime data snapshot. `JplVerifier` hashes its
caller-supplied kernel and each event retains that SHA-256 alongside the
static provider model. The analytical provider correctly records no external
snapshot.

`ecliptic_longitude_conjunctions` accepts two distinct bodies and a validated
`SearchWindow`. The caller selects the sampling step up to the safe one-day
ceiling and a positive refinement tolerance no larger than that step. Every
result includes:

- the two bodies;
- a TT interval bounded by the selected tolerance;
- great-circle angular separation at the interval midpoint;
- the position provider's model and runtime data-snapshot identity.

The search unwraps relative longitude continuously. Executable cases prove a
longitude wrap produces the intended conjunction while the discontinuity at
opposition does not produce a false event.

## External evidence

The committed fixture contains apparent geocentric Sun and Moon positions
from NASA/JPL Horizons DE441, observer-table quantities 20 and 31, at ten
minute spacing around the 2024-04-08 eclipse. NASA GSFC independently
publishes the ecliptic conjunction as 18:20:46.8 UT.

With a one-second requested time tolerance:

| Provider | Difference from NASA | Midpoint separation |
| --- | ---: | ---: |
| Horizons DE441 fixture | +4.469 s | 0.349373 degrees |
| Turquet analytical | +8.571 s | 0.349580 degrees |

The provider midpoints differ by 4.102 seconds. Both returned intervals are
at most one second wide. The external fixture is interpolation over committed
Horizons facts, not a second calculation through Turquet's series.

Regenerate those facts with:

```powershell
pwsh -File scripts/fetch_horizons_conjunction_vectors.ps1 > tests/vectors/eclipse_conjunction_horizons.tsv
```

## Remaining T4 work

- stationary-point roots rather than direction classification;
- quarter phases and illuminated-fraction extrema over the provider;
- eclipse contact and visibility geometry;
- observer rise, transit, and set with explicit no-event intervals;
- elongation/visibility and distance extrema;
- execution of event searches through a live kernel-backed `JplVerifier`.

## Gates

```powershell
cargo test --all-targets --all-features
cargo test --doc
$env:CARGO_TARGET_DIR = '<external-target>'
cargo package --allow-dirty
```
