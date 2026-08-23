# Provenance

Turquet is a history-preserving fork and adoption of
[`saurvs/astro-rust`](https://github.com/saurvs/astro-rust).

- Upstream founding revision: `c62ffdc7d55adfa1ee835fc7006d42d967bc4836`
- Turquet fork date: 2026-08-13
- Original author: Saurav Sachidanand
- Inherited license: MIT

The original commit history is retained. New work should preserve attribution
for inherited source and record the source, version, supported range, and
validation authority for added algorithms and coefficient sets.

The inherited implementation names these principal references:

- Jean Meeus, *Astronomical Algorithms*, second edition;
- Bretagnon and Francou's VSOP87D planetary theory;
- selected terms from Chapront's ELP-2000/82 lunar theory;
- Espenak and Meeus delta-T approximations;
- World Geodetic System 1984 constants.

These references describe provenance. They do not establish that every current
function is complete, correctly framed, or accurate over an unstated interval.
Each calculation must earn that claim through explicit documentation and
independent comparison.

## IAU orientation model

Turquet 0.2.0 uses `sofars` 0.6.1 for the numerical IAU 2006 precession and
IAU 2000A nutation series. `sofars` is a pure-Rust implementation derived from
the IAU Standards of Fundamental Astronomy collection. Its crate metadata is
MIT, and its distribution reproduces the additional SOFA terms governing the
derived routines.

Turquet does not copy or rename the SOFA routines. `src/orientation.rs` wraps
the dependency in Turquet-owned scale- and frame-typed contracts and reports
the backend revision. Turquet is not SOFA software and is not endorsed by the
IAU SOFA Board. Validation fixtures in `tests/orientation.rs` come from the
SOFA issue 2023-10-11 validation suite.

The applicable SOFA terms are published at
<https://www.iausofa.org/terms-and-conditions>. Downstream source and product
distributions should retain the `sofars` notices and use the acknowledgement
requested there when applicable.

## Time scales

Typed UTC-to-TT conversion delegates to `hifitime` 4.3. Its source is licensed
MPL-2.0 and remains a separate dependency; Turquet's wrapper source is MIT.
