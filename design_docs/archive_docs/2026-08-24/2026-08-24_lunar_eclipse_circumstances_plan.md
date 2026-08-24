# Lunar eclipse circumstances plan

**Status (2026-08-24): landed in Turquet 0.8.0**

## Target

Extend T4 eclipse candidates with provider-neutral greatest-event refinement
and bounded atmosphere-free lunar shadow contacts. Preserve the distinction
between geocentric lunar circumstances and future observer-relative solar
contacts or visibility.

## Phase 1: contract and numerical model

- Add validated search controls for the full circumstance span around a
  full-moon root.
- Refine the minimum Moon-to-shadow-axis offset to a bounded TT interval.
- Expose lunar eclipse class, greatest geometry, provider identity, geometry
  revision, and ordered contact kinds.

Done when invalid controls are typed errors, results retain every authority
boundary, and greatest event is an interval bounded by caller tolerance.

## Phase 2: contacts

- Solve penumbral ingress/egress for every lunar eclipse.
- Add umbral ingress/egress for partial and total events.
- Add totality begin/end for total events.
- Return an explicit error when the caller's circumstance span does not reach
  both penumbral exterior states.

Done when contact intervals are ordered, bounded by caller tolerance, and the
contact set follows the eclipse class.

## Phase 3: independent evidence and release

- Execute the same search over Turquet analytical states and committed
  NASA/JPL Horizons DE441 facts.
- Check penumbral, partial, and total events against NASA GSFC classifications
  and published contact times while keeping atmospheric enlargement
  differences visible.
- Update the audit, roadmap, provenance, architecture addendum, README, and a
  dated receipt; run full tests, doctests, package verification, and CI.

Done when both providers agree on class and ordered contacts across all three
lunar eclipse classes, documented tolerances reflect measured results, and
`main` is clean and synchronized after green CI.

## Findings

- **2026-08-24:** T4d classifies at the ecliptic-longitude phase midpoint. T4e
  must find the actual minimum shadow-axis offset before determining class or
  contacts.
- **2026-08-24:** Contact roots can be bracketed between the refined greatest
  epoch and caller-supplied exterior endpoints. This avoids a sampling step
  that could miss arbitrarily short grazing contacts.
- **2026-08-24:** The revision-1 shadow is atmosphere-free. NASA contact plots
  use an enlarged terrestrial shadow, so comparison gates must disclose the
  expected systematic timing difference.
- **2026-08-24:** Across three lunar eclipse classes, the analytical and
  Horizons providers differ by at most 8.708 seconds at greatest eclipse and
  22.595 seconds at contacts. NASA's Danjon-enlarged contact times differ by
  as much as 247.598 seconds, confirming that model scope must stay visible.

## Progress

- **2026-08-24:** Contract scoped; independent NASA plot sources identified.
- **2026-08-24:** Shared shadow geometry, greatest-event refinement, ordered
  contacts, typed span controls, and insufficient-span failure landed.
- **2026-08-24:** A 162-state DE441/Horizons fixture, three-class numerical
  test, provenance, public audit, roadmap, architecture addendum, and release
  receipt landed. Remaining T4 work stays tracked in `ROADMAP.md`.
