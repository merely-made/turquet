# T4 receipt: live-kernel event execution

**Date:** 2026-08-24
**Engine:** Turquet 0.8.1
**Scope:** live execution gate for every landed T4 event family; T4 remains open

## Contract landed

The opt-in `verify_events` binary runs one fixed cohort through both
`AnalyticalEphemeris` and a live `JplVerifier`. It covers:

- the 2024-04-08 Sun-Moon conjunction;
- Mercury's 2024-04-25 direct station;
- all four April 2024 lunar quarter phases;
- five eclipse candidates and two ordinary-phase negative controls;
- the 2024-03-25 penumbral, 2024-09-18 partial, and 2025-03-14 total lunar
  eclipse circumstances, including all class-applicable contacts.

The command fails when either provider returns a partial or unexpected cohort,
when event classification or contact order differs, when a result interval is
wider than 1.001 seconds, when the live result loses its kernel snapshot, or
when a provider difference crosses its measured gate. The gates are 15 seconds
for event roots, 2 seconds for the station, 10 seconds for greatest eclipse,
and 25 seconds for contacts.

## Verification authority

The run used NASA/JPL NAIF's generic planetary `de440s.bsp`, obtained outside
the repository from:

<https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/>

- Kernel SHA-256:
  `c1c7feeab882263fc493a9d5a5b2ddd71b54826cdf65d8d17a76126b260a49f2`
- Provider: `JPL SPK verifier through ANISE`
- ANISE revision: `71e973a245e6701e14a5d4c88a3c4e7dedbf7702`
- Time scale: TT event intervals derived from caller-constructed UTC epochs
- Output frame: geocentric apparent true ecliptic equinox of date
- Runtime data ownership: caller-supplied kernel, absent from the crate and CI

On Windows, ANISE's first live translation exceeded the default thread stack.
Both maintainer binaries run their verifier work on a 32 MiB worker stack. The
library API and default consumer graph are unchanged.

## Measured result

```text
verified 26 paired event results; worst root 12.891s; station 0.659s; greatest 8.537s; contact 22.455s
```

Every interval was at most one second wide. Both providers agreed on lunar
phase and eclipse class, rejected both negative controls, and returned the same
class-applicable contact order.

| Family | Worst analytical to DE440s difference | Gate |
| --- | ---: | ---: |
| conjunction, phase, and candidate roots | 12.891 s | 15 s |
| Mercury station | 0.659 s | 2 s |
| lunar greatest eclipse | 8.537 s | 10 s |
| lunar contacts | 22.455 s | 25 s |

The existing broad-position verifier was also smoke-tested against the same
kernel at an 80,000-day step. It compared all ten bodies at its sampled epoch
and completed with a worst residual of 1 millidegree. This smoke proves the
larger worker stack for both live binaries; the broad seven-day cohort remains
the authoritative T3 position receipt.

## Reproduction

```powershell
cargo run --features verify --bin verify_events -- <path-to-de440s.bsp>
cargo run --features verify --bin verify_cohort -- <path-to-de440s.bsp> 80000
```

The kernel is deliberately caller-owned. The commands do not download it, and
ordinary builds and CI do not need it.

## Remaining T4 work

- observer-relative solar contacts, local eclipse type, and visibility;
- provider-neutral airless altitude crossings using epoch-indexed Earth
  orientation and observer geometry;
- transit and richer no-crossing or grazing classifications;
- general visibility and illuminated-fraction or distance extrema.

## Publication gates

```powershell
cargo test --all-targets --all-features
cargo test --doc
$env:CARGO_TARGET_DIR = '<external-target>'
cargo package --allow-dirty
```
