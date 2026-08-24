# T4c receipt: provider-neutral lunar phases

**Date:** 2026-08-23
**Engine:** Turquet 0.6.0
**Scope:** third T4 event slice; T4 remains open

## Contract landed

`LunarPhase` names four roots of the Moon's apparent ecliptic longitude east
of the Sun:

| Phase | Target elongation |
| --- | ---: |
| New moon | 0 degrees |
| First quarter | 90 degrees |
| Full moon | 180 degrees |
| Last quarter | 270 degrees |

`ecliptic_longitude_lunar_phases` searches all four roots through any
`GeocentricPositionProvider`. Each `LunarPhaseEvent` retains the phase,
provider model and runtime snapshot, a caller-bounded TT interval, and the
great-circle Sun-Moon center separation at the interval midpoint.

The phase name states an ecliptic-longitude relation. Latitude remains visible
in the separation, so new moon does not imply a solar eclipse and full moon
does not imply a lunar eclipse.

## External evidence

Fred Espenak's NASA GSFC phase catalogue publishes the four April 2024 phase
times to the nearest minute. The committed Horizons DE441 fixture contains
apparent geocentric Sun and Moon positions ten minutes before, at, and after
each catalogue minute. Those 24 facts do not pass through Turquet's analytical
series.

Each search uses a ten-minute sampling step and a one-second time tolerance.
Offsets below are measured from the published NASA minute, whose own precision
is one minute:

| Phase | Horizons offset | Analytical offset | Provider difference | Horizons separation | Analytical separation |
| --- | ---: | ---: | ---: | ---: | ---: |
| Last quarter | -16.113 s | -14.356 s | 1.758 s | 89.999968 degrees | 89.999968 degrees |
| New moon | -8.496 s | -4.395 s | 4.102 s | 0.349376 degrees | 0.349584 degrees |
| First quarter | +7.324 s | +2.051 s | 5.273 s | 90.000026 degrees | 89.999975 degrees |
| Full moon | -1.465 s | +3.809 s | 5.273 s | 178.312841 degrees | 178.312631 degrees |

All eight returned intervals are at most one second wide. Both providers stay
within 20 seconds of every NASA catalogue minute, and their measured worst
difference is 5.273 seconds.

NASA GSFC permits reproduction of the catalogue data with acknowledgment.
The fixture header and `PROVENANCE.md` retain its source and credit.

Regenerate the external positions with:

```powershell
pwsh -File scripts/fetch_horizons_phase_vectors.ps1 > tests/vectors/lunar_phases_horizons.tsv
```

## Remaining T4 work

- eclipse contact and visibility geometry;
- observer rise, transit, and set with explicit no-event intervals;
- elongation/visibility and illuminated-fraction or distance extrema;
- execution of event searches through a live kernel-backed `JplVerifier`.

## Gates

```powershell
cargo test --all-targets --all-features
cargo test --doc
$env:CARGO_TARGET_DIR = '<external-target>'
cargo package --allow-dirty
```
