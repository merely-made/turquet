# T4i conventional rise/set circumstances plan

**Status (2026-08-25): landed; archive after extraction.**

## Scope

Add a provider-neutral observer-relative event search for conventional rise and
set circumstances. The caller composes the refraction, limb, and horizon-dip
models. Results retain the complete policy, model identities, provider and
Earth-orientation provenance, and bounded TT interval.

This slice does not infer terrain, obstructions, civil-day ownership,
visibility, weather, or a general altitude-dependent atmospheric model.

## Phases

### 1. Explicit policy and replayable result

Define fixed-target refraction; center, fixed-angular, or physical upper-limb
selection; and level, constant, or spherical horizon-dip models. Preserve every
selected model and its physical parameters in the event result.

Done when a caller can reproduce the scalar root from the retained policy and
midpoint terms without a hidden convention.

### 2. Provider-neutral search

Search sampled signs of `airless center altitude + refraction + limb + dip`.
Use the existing topocentric transform, so a physical lunar limb is computed
from topocentric range and does not add horizontal parallax a second time.

Done when ascending and descending bounded roots preserve the existing
one-hour sampling ceiling, identity checks, and limited empty-result meaning.

### 3. Primary-source proof and public boundary

Exercise sea-level USNO Sun and Moon conventions through the committed
independent Horizons position fixture. Compare against the USNO one-day API's
minute values, document numerical assumptions, and state remaining proof gaps
without a stronger claim.

Done when fixture results are minute-bounded, the analytical lane is reported
separately, and README, ROADMAP, AUDIT, PROVENANCE, and a dated receipt agree.

## Findings

- **2026-08-25:** T4h already provides the provider-neutral airless
  topocentric altitude and keeps conventional policy open. Its observer state
  retains topocentric distance, so a physical-radius limb can be evaluated
  without another parallax correction.
- **2026-08-25:** USNO defines standard sea-level solar rise/set as a
  geometric center altitude of minus 50 arcminutes: 34 arcminutes fixed
  refraction plus a 16 arcminute average upper solar limb. USNO defines Moon
  events from geocentric coordinates with a parallax term; this implementation
  operates on topocentric coordinates, so only fixed refraction plus dynamic
  topocentric semidiameter is appropriate.

## Progress

- **2026-08-25:** All phases landed. The executable proof covers policy
  parity with airless crossings, fixed and spherical offsets, construction and
  evaluation errors, a Tromso empty control, and the USNO Sun/Moon examples
  through fixture and analytical lanes. Deferred: altitude-dependent
  meteorological refraction, terrain, obstruction, civil-day, and visibility.
