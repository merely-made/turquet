# Source splits: H5 Turquet batch

**Status (2026-11-03):** landed, 6/6 paths qualified.

## Phases and done-conditions

- **S1. Numerical responsibilities.** Separate typed model/provenance values,
  lunar libration and phase calculations, and planetary photometry from
  their current source roots. Preserve operation order and public paths.
- **S2. Verification fixtures.** Group altitude, event and house cases by
  behavior without altering reference data, assertions or tolerances.
- **S3. Qualification.** All six original files and child modules fit the
  600-line ceiling, retire those exceptions, pass numerical tests and
  appropriate feature/docs/local gates, commit and push.

Mark requested 50 additional oversize files under Mere's house-style H5.
No numerical policy ruling. Coefficient tables and frozen reference receipts
stay intact, including the recorded table-only Clippy exception.

## Findings

- 2026-11-03: the chosen six files contain distinct provenance, libration,
  phase, photometry and regression groups. The 4,304-line event engine and
  coefficient tables remain outside this batch; no claim of eliminating
  this repository's size debt follows.
- Compact evidence appends to [the receipt](archive_docs/receipts/2026-11-03_source_splits_plan_receipts.md).
  Shared H5 retained-byte cap: 100 MB across repositories.

## Progress

- 2026-11-03: selected six source/fixture paths in a clean checkout.

- 2026-11-03: S1–S3 landed. Six exceptions retired; 163 baseline/post test and doctest cases pass, with optional-feature compilation, docs and the full local gate. Numerical tokens and fixture thresholds preserved.
