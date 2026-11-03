# Source splits: H5 Turquet receipts

## 2026-11-03: six numerical and verification paths

Owner: house-style H5 / 01a19f86-cb71-7820-a6ac-aaec39d8bfd4.
Baseline source: `84d4089cf2a326ff59d3db59499953cee8215fab`. Compact text only, under H5's shared
100 MB cap. Frozen numerical receipts and coefficient tables are intact.

| Original source | Before | After |
|---|---:|---:|
| `src/foundation.rs` | 669 | 565 |
| `src/lunar.rs` | 1091 | 466 |
| `src/planet/mod.rs` | 662 | 489 |
| `tests/altitude_crossings.rs` | 1705 | 480 |
| `tests/events.rs` | 1111 | 428 |
| `tests/houses.rs` | 602 | 255 |

All children also fit 600 physical lines. Six exceptions retired, nine
remain: the event engine and eight VSOP coefficient tables. The recorded
coefficient-only approximate-constant exception is unchanged.

- Baseline/post `cargo test --locked`: 163 passed test/doctest cases each,
  zero failed or ignored. Existing independent Swiss/Horizons fixtures,
  tolerances, operation order, error boundaries and numerical values stay.
- `cargo check --all-targets --all-features --locked`: pass, including the
  optional verification binaries. This is compile evidence, not a new
  live-kernel or external ephemeris execution receipt.
- `cargo doc --all-features --locked --no-deps`: pass.
- `prek run --all-files`: ceiling and Clippy pass, including the actual
  commit gate. Existing inherited style warnings are not promoted to new
  lint exceptions. No dependency or lockfile changes.
- Structural review: all 65 old/new changed function bodies retain the same
  ordered tokens after source-scope, include-path and formatting separator
  normalization. Existing MIT notices accompany extracted calculations.
  Private implementation modules preserve the original public facade paths.
