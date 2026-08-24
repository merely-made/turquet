# Live-kernel event verification plan

**Status (2026-08-24): landed; archived after publication**

## Target

Close T4's live-verifier execution gate with one maintainer command that runs
every landed provider-neutral event family through a caller-supplied JPL SPK
kernel and compares it with Turquet's analytical provider. Keep kernel
acquisition and storage outside the crate and ordinary CI.

## Phase 1: executable cohort

- Add a `verify`-feature binary dedicated to event verification.
- Cover conjunctions, stationary points, lunar phases, eclipse candidates,
  and penumbral, partial, and total lunar circumstances.
- Print provider model, kernel SHA-256 snapshot, bounded event intervals, and
  measured provider differences.
- Return a failing process status for missing events, class/contact
  disagreement, interval-width failures, or exceeded documented tolerances.

Done when one command executes the complete landed T4 event stack through
`JplVerifier` and cannot report a partial success as a passing run.

## Phase 2: live evidence and publication

- Run the cohort against the official NASA/JPL NAIF DE440s kernel.
- Record the kernel digest, ANISE revision, event cohort, measured maxima, and
  exact command in a dated receipt.
- Update the roadmap, audit, provenance, provider architecture, README, and
  this plan without implying that the verifier entered the default graph.
- Run all-feature tests, doctests, package verification, and GitHub CI.

Done when the live run is reproducible from a caller-owned kernel, local and
hosted gates pass, `main` is synchronized, and only the live-kernel T4 gate is
closed.

## Findings

- **2026-08-24:** `JplVerifier` already implements
  `GeocentricPositionProvider`, retains the kernel SHA-256, and supplies the
  typed state required by every landed event search.
- **2026-08-24:** Observer-relative solar contacts and altitude crossings both
  require an epoch-indexed Earth-orientation source and a provider-neutral
  topocentric transform. That prerequisite remains a separate next slice.
- **2026-08-24:** NASA/JPL NAIF publishes `de440s.bsp` as a 31 MB generic
  planetary SPK. The verification run may download it outside the repository;
  the binary and crate must never acquire it automatically.
- **2026-08-24:** The first live DE440s translation overflowed Windows' default
  thread stack inside verifier tooling. Both live-kernel binaries now run their
  work on an explicitly sized worker thread; the library path is unchanged.
- **2026-08-24:** The complete live cohort compared 26 paired results. Measured
  worst differences were 12.891 seconds for roots, 0.659 seconds for the
  station, 8.537 seconds for greatest eclipse, and 22.455 seconds for contacts.
  Every event interval was at most one second wide, and class/contact order
  agreed across providers.

## Progress

- **2026-08-24:** Luna and Terra audits completed; live-kernel execution was
  selected as the smallest gate already supported by the landed architecture.
- **2026-08-24:** `verify_events` landed with measured failure gates of 15
  seconds for roots, 2 seconds for the station, 10 seconds for greatest
  eclipse, 25 seconds for contacts, and 1.001 seconds for result intervals.
- **2026-08-24:** The official DE440s run and a live `verify_cohort` smoke test
  passed. Provenance, audit, roadmap, architecture, README, and the dated
  receipt record the reproducible boundary. Observer altitude crossings remain
  in `ROADMAP.md` as the next separate engine prerequisite and event slice.
