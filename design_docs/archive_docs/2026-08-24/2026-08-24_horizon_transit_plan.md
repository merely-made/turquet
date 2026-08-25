# Horizon and meridian-event plan

**Status (2026-08-25): landed**

## Target

Complete the smallest honest named-event layer over the landed observer facts:
airless geometric horizon rise/set, and separate topocentric upper/lower
meridian transits. Keep conventional refracted limb rise/set, landscape,
horizon dip, civil-day selection, visibility, and interpretation outside this
slice.

## Phase 1: contracts

- Add a named projection of the existing caller-threshold airless crossing
  contract. An ascending crossing is `Rise`; a descending one is `Set`. Its
  caller-selected airless center threshold stays visible. Do not call it a
  standard civil or refracted event.
- Add validated meridian-transit search controls and results. Each result is a
  bounded TT interval, upper or lower kind, observer, position-provider
  identity, airless topocentric transform model, and Earth-orientation
  authority/snapshot.
- State that a meridian transit is a root of local apparent sidereal time minus
  *topocentric apparent* right ascension, rather than an altitude extremum.

Done when callers can distinguish airless horizon facts from conventional
rise/set policy, and upper/lower transit from altitude maximum/minimum.

## Phase 2: provider-neutral solver

- Reuse the T4f sampled scalar-root machinery, its one-hour ceiling, and its
  typed position/EOP/identity/transform error boundary.
- Obtain topocentric apparent right ascension through `ObserverTransform`; use
  IAU 2006/2000A Greenwich apparent sidereal time with epoch-specific UT1 and
  the observer's east-positive longitude.
- Search sampled sign changes of the sine of local hour angle, refine by
  bisection, and classify the refined root from the cosine as upper or lower.
  Exact sample and boundary semantics match the existing crossing contract.
- Keep the legacy anonymous-scalar `compat::transit` API unchanged.

Done when both provider lanes agree on ordered upper/lower transits, every
result interval meets caller tolerance, and error/identity behavior is covered.

## Phase 3: independent evidence and publication

- Extend the committed NASA/JPL Horizons observer fixture with direct local
  apparent hour angle (quantity 42) alongside its direct airless elevation and
  DUT1 facts. Reuse its Boston Sun, Sydney Moon, and Tromso Sun cases for
  caller-threshold airless rise/set, upper/lower transit, and high-latitude
  controls.
- Derive direct transit references from the independently supplied local hour
  angle values, without routing a Turquet position or event solver through the
  reference path.
- Update provenance, architecture, audit, roadmap, README, receipt, release
  metadata, and this plan; run the usual all-target, doc, and package gates,
  then commit and push directly to `main`.

Done when direct Horizons references and both provider lanes establish the
bounded facts, remaining conventional/visibility work is explicit, and main is
synchronized.

## Findings

- **2026-08-24:** T4f already owns exact zero-airless-altitude crossings and
  direction. The named airless horizon projection must reuse it instead of
  creating a second threshold solver.
- **2026-08-24:** `Observation::equatorial()` exposes topocentric apparent
  right ascension on the true equator and equinox of date. `sofars::erst::gst06a`
  supplies the compatible Greenwich apparent sidereal angle from TT and UT1.
- **2026-08-24:** A read-only Horizons quantity-42 query confirms that the
  existing fixture can carry direct local apparent hour angle together with
  airless azimuth/elevation and DUT1. This is a stronger independent transit
  oracle than reconstructing the hour angle from a Turquet transform.
- **2026-08-24:** Ultra review found that a first hour-angle draft added raw
  geodetic longitude to GAST despite accepting polar motion elsewhere. The
  corrected path uses SOFA `apio`'s TIO/polar-motion-adjusted longitude with
  GAST, preserving the true-equator/equinox right-ascension basis. An IAU SOFA
  `iauApio` validation vector now checks that local-meridian seam directly.
- **2026-08-24:** Altitude extrema are not transit: parallax, changing
  declination, and Earth orientation can displace an altitude maximum from a
  meridian crossing.

## Progress

- **2026-08-24:** Re-read the active documentation policy, T4 roadmap and
  current T4f/T4g implementation on clean synchronized `main`.
- **2026-08-24:** Started independent Ultra contract, implementation-seam, and
  evidence audits before selecting the public names and fixture change.
- **2026-08-24:** Review settled a configurable caller threshold for named
  airless rise/set and a `cos(declination) * sin(hour angle)` transit root.
  The latter deliberately becomes a plateau at the celestial pole instead of
  manufacturing a meridian event.
- **2026-08-24:** Implemented the named crossing projection and separate
  topocentric transit search. The reusable hour-angle scalar retains the
  one-hour sampling and typed provider/EOP/transform boundaries; an exact-pole
  control treats its undefined right ascension as a zero plateau.
- **2026-08-24:** Regenerated the 867-row Horizons fixture with quantity 42's
  direct local apparent hour angle. The focused provider-lane check has so far
  measured a worst 0.220-second direct-transit residual and a 213.281-second
  separation between Sydney Moon's upper transit and its altitude maximum.
- **2026-08-24:** Reworked the local-hour-angle seam after Ultra review and
  passed the SOFA `iauApio` local-meridian vector. The direct Horizons transit
  check will be rerun before publication.
- **2026-08-25:** The corrected local-meridian path passed its fresh direct
  Horizons replay: all twelve provider/reference comparisons were within
  0.220 seconds. `cargo test --all-targets --all-features`, `cargo test --doc`,
  `rustfmt --edition 2015 --check`, `git diff --check`, and
  `cargo package --allow-dirty` passed. The completed plan is archived with
  its receipt; T4 remains open for the explicitly deferred conventional and
  visibility work.
