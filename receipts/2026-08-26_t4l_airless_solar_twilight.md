# T4l airless solar twilight receipt

## Scope

Turquet 0.17.0 adds `airless_solar_twilight_events`: a thin naming projection
over the existing airless Sun-center `AltitudeCrossingSearch`. An ascending
crossing is `Dawn`; a descending crossing is `Dusk`. The caller continues to
choose the threshold, search window, sampling step, and TT interval tolerance.
Every event retains the nested crossing's observer, threshold, bounded TT
interval, position-provider model and snapshot, WGS84 airless transform, and
Earth-orientation authority and snapshot, plus the revisioned naming model.

This is not a conventional twilight or visibility policy. It selects no band or
default threshold, and adds no refraction, limb, horizon dip, civil date,
terrain, obstruction, weather, luminance, or eye/optical visibility convention.
An empty result keeps the underlying sampled-crossing meaning only.

## Independent evidence

The focused lane reuses `tests/vectors/altitude_crossings_horizons.tsv`; no
new external capture was added. NASA/JPL Horizons API 1.2 / DE441 provides the
five-minute raw quantities: topocentric `APPARENT=AIRLESS` quantity 4 altitude,
geocentric quantities 20 and 31, and quantity 49 DUT1. The source script is
`scripts/fetch_horizons_altitude_crossing_vectors.ps1`; its EOP boundary is
`eop.260824.p261120`, with polar motion explicitly approximated as zero in the
fixture adapter.

The T4l adapter reads only Boston Sun and Tromso Sun fixture rows. It converts
raw UTC Julian Dates through `ScaleAwareEpoch::from_jde_utc` before retaining TT
epochs, and separately converts DUT1 to typed UT1 before constructing its
Earth-orientation values. The external reference is a piecewise-linear root of
direct quantity-4 altitude in that TT scale. The raw Horizons ecliptic columns
are an IAU76/80 numerical adapter for the fixture position seam; direct
quantity-4 altitude is the independent reference authority.

## Measurements

Boston's 2024-04-08 Sun runs caller-selected -6, -12, and -18 degree
thresholds through both fixture and analytical position lanes. The twelve named
event/reference residuals are independently gated at 0.5 seconds for each lane:

| Threshold | Dawn fixture | Dawn analytical | Dusk fixture | Dusk analytical |
| --- | ---: | ---: | ---: | ---: |
| -6 degrees | 0.148 s | 0.148 s | 0.420 s | 0.420 s |
| -12 degrees | 0.206 s | 0.206 s | 0.031 s | 0.031 s |
| -18 degrees | 0.214 s | 0.214 s | 0.068 s | 0.068 s |

The measured maximum is 0.420 seconds. Tromso's 2024-06-21 midsummer Sun has
no sampled crossings for the tested thresholds in either lane. This is an empty
control, not a polar-day, continuous-above-threshold, or visibility assertion.

The focused tests also prove wrapper parity with `airless_altitude_crossings`,
position/transform/EOP/naming provenance, existing threshold and step
validation, and typed propagation of a fixture position failure. Each returned
event interval is at most one second wide.

## Verification

- `rustfmt --check src/events.rs src/events/twilight.rs tests/twilight.rs tests/support/horizons_altitude_fixture.rs`
- `cargo test --test twilight -j 1 -- --nocapture` with isolated T4l Cargo home and target directory: 6 passed
- `cargo test -j 1` with the same isolated directories: passed
- `cargo package --allow-dirty --no-verify` with the same isolated directories: passed (154 files, 3.9 MiB)

The new production module, focused integration test, and fixture support module
are each below the 600-line guidance limit.

## Deferred

T4m remains a separate, consumer-forced sampled airless-above-threshold window
capability. It must not convert a geometric threshold result into human or
optical visibility. T5b remains Cleromancy's optional daily-facts adapter and
durable interpretation provenance, not an altitude-window dependency.
