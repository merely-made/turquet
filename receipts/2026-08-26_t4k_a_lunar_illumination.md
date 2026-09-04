# T4k-a: geocentric lunar illumination

Date: 2026-08-26

## Scope

Turquet 0.15.0 adds `events::lunar_illumination_at`: one provider-neutral,
geocentric apparent lunar illumination fact at a selected TT epoch. It retains
the illuminated fraction, Sun-Moon elongation, phase angle at the Moon,
Earth-Moon, Earth-Sun, and Moon-Sun distances, the revisioned triangle model,
and provider model/snapshot.

The calculation fetches same-epoch Sun and Moon states, rejects a returned
state tagged with another TT epoch, constructs the Sun-Moon leg from Cartesian
same-frame vectors, then evaluates `fraction = (1 + cos(phase angle)) / 2`.
It excludes topocentric illumination, lunar limb relief or libration,
atmospheric transmission, terrain, obstruction, weather, eye safety, and every
visibility policy.

## Independent fixture

`tests/vectors/lunar_illumination_horizons.tsv` contains 30 paired Moon/Sun
rows from NASA/JPL Horizons API 1.2, DE441, geocenter `500@399`, captured on
2026-08-26 with EOP file `eop.260825.p261121`. It samples five instants from
minus 12 through plus 12 hours around NASA GSFC's 2024-04-08 new, 2024-04-15
first-quarter, and 2024-04-23 full Moon catalogue times. The Moon query records
Horizons illuminated percentage, phase angle, solar elongation, apparent range,
and ecliptic state; the Sun query records its apparent range and ecliptic state.
The regeneration command is:

```powershell
pwsh -NoProfile -File scripts/fetch_horizons_lunar_illumination_vectors.ps1 `
  > tests/vectors/lunar_illumination_horizons.tsv
```

Horizons's raw ecliptic fixture is IAU76/80 ecliptic-of-date. The fixture test
therefore establishes numerical agreement with Horizons fields, while the
typed Turquet analytical path retains its separately disclosed IAU 2006/2000A
orientation model. It does not claim that the two frame labels are identical.

## Measurements and gates

The fixture triangle is gated within 0.000010 illuminated fraction and 0.001
degrees of Horizons's independently reported illumination/phase/elongation
fields. The analytical result is gated within 0.000015 illuminated fraction of
the same external illumination field for all 15 lunar samples. New, first
quarter, and full-Moon analytical fractions are 0.00000936, 0.50131516, and
0.99978435 respectively at the NASA catalogue minutes.

## Verification

```text
cargo test --test lunar_illumination -j 1 -- --nocapture
```

Passed: 4 tests. They cover the three phase classes, independent fixture and
analytical comparison, provenance, both provider position errors, returned-state
epoch mismatch, zero range, coincident Sun/Moon, and an overflow-safe invalid
triangle.

## Deferred work

T4k-b will add provider-neutral distance facts and sampled extrema. T4l remains
caller-threshold twilight, and T4m remains consumer-forced general visibility.
