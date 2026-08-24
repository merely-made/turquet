# Adoption roadmap

Turquet's first objective is a trustworthy celestial fact engine. Completing
the inherited catalogue of algorithms follows that foundation.

## T0: Founding baseline

- Preserve upstream history and MIT attribution.
- Establish the Turquet package and repository identity.
- Run the inherited suite on stable Rust in continuous integration.
- Record inherited capabilities, known defects, and unmeasured claims.

Done when the renamed crate builds and its inherited tests pass from a clean
checkout.

## T1: Correctness audit

- Reproduce and repair the open upstream coordinate and lunar defects.
- Inventory every public function by source, units, frame, time scale, range,
  and existing evidence.
- Add independent vectors for corrected behavior.

Done when no public function presents an unstated frame or unit as a verified
calculation.

**Completed 2026-08-23.** `AUDIT.md` inventories every exported calculation
and marks its source, quantities, frame/time contract, range, and actual
evidence. Upstream longitude and horizontal-declination defects 18 and 19
(also reported in 13) are repaired against IAU SOFA validation vectors. The
discarded lunar quarter term, missing eccentricity factor, and negative-index
lunation selection are repaired against NASA's phase catalog and an
independent Meeus implementation. The remaining inherited functions are
explicitly compatibility surfaces, not verified calculations; their typed
migration belongs to T2.

## T2: Typed foundations

- Introduce typed epochs, time scales, angles, distances, observers, frames,
  states, and accuracy metadata.
- Keep the inherited API in an explicitly named compatibility module until its
  consumers have migrated.
- Add IAU 2006 precession with IAU 2000A nutation and SOFA conformance vectors.

Done when invalid frame and time-scale combinations cannot enter the primary
API as interchangeable floating-point values.

**Completed 2026-08-23.** `foundation` supplies two-part
`JulianDate<Scale>`, unit-safe angles, distances, and observers,
frame-parameterized directions and rotations, modelled states, and explicit
accuracy evidence. `orientation` wraps the pure-Rust SOFARS IAU 2006/2000A
model and matches the published SOFA validation vectors. `apparent` is the
forcing consumer and returns typed true-ecliptic-of-date states; its
epoch-scoped `ApparentSky` reuses orientation work across a chart. The
anonymous inherited catalogue is reachable through `compat`.

## The verification lane

Turquet ships two position providers and only one of them is a dependency.

```text
Position provider
    +-- Analytical Turquet provider      (production; default feature set)
    +-- ANISE + DE440s verifier provider (opt-in `verify` feature)
```

The verifier exists so the analytical engine is measured against an
authority rather than against itself: it routes through ANISE and SOFA and
never touches Turquet's own series. It is maintainer tooling. The kernel is
supplied by the maintainer, its output is committed as vectors, and ordinary
builds and CI compare against those vectors without acquiring a kernel. The
default dependency tree contains `hifitime` for scale-aware epochs and
`sofars` for the production IAU orientation model; both are pure Rust.

```text
cargo run --features verify --bin verify_cohort -- <kernel.bsp> [step_days]
cargo run --features verify --bin verify_events -- <kernel.bsp>
```

## T3: Analytical ephemeris

- Complete the apparent geocentric and topocentric Sun, Moon, Mercury through
  Pluto pipeline.
- Apply light-time, aberration, deflection, precession, and nutation through
  explicit stages.
- Compare a broad date and observer cohort against JPL output.

Done when the Rust-only provider meets documented tolerances and every result
records its model and supported range.

**Date cohort measured 2026-08-13** (`receipts/2026-08-13_t3_cohort_de440s.md`):
112,137 body-samples across 1885 to 2099 at 7-day steps agree with the DE440s
oracle to within 5 millidegrees, every body except the Moon within 1, with
zero analytical or reference failures and every skip accounted as Pluto
beyond its declared range. The observer cohort, targeted stations, lunar
extremes, and eclipse instants remain open.

**Completed 2026-08-23.** `apparent` now discloses and applies light-time,
solar deflection, annual aberration, IAU 2006 precession, and IAU 2000A
nutation. `observer` composes the typed geocentric state with WGS84 site
geometry, separately typed TT and UT1, caller-supplied polar motion, and a
runtime-owned Earth-orientation snapshot. It returns observer-centered true
equatorial and airless north-zero horizon states. A committed 90-vector
DE441/Horizons cohort covers all ten bodies at three epochs from Boston,
Sydney, and Tromso; measured worst angular residual is 0.001522 degrees and
worst range residual is 0.000108 AU. The targeted suite includes the 2024
eclipse, a Mercury station bracket, lunar perigee/apogee samples, and the
high-latitude Tromso site. See
`receipts/2026-08-23_t3_analytical_ephemeris.md`.

## T4: Celestial events

- Complete the useful missing Meeus algorithms.
- Express conjunctions, stations, phases, eclipses, rise and set, visibility,
  and extrema as searches over a position provider.
- Report event intervals and uncertainty rather than isolated magic numbers.

Done when the same event algorithms operate over both analytical and external
verification providers.

**First slice completed 2026-08-23.** `provider` now defines the typed
`GeocentricPositionProvider` seam, implemented by the kernel-free analytical
engine and the opt-in JPL SPK verifier. `events` searches apparent
ecliptic-longitude conjunctions with configurable safe sampling and a caller
selected time tolerance. Results are TT intervals, retain provider model
and runtime snapshot identity, report midpoint angular separation, and
propagate provider failures.
The 2024 eclipse conjunction is exercised through both the analytical engine
and a committed DE441/Horizons fixture: their midpoints differ by 4.102
seconds, while the analytical result is 8.571 seconds from NASA's published
ecliptic conjunction. See
`receipts/2026-08-23_t4a_conjunction_search.md`.

**Second slice completed 2026-08-23.** `events` now finds sign-changing roots
of apparent ecliptic-longitude speed. The caller selects the central-difference
span, and results state the motion before and after the bounded root. The
2024-04-25 Mercury direct station agrees within 0.659 seconds between the
analytical engine and an hourly DE441/Horizons fixture. See
`receipts/2026-08-23_t4b_station_search.md`.

**Third slice completed 2026-08-23.** `events` now finds all four apparent
Moon-Sun ecliptic-longitude quarter angles in one search. April 2024 new moon,
first quarter, full moon, and last quarter are each checked through analytical
and Horizons providers against NASA GSFC's minute-resolution phase catalogue;
the measured worst provider difference is 5.273 seconds. See
`receipts/2026-08-23_t4c_lunar_phases.md`.

**Fourth slice completed 2026-08-24.** `events` now filters new and full moons
through named spherical eclipse candidate geometry. Solar candidates include
a conservative global observer-parallax allowance. Lunar candidates report
atmosphere-free penumbral, partial, or total shadow intersections. Analytical
and Horizons providers agree on five NASA-listed eclipses across every lunar
class and two solar geometries, and reject an ordinary new and full moon. See
`receipts/2026-08-24_t4d_eclipse_candidates.md`.

**Fifth slice completed 2026-08-24.** `events` now refines the geocentric
greatest event and every applicable lunar shadow contact to caller-bounded TT
intervals. The caller selects a full circumstance span and receives a typed
error when its endpoints do not bracket the penumbra. Across penumbral,
partial, and total eclipses, analytical and DE441/Horizons providers agree
within 8.708 seconds at greatest eclipse and 22.595 seconds at contacts. See
`receipts/2026-08-24_t4e_lunar_eclipse_circumstances.md`.

**Live-verifier gate completed 2026-08-24.** `verify_events` executes every
landed event family through both `AnalyticalEphemeris` and `JplVerifier` using
a caller-supplied SPK kernel. The official DE440s run compared 26 paired event
results. Measured worst differences were 12.891 seconds for event roots, 0.659
seconds for the Mercury station, 8.537 seconds for greatest eclipse, and
22.455 seconds for lunar contacts. Every interval remained at most one second
wide and both providers agreed on event classes and contact order. The kernel
stays outside the repository and ordinary CI. See
`receipts/2026-08-24_t4_live_kernel_events.md`.

**Sixth slice completed 2026-08-24.** `events` now composes any
`GeocentricPositionProvider` with an epoch-indexed
`EarthOrientationProvider`, the WGS84 observer, and the provider-neutral
airless transform. It returns ascending or descending crossings of a
caller-selected physical altitude as bounded TT intervals. Results retain
the observer, threshold, ephemeris identity, transform revision, and
Earth-orientation authority and snapshot. The one-hour sampling ceiling and
empty-result meaning are explicit: this contract finds sampled sign changes
and does not classify grazing or persistent above/below states. Both the
analytical and committed DE441/Horizons position providers agree with direct
Horizons airless altitude roots for Boston Sun and Sydney Moon pairs within
0.232 seconds; a Tromso midsummer Sun case is the empty control. See
`receipts/2026-08-24_t4f_airless_altitude_crossings.md`.

T4 remains open for observer-relative solar contacts and eclipse visibility,
named rise/set and transit policy, grazing and persistent altitude
classification, general visibility, and illuminated-fraction and distance
extrema.

## T5: Consumers and embedding

- Prove one legitimate astronomy consumer and one interpretive consumer.
- Add a bounded embedded profile for position and orientation calculations.
- Keep control policy, secrets, interpretation, and social authority outside
  the engine.

Done when two materially different consumers reuse the same celestial state
and derivation receipt without duplicating the calculation.
