# Tropical ascendant and house geometry

**Status (2026-10-17):** implementation in progress.

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
