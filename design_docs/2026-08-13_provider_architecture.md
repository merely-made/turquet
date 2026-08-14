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
| 3. Analytical body provider | Done at chart precision for ten bodies (light-time, aberration, nutation, Pluto range, retrograde); open: deflection, FK5/ICRS, IAU 2006/2000A, equatorial output, stations |
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
