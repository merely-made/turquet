# T4e receipt: lunar eclipse circumstances

**Date:** 2026-08-24
**Engine:** Turquet 0.8.0
**Scope:** fifth T4 event slice; T4 remains open

## Contract landed

`lunar_eclipse_circumstances` composes the provider-neutral full-moon search
with a bounded refinement of minimum Moon-to-shadow-axis offset. It reports
the geocentric class, greatest geometry, and all applicable shadow contacts:

- penumbral: P1 and P4;
- partial: P1, U1, U4, and P4;
- total: P1, U1, U2, U3, U4, and P4.

Greatest eclipse and every contact are TT intervals bounded by the caller's
tolerance. The caller also selects a full circumstance span around the phase
root. Both endpoints must be outside the penumbra or the search returns
`EventError::CircumstanceSpanTooShort`. Results retain the phase interval,
span, provider model and runtime snapshot, and spherical geometry revision.

Revision 1 uses the IAU nominal solar radius, WGS84 equatorial Earth radius,
and mean lunar radius. It is atmosphere-free and spherical. It does not claim
observer visibility, atmospheric shadow enlargement, Earth oblateness,
terrain, or observer-relative solar contacts.

## Independent evidence

The committed fixture contains 162 NASA/JPL Horizons DE441 apparent
geocentric Sun and Moon positions at fifteen-minute spacing. It spans every
contact for one NASA-published event in each lunar class. The same circumstance
search runs over this independent provider and Turquet's analytical provider.

| Eclipse | Class | Greatest provider difference | Worst contact provider difference |
| --- | --- | ---: | ---: |
| 2024-03-25 | Penumbral | 1.758 s | 2.463 s |
| 2024-09-18 | Partial | 8.708 s | 22.595 s |
| 2025-03-14 | Total | 5.539 s | 7.913 s |

All greatest and contact intervals are at most one second wide. Across the
three events, the Horizons-derived greatest times remain within 7.339 seconds
of NASA's detailed plots and the analytical greatest times within 15.415
seconds. The providers agree on class and contact order.

NASA's plots use Danjon-enlarged shadow radii. Turquet's atmosphere-free
contacts consequently fall inside NASA's published eclipse duration. The
largest observed absolute difference is 225.004 seconds for the
Horizons-derived contacts and 247.598 seconds for analytical contacts. This is
a disclosed model difference; the provider-to-provider gate remains 30
seconds.

Regenerate the independent positions with:

```powershell
pwsh -File scripts/fetch_horizons_lunar_eclipse_circumstances.ps1 > tests/vectors/lunar_eclipse_circumstances_horizons.tsv
```

## Remaining T4 work

- observer-relative solar contacts, local eclipse type, and visibility;
- observer rise, transit, and set with explicit no-event intervals;
- general elongation/visibility and illuminated-fraction or distance extrema;
- event execution through a live kernel-backed `JplVerifier`.

## Gates

```powershell
cargo test --all-targets --all-features
cargo test --doc
$env:CARGO_TARGET_DIR = '<external-target>'
cargo package --allow-dirty
```
