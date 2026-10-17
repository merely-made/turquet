# Tropical ascendant and house geometry

**Status (2026-10-17):** supplier geometry qualified; Cleromancy consumer qualification in progress.

## Findings

2026-10-17: main `9d830e1` supplies typed TT/UT1 and IAU 2006/2000A orientation
but no house geometry. Cleromancy's sealed natal v1 excludes houses.

## Phases

1. Typed pure-Rust ascending horizon intersection, MC, Whole Sign and Equal
   cusps. Done when independent numerical fixtures, hemisphere/quadrant,
   cusp-boundary and degenerate/polar refusal tests pass.
2. Consumer intake. Cleromancy explicitly selects the system and supplies
   retained time/place plus disclosed Earth-rotation approximation. Existing
   v1 bytes/replay remain exact. Product-owned native receipts stay there.

## Progress

2026-10-17: Mark selected Whole Sign and Equal House first; no Placidus
substitution or silent location inference. Pure geometry remains Turquet-owned;
interpretation and admission remain consumer-owned.

2026-10-17: source `4c27a6b` passes 139 default tests/doctests (0 ignored), including 18 independent Swiss geometry rows at a 0.002-degree bound and typed-time compile-fail checks. Receipt: `receipts/2026-10-17_natal_houses_qualification.json`. Earlier failed generated-fixture literal build remains external diagnostic evidence.

2026-10-17: independent review found Equal cusp 1 could differ from its retained ascendant by a radian rounding ULP. The corrected edition retains the original Longitude exactly and tests house 12/1 immediately before/at/after that boundary. Consumer editions pin the immutable supplier revision. Earlier source and tests retain their identities.

2026-10-17: corrected source `e2f65c0` passes all 139 default tests/doctests, zero ignores, including the independent Swiss fixtures and exact-boundary regression. Earlier `4c27a6b` evidence remains unchanged. Consumer native qualification remains separately owned by Cleromancy.
