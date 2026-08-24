# T4b receipt: provider-neutral station search

**Date:** 2026-08-23
**Engine:** Turquet 0.5.0
**Scope:** second T4 event slice; T4 remains open

## Contract landed

`StationSearch` combines a validated `SearchWindow` with a caller-selected
central-difference span. Both the event sampling step and the full velocity
span are limited to one TT day. The span must be finite and positive.

`ecliptic_longitude_stations` searches for sign-changing roots in apparent
ecliptic-longitude speed through any `GeocentricPositionProvider`. Each
`EclipticLongitudeStation` retains:

- the body and bounded TT interval;
- apparent ecliptic longitude at the interval midpoint;
- direct or retrograde motion on both sides of the root;
- the velocity-difference span;
- the provider model and optional runtime data snapshot.

Provider requests extend half the velocity span around every evaluated epoch.
An unsupported boundary remains a position error rather than becoming an
empty event list.

## External evidence

The committed external provider interpolates 37 hourly apparent geocentric
Mercury positions from NASA/JPL Horizons DE441. The rows cover 2024-04-24
18:00 through 2024-04-26 06:00 UTC and use observer-table quantities 20 and
31. They do not pass through Turquet's analytical series.

The receipt searches 2024-04-25 with a three-hour sampling step, six-hour
central-difference span, and one-second time tolerance. Both providers find
one reversal from retrograde to direct motion:

| Provider | Midpoint UTC | Longitude |
| --- | ---: | ---: |
| Horizons DE441 fixture | 2024-04-25 12:54:10.1 | 15.9812005 degrees |
| Turquet analytical | 2024-04-25 12:54:09.4 | 15.9812446 degrees |

The provider midpoints differ by 0.659 seconds and the midpoint longitudes by
0.0000441 degrees. Both returned intervals are at most one second wide.

This is a precisely named apparent ecliptic-longitude station under the stated
six-hour numerical difference. It is not a claim that every catalogue's
unstated definition of a generic "Mercury station" must produce the same
instant.

Regenerate the external facts with:

```powershell
pwsh -File scripts/fetch_horizons_station_vectors.ps1 > tests/vectors/mercury_station_horizons.tsv
```

## Remaining T4 work

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
