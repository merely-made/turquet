# Turquet documentation

This is the canonical index for active documents in `design_docs/`.

## Working principles

- Turquet owns celestial facts and reproducible derivations. Product
  interpretation, control policy, secrets, social authority, and presentation
  remain with consumers.
- Primary calculations use typed time scales, frames, units, model revisions,
  and runtime data snapshots where applicable.
- Reusable event algorithms operate across both the analytical provider and an
  independent verification provider before their contract is treated as live.
- Event times are bounded intervals. Search tolerance and approximation scope
  stay visible to callers.
- The default engine remains pure Rust and kernel-free. External ephemerides
  are verification authorities or explicitly selected providers.
- Keep incomplete gates explicit in `ROADMAP.md` and support landed claims with
  executable tests and a dated receipt.

## Active documents

- [DOC_POLICY.md](DOC_POLICY.md): shared documentation rules and Turquet's
  local addendum.
- [2026-08-13_provider_architecture.md](2026-08-13_provider_architecture.md):
  provider boundary, verification doctrine, and landed T2 through live T4
  addenda.
The completed T4e, live-kernel verification, airless altitude-crossing, and
altitude-circumstance plans are archived under `archive_docs/2026-08-24/`.
Remaining event gates are canonical in `ROADMAP.md`.

## Maintainer-owned project description

`PROJECT_DESCRIPTION.md` is reserved for the maintainer and is not present.
Until it is supplied, the root `README.md` remains the available project
description. This is the known local exception recorded by `DOC_POLICY.md`.
