# T4d receipt: provider-neutral eclipse candidates

**Date:** 2026-08-24
**Engine:** Turquet 0.7.0
**Scope:** fourth T4 event slice; T4 remains open

## Contract landed

`eclipse_candidates` composes the lunar-phase search with apparent Sun and
Moon directions and distances from any `GeocentricPositionProvider`.

- A solar candidate means geocentric disk separation plus a conservative sum
  of solar and lunar horizontal parallax permits alignment somewhere on Earth.
- A lunar candidate compares the Moon with the spherical Earth umbra and
  penumbra at the full-moon interval midpoint, then reports penumbral, partial,
  or total intersection.

Each result retains the bounded TT phase interval, provider model and runtime
snapshot, geometry-model revision, and the angular terms used by its
predicate. Revision 1 is atmosphere-free and spherical. It does not claim
local contacts, solar local type, observer visibility, atmosphere, Earth
oblateness, or terrain.

## External evidence

The committed fixture contains 42 NASA/JPL Horizons DE441 apparent geocentric
Sun and Moon positions around seven NASA GSFC phase-catalog minutes. Five are
catalogued eclipses. One ordinary new moon and one ordinary full moon are
negative controls. Both the fixture provider and Turquet's analytical
provider return these same results:

| Phase root | NASA class | Turquet result | Horizons geometry in degrees | Provider root difference |
| --- | --- | --- | --- | ---: |
| 2024-03-25 full | Penumbral lunar | Penumbral lunar | axis 0.961109; Moon 0.245554; umbra 0.636824; penumbra 1.171033 | 1.172 s |
| 2024-04-08 new | Total solar | Solar candidate | separation 0.349376; Sun 0.266052; Moon 0.276665; parallax allowance 1.018147 | 4.102 s |
| 2024-04-23 full | Ordinary | Rejected | phase separation 178.312841 | 5.273 s |
| 2024-05-08 new | Ordinary | Rejected | phase separation about 2.816 degrees | not promoted |
| 2024-09-18 full | Partial lunar | Partial lunar | axis 1.005698; Moon 0.278462; umbra 0.759616; penumbra 1.289804 | 8.203 s |
| 2025-03-14 full | Total lunar | Total lunar | axis 0.318744; Moon 0.247936; umbra 0.644625; penumbra 1.180502 | 5.859 s |
| 2025-03-29 new | Partial solar | Solar candidate | separation 1.062545; Sun 0.266853; Moon 0.277526; parallax allowance 1.021316 | 12.891 s |

The 2025-03-29 disks do not overlap geocentrically: their angular radii sum
to 0.544379 degrees. Its 1.062545-degree center separation becomes a global
candidate only when observer parallax is included. This is the executable
proof that the solar result is global rather than geocentric.

Across the five accepted candidates, the measured worst analytical versus
Horizons phase-root difference is 12.891 seconds. The largest difference in a
reported angular term is 0.000741 degrees. All returned phase intervals are at
most one second wide.

NASA's 2025-03-14 detailed plot uses an enlarged Earth shadow for contact
prediction. It reports a 0.6537-degree umbral radius, while Turquet's explicit
atmosphere-free model reports 0.644625 degrees at the phase midpoint. This
difference is expected model scope, not hidden residual.

Regenerate the independent positions with:

```powershell
pwsh -File scripts/fetch_horizons_eclipse_vectors.ps1 > tests/vectors/eclipse_geometry_horizons.tsv
```

## Remaining T4 work

- eclipse greatest-event search, contacts, local solar type, and visibility;
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
