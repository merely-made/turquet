# T4k-b: geocentric distance extrema

Date: 2026-08-26

## Scope

Turquet 0.16.0 adds `events::geocentric_distance_extrema`: a provider-neutral,
sampled search for minima and maxima of one supported body's apparent
geocentric range. `GeocentricDistanceExtremumSearch` combines an existing typed
`SearchWindow` with a caller-selected full central-difference span. Each result
retains body, kind, bounded TT interval, evaluated midpoint range, span,
revisioned extrema model, and provider model/snapshot.

The classifier is `r(t + h/2) - r(t - h/2)`: negative-to-positive is a minimum
and positive-to-negative a maximum. A strict sampled bracket is bisected to the
caller tolerance. An exact zero needs opposite neighboring signs; a boundary
zero or flat zero plateau is deliberately omitted. The provider is queried half
the selected span beyond either window endpoint. A returned state with a
different TT epoch is a typed error.

This is a sampled apparent Earth-body range contract. It is not an orbital,
barycentric, topocentric, terrain, atmosphere, or visibility result. An empty
vector establishes only that the selected sampling found no bracketed reversal.

## Independent fixture

`tests/vectors/distance_extrema_horizons.tsv` has 77 six-hour NASA/JPL Horizons
API 1.2 / DE441 rows at geocenter `500@399`, captured on 2026-08-26 with EOP
file `eop.260825.p261121`. It covers Moon perigee, Moon apogee, and Mars close
approach. Each case's independent reference is a three-point parabola through
the surrounding raw Horizons ranges. Regenerate it with:

```powershell
pwsh -NoProfile -File scripts/fetch_horizons_distance_extrema_vectors.ps1 `
  > tests/vectors/distance_extrema_horizons.tsv
```

The fixture provider deliberately linearly interpolates the six-hour captured
states to exercise the public seam. It is not presented as an ephemeris. Raw
ecliptic fixture columns are IAU76/80 ecliptic-of-date; Turquet's analytical
path retains its separately disclosed IAU 2006/2000A orientation model. Range
is frame-independent in this contract.

## Measurements and gates

Across the three committed cases, the linear fixture adapter is within
4,735.327 seconds and 159.186 km of the three-point reference. The analytical
provider is within 368.322 seconds and 27.169 km. The focused test gates the
fixture lane at two hours and 200 km, and the analytical lane at 600 seconds
and 50 km. These are cohort-specific receipts, not global accuracy bounds.

## Verification

```text
cargo test --test distance_extrema -j 1 -- --nocapture
```

Passed: 5 tests. They cover off-grid extrema ordering and provenance, isolated
exact zeros, boundary zeros, plateaus, derivative-span validation, provider
errors, returned-state epoch mismatch, and all three independent cases. The
new Rust source (500 LOC), integration test (438 LOC), fixture support (200
LOC), and regeneration script (73 LOC) each remain below the 600-LOC guidance.

## Release gates

- `rustfmt --check src/events/distance_extrema.rs tests/distance_extrema.rs tests/support/distance_extrema_fixture.rs`: passed.
- `cargo test -j 1`: passed, including all integration and doctests.
- `cargo package --allow-dirty --no-verify`: passed; 150 files packaged.

## Deferred work

T4l remains caller-threshold twilight. T4m remains general visibility only when
a real consumer forces an explicit policy. T5b follows with Cleromancy's
interpretive projection, then T5c's bounded solar-tracker profile.
