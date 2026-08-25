# T4g receipt: altitude circumstances

**Date:** 2026-08-24
**Engine:** Turquet 0.10.0
**Scope:** seventh T4 event slice; T4 remains open

## Contract landed

`airless_altitude_extrema` composes any `GeocentricPositionProvider` with an
epoch-indexed `EarthOrientationProvider`, a WGS84 observer, and the revision-1
airless topocentric transform. It samples the sign of a caller-selected full
central difference of altitude, brackets motion reversals, and returns local
minima or maxima as TT intervals bounded by the caller's time tolerance.

Each extremum retains its kind, derivative span, midpoint-altitude estimate,
observer, position-provider model and snapshot, transform revision, and
Earth-orientation authority and snapshot. Provider requests may extend half
the derivative span beyond the search window. Exact sampled zero slope needs
two-sided motion reversal; a flat plateau and a boundary zero are not named as
extrema.

`airless_altitude_circumstances` shares altitude samples between crossing and
state work. It reports `Crosses`, a tolerance-based `GrazingCandidate`,
`AboveAtAllSamples`, `BelowAtAllSamples`, or `Unresolved`. These are facts
about evaluated samples and refined extrema, not proofs of continuous,
persistent, circumpolar, or visible state. Transit, refraction, horizon dip,
terrain, apparent limb, and civil rise/set naming remain separate contracts.

## Independent evidence

The committed 867-row NASA/JPL Horizons fixture supplies five-minute direct
topocentric airless elevations from API 1.2 and DE441. Each independent
extremum reference is the vertex of the parabola through a direct Horizons
elevation and its immediate neighbors. The reference path does not reuse
Turquet positions, transforms, or extrema solving.

| Case and extremum | Fixture provider time / altitude residual | Analytical provider time / altitude residual |
| --- | ---: | ---: |
| Boston Sun minimum | 0.054 s / 0.000003 deg | 0.054 s / 0.000015 deg |
| Boston Sun maximum | 0.142 s / 0.000002 deg | 0.142 s / 0.000015 deg |
| Sydney Moon maximum | 0.292 s / 0.000004 deg | 0.292 s / 0.000244 deg |
| Sydney Moon minimum | 0.425 s / 0.000003 deg | 0.454 s / 0.000135 deg |
| Tromso Sun maximum | 0.063 s / 0.000010 deg | 0.063 s / 0.000000 deg |
| Tromso Sun minimum | 0.152 s / 0.000010 deg | 0.152 s / 0.000000 deg |

All twelve comparisons stay inside the 1-second and 0.001-degree gates. Every
returned interval is at most one second wide. Tromso controls additionally
exercise sampled-above, sampled-below, and grazing-candidate states. A
near-threshold sample without a refined extremum remains unresolved.

Horizons applies polar motion from EOP snapshot `eop.260824.p261120`;
Turquet's test records its explicit zero-polar-motion approximation and
interpolates the supplied DUT1 at every TT request. Regenerate the shared
fixture with:

```powershell
pwsh -File scripts/fetch_horizons_altitude_crossing_vectors.ps1 > tests/vectors/altitude_crossings_horizons.tsv
```

## Remaining T4 work

- named rise/set and transit policy;
- observer-relative solar contacts, local eclipse type, and visibility;
- general visibility and illuminated-fraction or distance extrema.

## Publication gates

```powershell
cargo test --all-targets --all-features
cargo test --doc
$env:CARGO_TARGET_DIR = '<external-target>'
cargo package --allow-dirty
```

All three gates pass for Turquet 0.10.0. The changed Rust files also pass
direct `rustfmt --check`; the inherited crate-wide format baseline remains
unchanged. Existing warnings are confined to inherited lunar, Jupiter-moon,
time, and legacy-test source.
