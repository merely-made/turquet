# Airless altitude crossings plan

**Status (2026-08-24): complete**

## Target

Land the first provider-neutral observer event as bounded crossings of a
caller-selected airless topocentric altitude. Compose the existing
`GeocentricPositionProvider` with an epoch-indexed Earth-orientation source and
the measured WGS84 observer transform. Keep refraction, apparent limb,
terrain, horizon dip, transit, and interpretive rise/set conventions outside
this contract.

## Phase 1: epoch-indexed observer facts

- Add an `EarthOrientationProvider` contract whose input is typed TT and whose
  output supplies the corresponding typed UT1, polar motion, authority, and
  snapshot.
- Add a disclosed constant-offset implementation that advances UT1 at every
  requested TT epoch while retaining one declared polar-motion approximation.
- Extract a reusable observer transform from `ObserverSky` so a supplied
  geocentric state can produce the same typed topocentric equatorial and
  airless horizon observation.
- Preserve `ObserverSky` and its existing analytical model contract.

Done when the old 90-vector observer cohort passes unchanged, transform epoch
mismatches are rejected, and both analytical and fixture geocentric providers
can use the same observer projection.

## Phase 2: bounded altitude crossings

- Add a validated search wrapper with a physical altitude threshold and a
  dedicated one-hour maximum sample step.
- Return ascending or descending sign-changing roots as bounded TT intervals,
  retaining observer, threshold, ephemeris model/snapshot, topocentric model,
  and Earth-orientation authority/snapshot.
- Preserve ephemeris, Earth-orientation, and transform failures as distinct
  typed errors.
- Treat an empty result as no sampled sign crossing in the selected window.
  Do not infer always-above, always-below, grazing, or circumpolar state.

Done when synthetic tests cover both directions, exact boundary handling,
adjacent roots, empty/grazing behavior, validation, and every error source.

## Phase 3: independent evidence and publication

- Add a dated NASA/JPL Horizons fixture for ordinary Sun and Moon crossing
  pairs plus a high-latitude empty control. Record DE revision, API revision,
  EOP snapshot, sites, airless quantity, sample spacing, and regeneration
  command.
- Compare analytical and committed-Horizons geocentric providers with direct
  Horizons airless topocentric altitude brackets and derived reference roots.
- Update the audit, roadmap, provenance, provider architecture, README,
  release metadata, and a dated receipt without calling zero degrees a civil
  sunrise or sunset.
- Run all-feature tests, doctests, package verification, and GitHub CI.

Done when the independent cohort passes through both provider lanes, the
approximation boundary is reproducible, `main` is synchronized, and transit,
grazing/extrema classification, and observer-relative eclipse contacts remain
explicit future work.

## Findings

- **2026-08-24:** `ObserverSky` currently binds the measured WGS84 transform to
  `ApparentSky`; its transform can be factored without changing the existing
  analytical wrapper or 90-vector evidence.
- **2026-08-24:** Reusing one `EarthOrientation` across a search would freeze
  UT1 and produce incorrect terrestrial rotation. The source must be queried
  at every TT sample.
- **2026-08-24:** A direct search over `GeocentricPositionProvider` and
  `EarthOrientationProvider` is smaller than introducing a second position
  trait. It also runs unchanged over `JplVerifier` and committed fixtures.
- **2026-08-24:** JPL Horizons quantity 4 is topocentric apparent azimuth and
  elevation; `APPARENT=AIRLESS` leaves refraction out. Horizons' RAD rise/set
  convention is geometric and airless, but Turquet will compare direct
  altitude facts rather than inherit Horizons' rise/set naming or search
  policy.
- **2026-08-24:** A sign-change scan cannot prove a grazing root or a permanent
  above/below state. The first result contract reports sampled crossings only;
  extrema and stronger classifications remain separate work.
- **2026-08-24:** Across Boston Sun and Sydney Moon pairs, both analytical and
  committed DE441 position providers stayed within 0.232 seconds of roots
  interpolated from direct Horizons airless elevations. Tromso midsummer Sun
  remained the empty control.

## Progress

- **2026-08-24:** Luna and Terra re-audited `main` after the live-kernel gate.
  The direct two-provider search shape and one-hour sampling ceiling were
  selected for implementation.
- **2026-08-24:** `EarthOrientationProvider`, its constant-offset source, and
  the provider-neutral `ObserverTransform` landed without changing the
  analytical observer wrapper or its 90-vector evidence.
- **2026-08-24:** Bounded ascending and descending altitude crossings landed
  with separate position, Earth-orientation, identity, and transform errors.
  Exact sampled roots, grazing controls, boundary handling, validation, and
  empty results are covered.
- **2026-08-24:** The reproducible 867-row Horizons fixture, public audit,
  roadmap, provenance, architecture addendum, README, 0.9.0 metadata, and T4f
  receipt landed. All targets and features, doctests, and independent package
  verification passed.
