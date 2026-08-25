# Altitude circumstances plan

**Status (2026-08-24): complete**

## Target

Extend provider-neutral airless altitude crossings with bounded local extrema
and an honest threshold-state summary. Keep every claim scoped to the selected
TT window, sampling step, time tolerance, and altitude tolerance. Do not turn
finite black-box sampling into a proof about unsampled motion, and keep
meridian transit and named rise/set policy outside this contract.

## Phase 1: circumstance contract

- Add a validated extremum search with an explicit full central-difference
  span, plus a circumstance search with threshold and positive angular
  tolerance.
- Return ordered local minima and maxima as bounded TT intervals with altitude
  evaluated at each interval midpoint.
- Return a sampled threshold state that distinguishes refined crossings,
  a sampled extremum within angular tolerance, all samples above, all samples
  below, and an unresolved near-threshold case.
- Retain the complete observer, position-provider, transform, Earth-orientation,
  search-window, and tolerance provenance even when there are no crossings or
  extrema.

Done when the types make sampled evidence and continuous proof impossible to
confuse, and transit, refraction, limb, terrain, horizon dip, and civil naming
remain separate.

## Phase 2: provider-neutral extrema solver

- Reuse the T4f altitude projection and composite typed error boundary.
- Define altitude slope by the sign of a caller-selected central difference,
  scan it with a dedicated one-hour ceiling, and refine bracketed slope roots
  by bisection without longitude unwrapping or a hidden unimodality assumption.
- Share the circumstance's altitude samples between crossing and state work;
  reuse its refined extrema directly during classification.
- Classify only from refined crossings, sampled values, and refined extrema;
  retain an explicit unresolved state when angular tolerance prevents a
  stronger sampled claim.

Done when direction, extrema ordering, boundary behavior, near-threshold
grazing, above/below sampled controls, and every provider error remain tested.

## Phase 3: evidence and publication

- Reuse the committed 867-row NASA/JPL Horizons altitude fixture and its direct
  five-minute airless elevations for Boston Sun, Sydney Moon, and Tromso Sun.
- Derive independent direct-Horizons minimum and maximum references and compare
  both analytical and committed-DE441 position-provider lanes.
- Update the audit, roadmap, provenance, provider architecture, README, release
  metadata, and a dated receipt.
- Run all targets and features, doctests, independent package verification,
  commit and push directly to `main`, and watch GitHub CI.

Done when both provider lanes meet measured time and altitude gates for every
ordinary extremum, the high-latitude state is represented without a global
claim, remaining named-event work is explicit, and `main` is synchronized.

## Findings

- **2026-08-24:** `airless_altitude_crossings` already owns the correct
  position/EOP/observer composition and one-hour sample ceiling. Circumstances
  should factor its private sampling rather than introduce another provider
  seam.
- **2026-08-24:** A `GeocentricPositionProvider` can vary arbitrarily between
  calls. Finite samples can support `AboveAtAllSamples` or
  `BelowAtAllSamples`, not an unconditional continuous-state proof.
- **2026-08-24:** Meridian transit is an hour-angle event. Apparent altitude
  extrema can shift away from it as declination, parallax, and Earth
  orientation evolve, so the two must remain distinct contracts.
- **2026-08-24:** Luna and Terra independently recommended central-difference
  roots with a retained derivative span. Compared with bounded ternary
  optimization, this exposes the approximation, has a sign-change root
  contract parallel to stationary-longitude events, and avoids an unrecorded
  unimodality assumption. Provider calls extend half the selected span beyond
  the search window and must remain documented.
- **2026-08-24:** Suggested `PersistentAbove` and `PersistentBelow` names were
  rejected during review. The public state will say `AboveAtAllSamples` or
  `BelowAtAllSamples`, retain a tolerance-based `GrazingCandidate`, and use
  `Unresolved` when sampled/refined facts do not support a stronger label.
- **2026-08-24:** The existing five-minute Horizons elevations support a
  stable three-point parabolic reference without another fixture. Across all
  six provider/case lanes, measured worst extremum residuals are 0.454 seconds
  in time and 0.000244 degrees in midpoint altitude.
- **2026-08-24:** Final review found that circumstance refinement necessarily
  repeats some epoch requests. The provider traits now state the repeatability
  requirement already assumed by every numerical event solver; state remains
  scoped to those repeatable evaluated facts rather than continuous proof.

## Progress

- **2026-08-24:** The completed T4f implementation, evidence, roadmap, audit,
  and documentation policy were re-read on clean synchronized `main`.
- **2026-08-24:** Luna and Terra began independent read-only audits of the
  numerical contract and smallest implementation seam.
- **2026-08-24:** Both audits were reviewed against live T4f code. No observer
  or provider changes are required; T4g belongs in `events` and its existing
  fixture tests.
- **2026-08-24:** `AltitudeExtremumSearch`, bounded derivative-root extrema,
  `AltitudeCircumstanceSearch`, and sampled threshold states landed locally.
  Crossing samples are reused inside circumstances; standalone crossings keep
  their 0.9 contract.
- **2026-08-24:** Boston Sun, Sydney Moon, and Tromso Sun each returned ordered
  minimum/maximum pairs through analytical and committed DE441 position lanes.
  Tromso controls exercise sampled above, sampled below, and grazing-candidate
  states.
- **2026-08-24:** Final read-only review found no root-bracketing defect. Its
  error-coverage finding was addressed with public extrema and circumstance
  tests for position, Earth-orientation, identity, and transform failures; an
  overbroad audit sentence was made exact.
- **2026-08-24:** All-target/all-feature tests, doctests, direct format checks
  for changed Rust files, diff checks, and independent package verification
  pass for Turquet 0.10.0. Existing warnings remain confined to inherited
  source. The plan is complete and archived; remaining T4 gates stay explicit
  in `ROADMAP.md`.
