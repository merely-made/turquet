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

Turquet 0.17.0 uses `sofars` 0.6.1 for the numerical IAU 2006 precession and
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

## Observer verification

The committed observer vectors in `tests/vectors/observer_horizons.tsv` were
generated from the NASA/JPL Horizons API on 2026-08-23 using DE441, airless
observer quantities 2, 4, 20, and 49, and EOP snapshot
`eop.260821.p261117`. Horizons applies polar motion from that EOP snapshot;
the checked Turquet fixture explicitly sets polar motion to zero and records
that approximation. The regeneration script is
`scripts/fetch_horizons_observer_vectors.ps1`.

## Event verification

The T4i conventional rise/set receipt uses the U.S. Naval Observatory's
definitions and one-day API v4.0.1. USNO defines its sea-level Sun convention
as a 50-arcminute geometric center depression: 34 arcminutes fixed refraction
plus a 16-arcminute average upper limb. Its geocentric Moon formula includes
horizontal parallax; Turquet evaluates the Moon topocentrically, so it applies
only the selected refraction and dynamic topocentric semidiameter.
Definitions: <https://aa.usno.navy.mil/faq/RST_defs>. API documentation:
<https://aa.usno.navy.mil/data/api>. The dated T4i receipt records both exact
one-day request URLs, their coordinate/time-zone parameters, the API revision,
and the four extracted minute values.

## T4j local solar-eclipse circumstances

`tests/vectors/local_solar_eclipse_horizons.tsv` contains 2,950 five-minute
DE441 rows captured from NASA/JPL Horizons API 1.2 on 2026-08-26. It supplies
geocentric apparent ecliptic longitude, latitude, and range through quantities
20 and 31, plus site-specific airless azimuth/elevation, local apparent hour
angle, and DUT1 through quantities 4, 42, and 49. The five fixed WGS84 sites
are Boston partial, Dallas total, Albuquerque annular, Galway low-altitude
partial, and Cape Town outside the local footprint. The fixture's
`eop.260825.p261121` header is retained; tests interpolate DUT1 and explicitly
record their zero-polar-motion approximation while direct Horizons altitudes
retain the source path. Regenerate it with
`scripts/fetch_horizons_local_solar_eclipse_vectors.ps1`.

The independent event references are the U.S. Naval Observatory Solar Eclipse
Computer and NASA GSFC local Besselian circumstance tables. The USNO endpoint
documents UT1 output and its limb convention, while T4j instead discloses its
fixed IAU nominal solar and mean lunar spherical radii. The test keeps that
model distinction visible rather than presenting a limb-profile, refraction,
terrain, weather, or human-visibility comparison as an engine result. Reference
requests: <https://aa.usno.navy.mil/api/eclipses/solar/date?date=2024-4-8&coords=42.3601,-71.0589&height=43>,
<https://aa.usno.navy.mil/api/eclipses/solar/date?date=2024-4-8&coords=32.7767,-96.7970&height=0>,
and <https://aa.usno.navy.mil/api/eclipses/solar/date?date=2023-10-14&coords=35.0844,-106.6504&height=0>.
NASA's comparable maps are
<https://eclipse.gsfc.nasa.gov/SEgoogle/SEgoogle2001/SE2024Apr08Tgoogle.html>
and <https://eclipse.gsfc.nasa.gov/SEgoogle/SEgoogle2001/SE2023Oct14Agoogle.html>.
Horizons quantity definitions are in
<https://ssd.jpl.nasa.gov/horizons/manual.html>.

## T4k-a lunar illumination

`tests/vectors/lunar_illumination_horizons.tsv` contains 30 paired Moon and
Sun rows from NASA/JPL Horizons API 1.2, DE441, geocenter `500@399`, and EOP
file `eop.260825.p261121`, captured on 2026-08-26. It covers five samples from
12 hours before through 12 hours after NASA GSFC's April 2024 new, first-quarter,
and full-Moon catalogue instants. The Moon query uses quantities 2, 10, 20, 23,
24, 29, 31, and 32; the Sun query uses 2, 20, and 31. The fixture preserves the
reported illuminated percentage, phase angle, solar elongation, apparent range,
and apparent observer-centered ecliptic longitude/latitude. Regenerate it with
`scripts/fetch_horizons_lunar_illumination_vectors.ps1`.

Horizons labels these raw ecliptic state columns IAU76/80 ecliptic-of-date,
whereas Turquet's typed analytical provider uses its disclosed IAU 2006/2000A
path. The fixture is therefore a numerical reference adapter, not a claim that
the frames are textual aliases. The test checks the fixture triangle against
Horizons's independently reported illumination, phase, and elongation, then
checks the analytical result against the same illumination field. It gates the
fixture at 0.000010 fraction and the analytical result at 0.000015 fraction;
it does not establish a global illumination accuracy bound. The result is a
geocentric apparent triangle fact, not topocentric illumination, lunar limb
relief, atmospheric transmission, or a visibility convention. Quantity
definitions are in <https://ssd.jpl.nasa.gov/horizons/manual.html>.

## T4k-b geocentric distance extrema

`tests/vectors/distance_extrema_horizons.tsv` contains 77 six-hour rows from
NASA/JPL Horizons API 1.2, DE441, geocenter `500@399`, and EOP file
`eop.260825.p261121`, captured on 2026-08-26. It covers Moon perigee
(2024-04-07 through 09), Moon apogee (2024-04-19 through 21), and Mars close
approach (2022-11-25 through 12-08). Each row preserves UTC Julian Date,
apparent range in AU, range rate in km/s, ecliptic longitude/latitude, and DUT1. Regenerate
it with `scripts/fetch_horizons_distance_extrema_vectors.ps1`.

The fixture provider linearly interpolates the captured range only to exercise
the public provider seam. Its independent reference is the vertex of a
three-point parabola through the raw surrounding Horizons ranges, not a claim
that the linear adapter is a continuous ephemeris. The focused test gates that
adapter within two hours and 200 km, and Turquet's analytical provider within
600 seconds and 50 km, across the three named cases. The measured maxima are
4,735.327 seconds and 159.186 km for the fixture adapter, and 368.322 seconds
and 27.169 km for the analytical provider. Those are this small cohort's
receipts, not global distance-extremum accuracy bounds.

Horizons labels the retained raw ecliptic columns IAU76/80 ecliptic-of-date;
Turquet's analytical provider retains its separately disclosed IAU 2006/2000A
path. Distance itself is frame-independent here, and the ecliptic columns only
construct the typed fixture state. This evidence does not establish an orbital,
barycentric, topocentric, or visibility-distance contract. Horizons quantity
definitions are in <https://ssd.jpl.nasa.gov/horizons/manual.html>.

`tests/vectors/altitude_crossings_horizons.tsv` contains five-minute apparent
geocentric positions, DUT1, direct topocentric airless elevations, and direct
local apparent hour angles from NASA/JPL Horizons API 1.2 and DE441. It covers
ordinary Boston Sun and Sydney Moon crossing pairs plus a Tromso midsummer Sun
empty control. Quantity 4 defines topocentric azimuth/elevation, quantity 42
defines signed local apparent hour angle in decimal angular hours, quantity 49
supplies DUT1, and
`APPARENT=AIRLESS` omits atmospheric refraction. Horizons applies polar motion
from EOP snapshot `eop.260824.p261120`; the Turquet fixture records its
zero-polar-motion approximation while interpolating DUT1 at every TT request.
Regenerate the 867 rows with
`scripts/fetch_horizons_altitude_crossing_vectors.ps1`. The official quantity
definitions are in the Horizons manual:
<https://ssd.jpl.nasa.gov/horizons/manual.html>.

The altitude-extremum evidence reuses these direct five-minute airless
elevations. For every sampled minimum or maximum, the independent reference
time and altitude are the vertex of the parabola through the direct Horizons
row and its immediate neighbors. This derives a sub-sample reference from the
external altitude facts; it does not pass Turquet positions, transforms, or
the central-difference solver through the reference path. No additional
source fixture is introduced for the T4g evidence.

The T4h transit reference derives `sin(hour angle)` roots and `cos(hour
angle)` upper/lower classification directly from the quantity-42 samples. It
interpolates the shortest signed angular change across the signed-hour-angle
wrap, rather than reconstructing right ascension or passing a Turquet
transform through the reference path. Turquet's local meridian itself is
formed from IAU 2006/2000A GAST and `sofars`/SOFA `apio`'s
TIO-and-polar-motion-adjusted longitude. The canonical IAU SOFA 2023
`iauApio` validation vector is a unit control for that seam.

## T4l airless solar twilight

T4l reuses the existing altitude-crossing fixture and capture script without a
new external capture. Its narrow test adapter admits only the five-minute
Boston Sun and Tromso Sun rows. It converts every captured UTC Julian Date with
`ScaleAwareEpoch::from_jde_utc` before storing a typed TT epoch, and separately
converts Horizons DUT1 into the typed UT1 epoch used by the Earth-orientation
provider. Direct quantity-4 airless-altitude roots are linearly interpolated
only after that UTC-to-TT conversion.

Boston's -6, -12, and -18 degree threshold pairs run through both the fixture
and analytical position lanes. Each of the twelve named results is within the
separate 0.5-second gate from its direct quantity-4 reference; the measured
maximum is 0.420 seconds. Tromso midsummer is the high-latitude empty control.
The raw Horizons ecliptic state columns remain an IAU76/80 numerical adapter,
not an assertion that they are textual aliases of Turquet's analytical IAU
2006/2000A path; the direct quantity-4 altitude is the independent authority.
The fixture continues to disclose its zero-polar-motion approximation and EOP
snapshot boundary.

This evidence names only caller-chosen airless Sun-center crossings. It does
not validate a conventional twilight band, refraction, limb, horizon dip, civil
date, terrain, weather, luminance, or human-visibility convention.

`tests/vectors/eclipse_conjunction_horizons.tsv` contains geocentric apparent
Sun and Moon positions generated from the NASA/JPL Horizons API on 2026-08-23
using DE441 and observer-table quantities 20 and 31. The reference event is
NASA GSFC's published 2024-04-08 ecliptic conjunction at 18:20:46.8 UT:
<https://eclipse.gsfc.nasa.gov/SEhistory/SEplot/SE2024Apr08T.pdf>.

The fixture is external input to a test-only provider. It does not route
through Turquet's analytical series. Horizons quantity 31 is apparent
observer-centered IAU76/80 ecliptic-of-date longitude and latitude; its
definition is in the official manual:
<https://ssd.jpl.nasa.gov/horizons/manual.html>.

Regenerate the fixture with
`scripts/fetch_horizons_conjunction_vectors.ps1`; the script emits the API
version and complete header with the data rows.

`tests/vectors/mercury_station_horizons.tsv` contains hourly apparent
geocentric Mercury positions from the same API and DE441 model, spanning
2024-04-24 18:00 through 2024-04-26 06:00 UTC. It is regenerated by
`scripts/fetch_horizons_station_vectors.ps1`. The station receipt searches
these independent positions using the same caller-selected six-hour central
difference as the analytical provider; the resulting event time is derived
from Horizons facts rather than copied from a published station table.

`tests/vectors/lunar_phases_horizons.tsv` contains apparent Sun and Moon
positions ten minutes before, at, and after each April 2024 quarter-phase
minute published by Fred Espenak at NASA GSFC:
<https://eclipse.gsfc.nasa.gov/phase/phase2001gmt.html>. The catalogue states
that its data may be reproduced with acknowledgment; this paragraph and the
fixture header retain that acknowledgment. The positions themselves come from
Horizons DE441 and are regenerated by
`scripts/fetch_horizons_phase_vectors.ps1`.

`tests/vectors/eclipse_geometry_horizons.tsv` extends that independent
provider fixture to seven new- or full-moon roots. NASA GSFC identifies five
of them as eclipses spanning penumbral, partial, and total lunar classes plus
central and partial solar geometry; an ordinary new moon and full moon are
negative controls. The classifications and phase minutes come from the same
NASA phase catalogue and its lunar-eclipse decade table:
<https://eclipse.gsfc.nasa.gov/LEdecade/LEdecade2021.html?hidemenu=true>.
Regenerate the 42 apparent position facts with
`scripts/fetch_horizons_eclipse_vectors.ps1`.

The revision-1 spherical eclipse model uses the IAU 2015 nominal solar radius
of 695,700 km, the WGS84 equatorial Earth radius of 6,378.137 km, and a mean
lunar radius of 1,737.4 km. Their authorities are, respectively:

- IAU 2015 Resolution B3:
  <https://www.iau.org/static/resolutions/IAU2015_English.pdf>;
- the US National Geospatial-Intelligence Agency WGS84 definition:
  <https://earth-info.nga.mil/?action=wgs84&dir=wgs84>;
- NASA's lunar geodetic reference description:
  <https://ntrs.nasa.gov/api/citations/20240013031/downloads/20240013031.pdf?attachment=true>.

The model is explicitly atmosphere-free. It does not apply the enlarged
terrestrial shadow used for eclipse contact predictions. For example, NASA's
detailed 2025-03-14 eclipse plot reports enlarged penumbral and umbral radii:
<https://eclipse.gsfc.nasa.gov/LEplot/LEplot2001/LE2025Mar14T.pdf>. Turquet's
smaller geometric radii therefore produce deliberately different contact
times rather than claims of NASA-compatible shadow modeling.

`tests/vectors/lunar_eclipse_circumstances_horizons.tsv` contains 162 apparent
Sun and Moon position facts at fifteen-minute spacing across the complete
contacts of the 2024-03-25 penumbral, 2024-09-18 partial, and 2025-03-14 total
lunar eclipses. Regenerate them with
`scripts/fetch_horizons_lunar_eclipse_circumstances.ps1`. Published greatest
and contact times come from NASA's detailed plots for
[2024-03-25](https://eclipse.gsfc.nasa.gov/LEplot/LEplot2001/LE2024Mar25N.pdf),
[2024-09-18](https://eclipse.gsfc.nasa.gov/LEplot/LEplot2001/LE2024Sep18P.pdf),
and
[2025-03-14](https://eclipse.gsfc.nasa.gov/LEplot/LEplot2001/LE2025Mar14T.pdf).
NASA defines P1/P4 as external penumbral tangencies, U1/U4 as external umbral
tangencies, U2/U3 as internal umbral tangencies, and greatest eclipse as the
minimum center-to-axis distance. Its plots use the Danjon shadow enlargement.
Tests therefore gate tight analytical-versus-Horizons agreement while
retaining the measured systematic offset from NASA contact times.

When the opt-in `JplVerifier` reads a caller-supplied SPK kernel, it computes
the file's SHA-256 and carries that runtime snapshot into event results. The
digest records identity; it is not an allowlist or an accuracy claim.

The 2026-08-24 live event receipt used NASA/JPL NAIF's generic planetary
`de440s.bsp` with SHA-256
`c1c7feeab882263fc493a9d5a5b2ddd71b54826cdf65d8d17a76126b260a49f2`.
The kernel index is
<https://naif.jpl.nasa.gov/pub/naif/generic_kernels/spk/planets/>. It was read
through ANISE revision `71e973a245e6701e14a5d4c88a3c4e7dedbf7702` by
`verify_events`; it is caller-owned verification data and is not distributed,
downloaded, or required by Turquet's default graph or CI.
