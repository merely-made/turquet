# T4i conventional rise/set receipt

`conventional_rise_set_events` solves airless center altitude plus selected
fixed refraction, upper-limb offset, and horizon dip. Policy values are
validated, retained, and provider-neutral. A physical-radius limb uses the
existing topocentric range, so the Moon receives no second horizontal-parallax
correction.

USNO API v4.0.1 minute references for 2024-04-08 UTC use sea-level, level
horizon policy. Boston Sun uses USNO's fixed 34 arcminutes refraction and
16-arcminute fixed upper limb: the Horizons fixture residuals are 4.600 seconds
at 10:14 rise and 24.229 seconds at 23:19 set. Sydney Moon uses 34 arcminutes
plus a dynamic 1,737.4 km topocentric limb; its fixture residuals are 1.025
seconds at 07:20 set and 27.334 seconds at 20:24 rise. The analytical lane uses
the fixture EOP provider and has a measured worst residual of 26.455 seconds.

## Reference requests

The USNO responses were captured on 2026-08-25 with UTC output and daylight
saving disabled:

```text
https://aa.usno.navy.mil/api/rstt/oneday?date=2024-04-08&coords=42.3601,-71.0589&tz=0&dst=false
https://aa.usno.navy.mil/api/rstt/oneday?date=2024-04-08&coords=-33.8688,151.2093&tz=0&dst=false
```

API v4.0.1 returned Boston Sun `Rise=10:14`, `Set=23:19`, and Sydney
Moon `Set=07:20`, `Rise=20:24`. The test keeps these minute values next to the
same request URLs. USNO documents coordinates as north/east-positive and `tz=0`
as UT1 output. The test passes that minute label through Turquet's typed
UTC-to-TT path before comparing the bounded event midpoint; the fixture DUT1
is about -0.016 seconds, explicitly negligible only relative to the API's
minute quantization.

`cargo test --offline --test altitude_crossings -j 1 -- --nocapture` passed all
13 tests in 749.75 seconds. Existing inherited-source warnings remain.

Altitude-dependent meteorological refraction, terrain, obstruction, civil-day,
and visibility are deliberately not modeled.
