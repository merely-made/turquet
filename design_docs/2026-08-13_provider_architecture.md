# Provider architecture and the kernel-free default

**Date:** 2026-08-13
**Scope:** reconciles the maintainer's composition analysis (kernel-free,
Rust-owned default; ANISE as verifier) against the measured state at founding,
and fixes the two-provider architecture for T2 through T4.

## The target, as the maintainer put it

A kernel-free, Rust-owned default, not merely "pure Rust." ANISE itself is
Rust; the non-self-contained part is the DE440s binary kernel. The analytical
engine becomes production; ANISE remains available to verification. JPL and
SOFA stay external authorities rather than runtime owners.

## Where the analysis was already overtaken

The analysis predated the founding measurements by hours, and two of its
assumptions dissolved in Turquet's favor:

1. **It assumed the production path needs the `vsop87` and `sofars` crates.**
   Measured: Turquet's inherited VSOP87D tables match the `vsop87` crate's
   output exactly at millidegree rounding, and its inherited Meeus nutation
   and ecliptic precession match SOFA-derived matrices at the same scale, so
   the celestial math carries no dependency at all. The production tree is
   `hifitime` alone, taken later the same day for time scales and the
   maintained leap-second table.
2. **It assumed the transcribed Meeus Pluto table.** Turquet's inherited
   Pluto coefficients carry more digits: 1 millidegree against Horizons
   versus 14 for the commonly transcribed table.

Status of the analysis' seven steps at the time of writing:

| Step | State |
| --- | --- |
| 1. Fork astro-rust as the maintained repository | Done: Turquet is that repository, one crate, history preserved |
| 2. Typed calculation core | Partial: time scales come from hifitime (`jde_tt_frm_epoch`, `Epoch` re-exported); typed angles, distances, frames, and observers remain open |
| 3. Analytical body provider | Done at chart precision for ten bodies: light-time, solar deflection, aberration, IAU 2006/2000A orientation, Pluto range, typed equatorial output, and provider-neutral station roots |
| 4. Event algorithms over a position provider | Open: the architecture below fixes the shape; `verify_cohort` is the first consumer of the two-lane split |
| 5. ANISE out of the shipped graph | **Done**: the verifier lane lives here behind the opt-in `verify` feature; Cleromancy dropped its `ephemeris` feature and the anise, sha2, sofars, and ureq dependencies |
| 6. Broad evidence | Date cohort **done** (112,137 samples, 1885-2099, worst 5 millidegrees; `receipts/2026-08-13_t3_cohort_de440s.md`); observer cohort and targeted cases open |
| 7. Replace the consumer prototype | Done: Cleromancy's analytic feature is a rev-pinned Turquet adapter |

## The two-provider architecture

```text
Position provider (trait, T2 typed)
    +-- Analytical Turquet provider      (production; no data file)
    +-- ANISE + DE440s verifier provider (oracle; opt-in tooling only)

Shared algorithms (T4, generic over the provider)
    phases, eclipses, rise/set, conjunctions,
    aspects, stations, transits, visibility, houses
```

Rules:

1. The analytical provider is production. Its outputs carry model identity,
   supported range, and accuracy class.
2. The verifier provider never enters a consumer's dependency graph. It lives
   in Turquet as an opt-in verification feature or separate binary whose job
   is generating committed golden-vector files across the T3 cohort.
3. CI compares the analytical provider against **committed** vectors, so CI
   downloads no kernel. Regenerating vectors is a maintainer act with the
   kernel present, and the vector files record the kernel digest and ANISE
   revision that produced them. **Landed 2026-08-13**:
   `tests/vectors/cohort_de440s.tsv` holds 5,277 oracle values and
   `tests/cohort_vectors.rs` checks them without the `verify` feature.
4. Event algorithms are written once, generic over the provider, which makes
   every event receipt reproducible through either lane.

## What remains external, by doctrine

Completely static astronomy is not achievable, and pretending otherwise is
the silent-degradation failure mode. The external residue, handled as
versioned embedded snapshots with optional updates:

- **Leap seconds** change occasionally. The table now comes from `hifitime`
  rather than a hand-rolled constant, which delegates its maintenance to that
  crate's release cadence rather than eliminating the staleness; the engine
  revision in a disclosure is what pins which table was used.
- **Precise Earth rotation** (UT1, polar motion) is observational. It gates
  topocentric precision work, not geocentric chart work.
- **Artificial satellites** need current elements; **comets and asteroids**
  need element updates. Out of scope until a consumer pulls.

Every result should disclose the data snapshot and approximation used; the
snapshot identity joins the typed metadata at T2.

## Done-conditions from the analysis, tracked

- Default charts require neither network, filesystem data, ANISE, nor native
  code: **met**.
- The production dependency tree contains Rust crates only: **met**. It is
  `hifitime` alone; the celestial math itself needs nothing. Note that
  hifitime is MPL-2.0, so the tree is no longer MIT-only, though file-level
  copyleft leaves Turquet's own source unaffected.
- Sun through Pluto pass the defined JPL tolerance: **met across the date
  cohort**. 112,137 body-samples over 1885 to 2099 at 7-day steps hold within
  5 millidegrees, with the Moon the only body above 1. Targeted stations,
  lunar extremes, eclipse instants, and the high-latitude and topocentric
  cases remain open, since those need the observer layer.
- IAU transformations pass SOFA vectors: **open**, T2.
- ANISE exists solely as verifier and optional precision provider: **met**.
  The relocation landed the same day: `src/verify.rs` plus the
  `verify_cohort` binary here, and Cleromancy's kernel feature, download
  flow, and install UI removed.
- Every receipt records engine revision, models, supported range, coordinate
  frame, time scale, and data snapshot: **partial** (revision, models, and
  range are disclosed today; frame and time scale are documented rather than
  typed; snapshot identity is missing), completed by T2.

## Stop rule

This document fixes architecture, not implementation. The provider trait,
typed core, verifier relocation, and cohort evidence each arrive as their own
gate with receipts.

## Completion addendum: 2026-08-23

The status tables above record the founding state and are retained as history.
T2 and T3 are now complete. The primary API carries typed units, TT and UT1,
reference frames, model identity, accuracy evidence, and runtime-owned
Earth-orientation snapshot identity. SOFARS supplies the pure-Rust IAU
2006/2000A orientation implementation.

The analytical provider now includes explicit source-frame precession,
light-time, solar deflection, annual aberration, and nutation stages. Its
observer layer adds WGS84 site geometry, polar motion inputs, topocentric
true-equatorial output, and airless horizon output. The committed observer
cohort and targeted station, eclipse, lunar-distance, and high-latitude cases
close the remaining T3 evidence. See
`receipts/2026-08-23_t3_analytical_ephemeris.md` for the final measurements.

## T4a addendum: 2026-08-23

The provider trait assumed by the original diagram is now live as
`provider::GeocentricPositionProvider`. The analytical engine and opt-in
`JplVerifier` implement it, and `events::ecliptic_longitude_conjunctions` is
its first forcing consumer. Ordinary CI supplies the second implementation
through committed Horizons vectors, so the shared algorithm is executable
without a kernel. Live-kernel event execution and the remaining event
families are still T4 work.

## T4b addendum: 2026-08-23

Stationary points are now roots of provider-supplied apparent
ecliptic-longitude motion rather than classifications inside the analytical
engine. The velocity-difference span is explicit and retained in the result,
along with the direction on each side of the root. A committed hourly
Horizons provider and the analytical provider locate the 2024-04-25 Mercury
direct station with a 0.659-second difference.

## T4c addendum: 2026-08-23

Lunar phase events are now the four quarter-angle roots of provider-supplied
apparent Moon-minus-Sun ecliptic longitude. The result retains the phase,
bounded TT interval, great-circle center separation, and provider identity.
This keeps illumination and eclipse policy out of the phase name while making
the shared geometry executable through analytical and Horizons providers.

## T4d addendum: 2026-08-24

Eclipse candidates compose the phase search with provider-supplied directions
and distances. A solar candidate applies a conservative global parallax
allowance, so it does not pretend that a geocentric overlap is required or
that one observer's contacts have been solved. A lunar candidate compares the
Moon with a spherical, atmosphere-free Earth shadow and can therefore name
penumbral, partial, and total geocentric classes.

The result records the position-provider identity, geometry-model revision,
phase interval, and every angular term used by its predicate. NASA-listed
events and ordinary phase controls run through both analytical and committed
Horizons providers. Observer contacts, solar local type, visibility, Earth
oblateness, atmospheric shadow enlargement, and terrain remain later layers.

## T4e addendum: 2026-08-24

Lunar eclipse circumstances now refine the minimum geocentric
Moon-to-shadow-axis offset and solve every applicable spherical-shadow
tangency on either side of that minimum. The caller owns the full search span;
Turquet rejects spans whose endpoints remain inside the penumbra instead of
silently returning an incomplete contact set.

Greatest eclipse, P1/P4, U1/U4, and U2/U3 are bounded TT intervals carrying
the provider model and runtime snapshot. The same solver runs over analytical
states and committed Horizons facts. Contacts remain atmosphere-free and
geocentric. Observer-relative solar contacts, local eclipse type, visibility,
atmospheric enlargement, oblateness, and terrain remain separate contracts.

## T4 live-verifier addendum: 2026-08-24

`verify_events` now forces every landed provider-neutral event family through
a live `JplVerifier` backed by a caller-supplied SPK kernel. The command treats
missing results, event-class or contact-order disagreement, result intervals
wider than the selected tolerance, and provider differences outside the
measured gate as failures. It prints the ANISE-backed model revision and the
kernel SHA-256 retained by the provider.

The official DE440s cohort compared 26 paired results across conjunctions, a
station, all four lunar phases, positive and negative eclipse candidates, and
penumbral, partial, and total lunar circumstances. Worst measured differences
were 12.891 seconds for roots, 0.659 seconds for the station, 8.537 seconds for
greatest eclipse, and 22.455 seconds for contacts. This closes live verifier
execution for the event families already present. It does not supply the
epoch-indexed Earth orientation and provider-neutral topocentric transform
required by observer-relative contacts or altitude crossings.

## T4f addendum: 2026-08-24

Observer-relative event solving now adds a second, orthogonal provider seam:
`EarthOrientationProvider` supplies typed UT1, polar motion, authority, and a
data snapshot for every requested TT epoch. It does not select or wrap a
position provider. `airless_altitude_crossings` composes both sources directly
with `ObserverTransform`, the provider-neutral WGS84 projection extracted
from `ObserverSky`.

This keeps ephemeris and Earth-orientation failures distinct and prevents a
search from freezing one absolute UT1 value across its window. Results retain
both data identities and the airless transform revision. A one-hour sampling
ceiling bounds the first contract, while an empty result remains only an
absence of sampled sign changes. Named rise/set policy, transit, grazing and
persistent-state classification, refraction, horizon dip, terrain, and limb
selection remain later layers.

## T4g addendum: 2026-08-24

Airless altitude extrema are now sampled roots of a caller-selected central
difference. The full derivative span is validated, retained in every result,
and may cause provider requests up to half that span beyond the selected
search window. Roots are bracketed by altitude-motion reversal and refined by
bisection; midpoint altitude is an estimate, not an angular error bound.

`airless_altitude_circumstances` shares its altitude samples between crossing
and threshold-state work and reuses its refined extrema. Its state vocabulary
is deliberately evidence-scoped: crossing, grazing candidate, above or below
at all evaluated samples, or unresolved. Finite calls through a black-box
position provider do not establish a persistent or circumpolar state.
Meridian transit remains separate because it is an hour-angle event, not an
altitude-extremum synonym. Refraction, horizon policy, and named rise/set also
remain consumer-facing layers over these airless facts.

## T4h addendum: 2026-08-24

`airless_rise_set_events` now gives a deliberately narrow name to an existing
airless crossing: ascending means `Rise`, descending means `Set`, and the
caller-selected center-altitude threshold remains in the nested fact. The
revisioned naming model does not smuggle in a solar/lunar limb, refraction,
horizon dip, terrain, obstruction, civil-day, or visibility policy. Those
remain consumer-owned choices over the engine's physical airless facts.

`meridian_transits` is a separate event family. It samples the non-wrapping
scalar `cos(topocentric declination) * sin(local apparent hour angle)`, then
classifies a root with `cos(topocentric declination) * cos(local apparent hour
angle)` as upper or lower. The apparent hour angle combines equinox-based GAST,
SOFA's TIO/polar-motion-adjusted local meridian, and Turquet's topocentric
true-equator/equinox right ascension. This keeps a lower transit valid below
the horizon and prevents a moving Moon's altitude maximum from becoming an
accidental definition of transit.

Exact samples preserve the crossing family's explicit one-sided boundary rule;
flat sampled zero runs do not manufacture an event. An exact celestial pole is
also omitted because its right ascension is undefined. Each accepted result
retains its bounded TT interval, observer, provider model/snapshot,
topocentric transform model, and Earth-orientation authority/snapshot. Direct
Horizons quantity-42 local hour angles supply the independent reference path.

## T4i addendum: 2026-08-25

`conventional_rise_set_events` is a separate provider-neutral policy event. It
searches the signed scalar `airless center altitude + refraction + limb +
horizon dip` with T4f's one-hour sampling ceiling and its sign-change,
boundary-zero, and flat-plateau semantics. An event retains the complete
validated refraction, limb, and horizon-dip policy, alongside the midpoint
terms and usual position, transform, and Earth-orientation identities.

The policy chooses a fixed target refraction, a center, fixed-angular, or
physical-radius upper limb, and a level, constant, or spherical horizon dip.
Physical-radius limbs are evaluated from topocentric range. This is why the
Moon path does not add USNO's geocentric horizontal-parallax term a second
time. The USNO helpers are explicit 34-arcminute fixed refraction and a
16-arcminute fixed solar upper limb, not an implicit product default.

Altitude-dependent meteorological refraction, terrain, obstruction, civil-day,
and visibility are still separate policies. The search does not turn an empty
result into a persistent-state claim.

## T5a consumer-seam addendum: 2026-08-26

The first Sky-home daily-timeline consumer is owned across the repository
boundary by `turnstone/design_docs/2026-08-26_sky_home_timeline_plan.md`.
Turquet does not duplicate that product plan. It records the two engine seams
the consumer forced.

`JulianDate<TerrestrialTime>::to_epoch` is the inverse of the typed hifitime
input boundary. Event records remain TT facts, while a consumer can recover a
scale-aware epoch for civil presentation. The physical instant round-trips
across the 2016 leap-second boundary; the spelling of an inserted UTC `:60`
label remains hifitime's representation concern.

`GeocentricPositionProvider::accuracy` optionally discloses one homogeneous
provider-wide bound. Its default is `None`, meaning undisclosed rather than
exact. `AnalyticalEphemeris` returns the measured 10-millidegree angular bound
for its 5,277-vector DE440s cohort. The verifier retains `None` because kernel
identity alone does not establish one uniform provider-wide accuracy claim.

## T4j addendum: 2026-08-26

`local_solar_eclipse_circumstances` is a separate observer-relative family,
not a reclassification of geocentric `EclipseCandidate`. It first finds
provider-owned new-moon phase intervals, then at every evaluation acquires the
Sun and Moon states plus one epoch-indexed Earth orientation and applies the
same WGS84 airless `ObserverTransform` to both bodies. This prevents a second
parallax correction and keeps UT1, polar motion, and transform provenance in
the result.

The revision-1 geometry uses a fixed IAU nominal solar radius and Turquet's
mean lunar radius. It solves strict disk overlap, bounded greatest and C1--C4
contacts, and partial, annular, or total local class. Its visibility fact is
only the physical solar upper limb relative to the airless horizon at greatest
eclipse. Lunar limb relief, refraction, terrain, obstruction, weather, eye
safety, civil naming, and a general visibility-window policy remain outside
this contract and therefore remain available for later consumer-forced slices.

## T4k-a addendum: 2026-08-26

`lunar_illumination_at` is a provider-neutral fact, not a new search family.
It requests Sun and Moon apparent geocentric states at one typed TT epoch,
rejects any returned state tagged with another epoch, and forms the
Sun-Moon-Earth triangle in their shared frame. The retained fraction,
elongation, phase angle, and three distances make its geometric basis replayable
without turning it into a topocentric, limb, atmosphere, or human-visibility
claim.

Distance extrema remain a separate T4k-b contract because they introduce
sampling controls, central-difference semantics, and bounded event intervals.
Twilight and general visibility retain their own later policy boundaries.

## T4k-b addendum: 2026-08-26

`geocentric_distance_extrema` is a provider-neutral event search over one
body's apparent Earth-body range. `SearchWindow` owns typed TT start/end,
sampling step, and interval tolerance; `GeocentricDistanceExtremumSearch` adds
only the caller-selected, finite, positive central-difference span, capped at
one TT day. The provider is intentionally queried half that span outside both
window endpoints.

At each sample it evaluates `r(t + h/2) - r(t - h/2)`. A negative-to-positive
reversal yields a minimum, and positive-to-negative a maximum; each strict
bracket is bisected into an `EventInterval`. An isolated exact zero requires
opposite adjacent signs. Boundary zeros and flat zero plateaus are omitted so a
one-sided or undefined derivative does not manufacture an event. The retained
midpoint distance is evaluated at the interval midpoint, not a continuous bound
or a proof of a global extremum. Likewise an empty result means only no sampled,
bracketed reversal.

This path accepts no observer, Earth orientation, refraction, limb, terrain, or
visibility policy. It retains its own model revision plus provider model and
snapshot, and rejects a provider state whose typed TT epoch differs from the
request. Twilight and consumer-forced visibility remain separate policy slices.

## T4l addendum: 2026-08-26

`airless_solar_twilight_events` deliberately adds no solver. It accepts the
existing caller-owned `AltitudeCrossingSearch`, fixes the body to the Sun, and
names an ascending airless center crossing `Dawn` and a descending one `Dusk`.
The nested crossing retains the bounded TT interval, threshold, observer,
provider, WGS84 transform, and Earth-orientation provenance; the wrapper adds a
revisioned naming model only.

It selects no standard twilight band or default threshold, and adds no
refraction, limb, horizon dip, civil date, terrain, obstruction, weather,
luminance, or visibility convention. Empty retains the underlying sampled
crossing meaning. General visibility stays a separate T4m policy slice.
