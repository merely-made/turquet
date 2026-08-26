# T4j local solar-eclipse circumstances receipt

Turquet 0.14.0 adds `local_solar_eclipse_circumstances`. The provider-neutral
search starts with new-moon phase intervals, then evaluates the Sun and Moon
through one WGS84 airless observer transform per TT epoch. It returns strict
local disk-overlap events only: partial, annular, or total class; bounded
greatest and C1--C4 contact intervals; fixed-limb geometry; and the physical
solar-upper-limb airless-horizon state at greatest eclipse.

The fixed revision-1 model uses the IAU nominal solar radius and Turquet's mean
lunar radius. It does not claim lunar limb relief, atmospheric refraction,
terrain or obstruction clearance, weather, eye safety, civil naming, or a
general visibility window. A below-horizon geometric event remains a result
with an explicit below-airless-horizon state rather than being silently removed.

## Independent evidence

`tests/vectors/local_solar_eclipse_horizons.tsv` contains 2,950 rows from
NASA/JPL Horizons API 1.2, DE441, captured 2026-08-26. The five-minute fixture
covers Boston partial, Dallas total, Albuquerque annular, Galway's
below-horizon partial geometry, and Cape Town's no-overlap control. It retains
geocentric apparent ecliptic state terms, direct site-specific airless horizon
terms, DUT1, API revision, EOP snapshot `eop.260825.p261121`, request shape,
and regeneration command. The fixture applies Horizons' polar motion; the
test's deliberately named approximation interpolates DUT1 and uses zero polar
motion.

The direct Horizons Sun-center altitude check has a measured worst residual of
0.00648 degrees at each local event's refined greatest midpoint. Fixture and
analytical lanes agree on the expected local class, chronological contact set,
and greatest-horizon state. The fixture does not pass Turquet's analytical
series through its reference path.

Published local-circumstance checks exercise the three positive classes:

| Site and eclipse | Analytical greatest residual |
| --- | ---: |
| Boston, 2024-04-08 partial | 9.367 s |
| Dallas, 2024-04-08 total | 10.884 s |
| Albuquerque, 2023-10-14 annular | 11.630 s |

The published references are the USNO Solar Eclipse Computer and NASA GSFC
local circumstance material recorded in `PROVENANCE.md`. USNO labels its
values UT1; the stored DUT1 values are under 0.02 seconds, negligible for this
comparison's one-second bounded result intervals and the recorded residuals.

## Verification

All commands used isolated Cargo state to avoid unrelated workspace builds:

```text
CARGO_HOME=C:\t\turquet-t4j-cargo-home
CARGO_TARGET_DIR=C:\t\turquet-t4j-target
cargo test --test local_solar_eclipse -j 1
cargo test --lib -j 1
cargo test -j 1
```

The focused T4j suite passed 7 tests. The full `cargo test -j 1` suite passed,
including doctests. Existing inherited-source warnings remain and are not part
of this change. `rustfmt` ran only on the new T4j Rust modules and focused test
files because repository-wide formatting has inherited unrelated differences;
`git diff --check` passed. New production files are 488 and 317 lines, below
the 600-LOC guidance.
