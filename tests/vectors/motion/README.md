# Retained apparent-longitude motion oracle

`swiss.tsv` contains 31 independently generated Swiss Ephemeris samples.
`generate.py` imports only Python standard-library modules and `swisseph`;
it does not import Turquet or obtain expected values from Turquet output.

The generation environment used pyswisseph **2.10.3.2**, Swiss Ephemeris
**2.10.03**, at:

```text
/Users/markik/Code/testing/cleromancy/natal-houses-20261017/oracle-env
```

Reproduce from the repository root:

```sh
/Users/markik/Code/testing/cleromancy/natal-houses-20261017/oracle-env/bin/python tests/vectors/motion/generate.py
cargo +1.97.1 test --locked --test motion -- --nocapture
```

Generation on 2026-10-17 produced TSV SHA-256:

```text
2791e0f07c994414bac64e586d279752a1e1e9ed4554f82d19dc0bd175bbab2c
```

The TSV also records the SHA-256 of the installed Python extension. The
expected values are ordinary retained text; Rust tests require neither Python
nor Swiss Ephemeris. Regeneration requires a compatible pyswisseph installation.

## Oracle and numerical contract

The generator calls `swe.calc(jd_tt, body, FLG_MOSEPH | FLG_SPEED)` and checks
that the returned flags equal the requested flags, **260**, at every sample.
The oracle is explicitly Swiss's Moshier analytical backend. No Swiss data
files or JPL kernel are used, and no silent backend fallback is accepted.
Swiss's installed `calc` documentation identifies its input as ET Julian day;
the fixture supplies TT Julian days directly, with calendar fields interpreted
as TT. There is no UTC, UT1, or Delta-T conversion in generation.

Positions use the default geocentric apparent tropical true ecliptic and
equinox of date. Flags for sidereal, topocentric, J2000, mean/nonutated, or
geometric true position are absent. Longitudes are degrees, rates are signed
degrees per TT day, and each TT day is 86,400 SI seconds.

Each row retains the before, center, and after longitudes; the signed shortest
delta between the samples at **±0.01 day**, divided by **0.02 day**; Swiss's
independently reported longitude speed at the center; classification; and
returned flags. Both the sampled Swiss rate and Swiss speed must yield the
same expected classification before a row is written. `NearStation` is the
inclusive band **abs(rate) <= 0.001 degree/day**. A near-station row does not
assert an exact stationary instant for Turquet or Swiss's sampled derivative.

## Coverage and limits

- Sun through Pluto at 2024-04-08 18:00 TT and 2026-08-13 12:00 TT cover all
  ten bodies, Mercury direct and retrograde, and outer-planet retrogrades.
- Three samples around each of Mercury's April 2024 retrograde and direct
  stations cover both directions and the near-station band.
- Three samples around Saturn's June 2024 station cover a slower body's
  direct, near-station, and retrograde motion.
- Actual Mercury crossings of zero longitude during 2025 exercise both
  direct 359-to-0 and retrograde 0-to-359 sample wrapping.

Station centers are selected by bisecting Swiss's reported speed; wrap centers
are selected by bisecting Swiss longitude. These roots only choose fixture
epochs. They are not independently qualified station-time error bounds.

The Rust comparison gate is **0.001 degree/day** against each of the sampled
Swiss rate and reported Swiss speed, together with exact class agreement for
every retained row. The initial measured worst residuals were
**0.000770980 degree/day** against the sampled rate and
**0.000686569 degree/day** against reported speed, both for the August 2026
Moon row. This bounded cohort is not a universal rate accuracy guarantee.
No position-error ceiling is repurposed as a derivative-error ceiling.

The focused test also compares existing `apparent::is_retrograde` with the
oracle outside the near-station band. The inherited one-day sign test is
unchanged; it need not match the shorter sampled-rate sign close to a station.
Additional synthetic providers test numerical policy, failures, and source
identity, and are distinct from the independent astronomical oracle.
