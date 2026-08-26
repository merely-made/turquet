# Public calculation audit

This is the T1 boundary map with T2, T3, and T4 public-contract addenda for
Turquet 0.13.0. It inventories every exported calculation in the crate as of
2026-08-26. It is descriptive, not an accuracy certificate.

The status words have narrow meanings:

- **measured**: compared against an independent authority over a stated cohort;
- **corrected**: a reproduced defect has been repaired and covered by an
  independent vector;
- **example**: only inherited examples or algebraic regression tests exist;
- **unverified**: units may be documented, but frame, time scale, range, or
  independent numerical evidence is incomplete;
- **tooling**: maintainer verification code, outside the consumer API.

Unless a row says otherwise, inherited calculations come from the
MIT-licensed `astro-rust` implementation of Jean Meeus's *Astronomical
Algorithms*, second edition. Many source files do not record a chapter,
coefficient revision, validity interval, or expected error. Those omissions
remain omissions here rather than being filled with guesses.

All inherited symbols in the tables below are shorthand for paths under
`turquet::compat`. Their source modules are private in 0.13.0. The Turquet-era
`foundation`, `orientation`, `apparent`, `observer`, `provider`, and `events`
modules form the primary API.

## Turquet-era surface

| Public symbol | Source and contract | Range and evidence | Status |
| --- | --- | --- | --- |
| `foundation::{JulianDate, TimeOffset, Angle, Longitude, EastLongitude, Latitude, Length, Distance, Observer}` | Finite unit-safe values. `JulianDate<Scale>` carries TT or UT1 in the type and preserves two input parts; typed TT converts to and from a scale-aware hifitime epoch, while observed DUT1 constructs UT1 through a seconds-typed `TimeOffset`. Celestial longitude wraps to 0-2pi; geographic longitude stays signed and east-positive. | Constructor, range, normalization, ordinary UTC label, and physical-instant round-trip tests across the 2016 leap-second boundary in `tests/foundation.rs`. | measured |
| `foundation::{Direction, UnitVector, Rotation}` and frame markers | Reference frame is a type parameter; a `Rotation<From, To>` only accepts a `UnitVector<From>`. | Runtime SOFA vector plus compile-fail frame-mismatch doctest. | measured |
| `foundation::{Model, Accuracy, Modelled, State}` | A calculation carries its algorithm revision, evidence kind, bounded angular residual, epoch, typed frame, direction, and distance. | Metadata assertions in `tests/apparent.rs` and `tests/orientation.rs`. | measured |
| `orientation::nutation` | `JulianDate<TerrestrialTime>` to IAU 2000A nutation adjusted for IAU 2006 precession; radians on the mean equinox/ecliptic of date. | SOFARS 0.6.1; matches the SOFA 2023 `nut06a` vector to 1e-13 rad. | measured |
| `orientation::gcrs_to_true_equator` | TT epoch to `Rotation<Gcrs, TrueEquatorEquinoxOfDate>`, including frame bias, IAU 2006 precession, and IAU 2000A nutation. | Matches all nine elements of the SOFA 2023 `pnm06a` vector to 1e-12. | measured |
| `orientation::{true_obliquity, true_ecliptic_to_true_equator}` | Typed TT to IAU 2006/2000A true obliquity or a frame-safe true-ecliptic-to-true-equatorial rotation. | Composed from the measured SOFARS `obl06` and `nut06a` model; exercised through all 90 observer vectors. | measured |
| `apparent::ApparentBody::name`, `apparent::APPARENT_BODIES` | Body identifiers; no numerical contract. | Exhaustive for the ten supported bodies. | measured |
| `apparent::{ApparentStage, APPARENT_STAGES}` | Ordered disclosure of source-frame precession, light-time, solar deflection, annual aberration, and nutation. A stage may be the identity where the source series is already in the required frame or is the deflector. | Each stage is present in the composed analytical path; the full path supplies the numerical evidence. | measured |
| `apparent::ApparentSky::at`, `ApparentSky::position`, `apparent::position`, `apparent::analytical_accuracy` | `JulianDate<TerrestrialTime>` to `Modelled<State<TrueEclipticEquinoxOfDate>>`; direction is radians through typed accessors and distance is stored in metres with km/AU accessors. The public accuracy disclosure is explicitly angular and cohort-scoped. | Current path holds 5,277 committed DE440s vectors from 1885 through 2099 below a 10-millidegree gate, measured worst 3; Pluto rejects dates outside its series. Targeted eclipse and lunar distance-extreme vectors also pass. | measured |
| `apparent::is_retrograde` | Typed TT epoch; central difference of typed apparent longitude over one TT day. | Same body range, narrowed by half a day at both edges. This remains a convenient analytical classification; provider-neutral event solving is in `events`. | measured |
| `observer::{EarthOrientation, ObserverTransform, ObserverTransformError, ObserverSky, Observation, position}` | Typed TT ephemeris plus typed UT1 Earth rotation, caller-supplied polar motion and runtime snapshot identity, and WGS84 observer to topocentric true-equatorial state plus airless north-zero horizon direction. `ObserverTransform` applies the same observer geometry to a provider-supplied geocentric state and rejects epoch mismatch. Observer origin is encoded in distinct frame markers. | 90 DE441/Horizons vectors: all ten bodies, three epochs, Boston/Sydney/Tromso, measured worst 0.001522 degrees and 0.000108 AU. The factored transform is checked exactly against the analytical wrapper. Compile-fail proof rejects TT where UT1 is required. | measured |
| `provider::{GeocentricPositionProvider, AnalyticalEphemeris, ANALYTICAL_EPHEMERIS, EarthOrientationProvider, ConstantOffsetEarthOrientation}` | Provider-neutral TT to geocentric apparent true-ecliptic-of-date state plus a separate epoch-indexed source of UT1, polar motion, authority, and data snapshot. A position provider may disclose one homogeneous `Accuracy`; the default `None` means undisclosed rather than exact. The disclosed constant-offset source advances UT1 with TT while holding UT1-minus-TT and polar motion fixed. | Analytical and opt-in `JplVerifier` position implementations; the analytical provider exposes its measured 10-millidegree external angular cohort, while the verifier inherits the honest undisclosed default. Committed Horizons fixtures exercise both provider seams. Earth-orientation identity and source failures are typed and tested. `JplVerifier` computes and retains the supplied kernel's SHA-256. | measured |
| `events::{SearchWindow, EventInterval, EclipticLongitudeConjunction, ecliptic_longitude_conjunctions}` | Searches two distinct bodies for apparent ecliptic-longitude equality. Sampling is limited to one TT day; tolerance is caller-selected and results are bounded TT intervals with midpoint great-circle separation plus provider model/snapshot identity. | 2024 eclipse: analytical midpoint 8.571 seconds from NASA's published conjunction, Horizons-fixture midpoint 4.469 seconds from NASA, providers differ by 4.102 seconds. Wrap, opposition, invalid-control, same-body, and provider-failure tests. | measured |
| `events::{StationSearch, LongitudeMotion, EclipticLongitudeStation, ecliptic_longitude_stations}` | Searches sign changes in apparent ecliptic-longitude speed. Sampling, time tolerance, and the full central-difference span are explicit; each bounded TT result reports longitude, motion on both sides, provider identity, and the retained velocity span. | 2024 Mercury direct station over a six-hour difference: analytical and hourly Horizons-fixture midpoints differ by 0.659 seconds and longitudes by 0.0000441 degrees. Invalid-span, wrap, direction, interval, and provider-failure tests. | measured |
| `events::{LunarPhase, LunarPhaseEvent, ecliptic_longitude_lunar_phases}` | Searches all four quarter angles of apparent Moon-minus-Sun ecliptic longitude and returns bounded TT intervals with midpoint great-circle separation and provider identity. | Four April 2024 phases checked against NASA GSFC's minute catalogue through analytical and Horizons providers. Worst provider difference 5.273 seconds; both lanes remain within 20 seconds of the published minute. Sequence, wrap, interval, and provider-failure tests. | measured |
| `events::{EclipseCandidateKind, EclipseCandidateGeometry, EclipseCandidate, eclipse_candidates}` and eclipse radius/model constants | Filters new and full moons with revisioned atmosphere-free spherical geometry. Solar results disclose geocentric disk separation, angular radii, and a conservative global observer-parallax allowance. Lunar results disclose Moon and shadow radii plus antisolar-axis separation, classified as penumbral, partial, or total. Results retain the phase interval and both geometry and provider identities. Local solar type, contacts, visibility, atmosphere, oblateness, and terrain are outside this contract. | Analytical and DE441/Horizons providers agree on NASA's 2024-03-25 penumbral lunar, 2024-04-08 solar, 2024-09-18 partial lunar, 2025-03-14 total lunar, and 2025-03-29 partial solar events. The latter requires the parallax allowance. Both reject ordinary 2024-04-23 full and 2024-05-08 new moons. Measured worst provider phase-root difference is 12.891 seconds and worst reported angular-term difference is 0.000741 degrees. Invalid-distance behavior is typed and tested. | measured |
| `events::{LunarEclipseSearch, LunarEclipseGeometry, LunarEclipseKind, LunarEclipseContactKind, LunarEclipseContact, LunarEclipseCircumstances, lunar_eclipse_circumstances}` | Refines minimum Moon-to-shadow-axis offset and all class-applicable lunar shadow tangencies within a caller-selected full circumstance span. Greatest eclipse and P1/P4, U1/U4, and U2/U3 are bounded TT intervals. Results retain phase interval, greatest geometry, spherical-model revision, provider identity, and span. Short spans are typed errors. The shadow is geocentric, spherical, and atmosphere-free; observer visibility and NASA's Danjon enlargement are outside the model. | Penumbral 2024-03-25, partial 2024-09-18, and total 2025-03-14 events run over both analytical and DE441/Horizons providers. The measured worst provider difference is 8.708 seconds at greatest eclipse and 22.595 seconds across twelve contacts. Greatest events remain within 15.415 seconds of NASA plots; atmosphere-free contacts remain within 247.598 seconds of NASA's enlarged-shadow times. Every result interval is at most one second wide. Invalid and insufficient spans are tested. | measured |
| `events::{AltitudeCrossingSearch, AltitudeCrossingKind, AirlessAltitudeCrossing, AltitudeCrossingError, airless_altitude_crossings}` | Composes position and Earth-orientation providers at every TT sample, applies the revisioned WGS84 airless observer transform, and refines ascending or descending crossings of a caller-selected altitude. Sampling is limited to one hour. Results retain observer, threshold, ephemeris model/snapshot, transform model, and Earth-orientation authority/snapshot. Empty means only that no sampled sign crossing was found; grazing and persistent-state classification are outside the contract. | Boston Sun and Sydney Moon crossing pairs run through analytical and committed DE441/Horizons position providers against direct Horizons quantity-4 airless elevations. Measured worst root residual is 0.232 seconds; Tromso midsummer Sun is the empty control. Direction, exact-boundary, grazing, validation, identity, and each error seam are tested. | measured |
| `events::{AirlessRiseSetKind, AirlessRiseSetEvent, AirlessRiseSetError, AIRLESS_RISE_SET_NAMING, airless_rise_set_events}` | A named projection of `AirlessAltitudeCrossing`: ascending is `Rise`, descending is `Set`. The nested crossing retains the caller-selected airless center threshold and full provenance. This deliberately chooses no refraction, limb, horizon dip, terrain, obstruction, civil-day, or visibility policy. | A Boston Sun 5-degree caller-threshold pair is independently interpolated from Horizons quantity-4 airless elevations; the named fixture events stay within 0.380 seconds and equal the underlying crossing facts. | measured |
| `events::{ConventionalRiseSetSearch, RefractionModel, LimbModel, HorizonDipModel, ConventionalRiseSetPolicy, ConventionalRiseSetEvent, conventional_rise_set_events}` | Finds sampled sign-changing roots of airless center altitude plus caller-selected fixed target refraction, upper-limb offset, and horizon dip. Policy models are validated and retained for replay. Fixed physical-radius limbs use topocentric range; no horizontal parallax is added. | USNO sea-level April 8 2024 Boston Sun and Sydney Moon rise/set minute values are checked through the independent Horizons fixture. The measured worst residual is 27.334 seconds for the fixture lane and 26.455 seconds for the analytical lane. The Sun uses fixed 34-arcminute refraction plus a fixed 16-arcminute limb; the Moon uses the same refraction plus dynamic topocentric semidiameter. Policy parity, errors, and high-latitude empty controls are tested. Altitude-dependent meteorological refraction remains deferred. | measured |
| `events::{MeridianTransitSearch, MeridianTransitSearchError, MeridianTransitKind, MeridianTransitEvent, MeridianTransitError, MAX_MERIDIAN_TRANSIT_STEP_DAYS, TOPOCENTRIC_MERIDIAN_TRANSIT_MODEL, meridian_transits}` | Composes both provider seams into roots of `cos(topocentric declination) * sin(local apparent hour angle)`. Hour angle combines equinox-based GAST with SOFA `apio`'s polar-motion-adjusted local meridian and topocentric true-equator/equinox right ascension. Positive/negative cosine classifies upper/lower; a zero classifier and sampled pole plateau yield no event. Exact search-boundary roots use the available one-sided sign, while sign changes refine to bounded TT intervals. | Six direct roots from Horizons quantity-42 signed local apparent hour angles across Boston Sun, Sydney Moon, and Tromso Sun run through both fixture and analytical position lanes, worst residual 0.220 seconds and intervals at most one second wide. The IAU SOFA `iauApio` vector checks the local-meridian seam. Boston's lower transit is below the horizon; Sydney Moon's upper transit is 213.281 seconds from its altitude maximum. Error and provenance seams are tested. | measured |
| `events::{AltitudeExtremumSearch, AltitudeExtremumKind, AirlessAltitudeExtremum, airless_altitude_extrema}` | Searches sign changes in a central difference of provider-neutral airless altitude. The caller selects and each result retains the full derivative span; both sample step and derivative span are limited to one TT hour. Results report bounded TT intervals and evaluated midpoint altitude plus observer, ephemeris, transform, and Earth-orientation identities. Provider requests extend half the derivative span beyond the search window. These are sampled derivative roots, not meridian transits or proof that unsampled extrema are absent. | Analytical and committed DE441/Horizons providers return ordered minimum/maximum pairs for Boston Sun, Sydney Moon, and Tromso Sun. Against parabolic references derived from direct five-minute Horizons elevations, measured worst residuals are 0.454 seconds and 0.000244 degrees. Unit tests cover exact reversal, plateaus, and boundary zeros; validation and every provider error seam are tested through the public API. | measured |
| `events::{AltitudeCircumstanceSearch, AltitudeThresholdState, AirlessAltitudeSample, AirlessAltitudeCircumstances, airless_altitude_circumstances}` | Combines one threshold's crossings, sampled derivative extrema, and angular classification tolerance. States are `Crosses`, `GrazingCandidate`, `AboveAtAllSamples`, `BelowAtAllSamples`, or `Unresolved`. The names deliberately limit the claim to evaluated samples and refined extrema; they do not mean persistent, circumpolar, or always visible. The aggregate retains search controls and complete provider/transform/EOP provenance even when event lists are empty. | The independent three-site cohort checks crossing and extrema ordering through both position lanes. Tromso controls exercise sampled above, sampled below, and grazing-candidate states; a near-threshold non-extremum remains unresolved. Every provider error seam is tested through the aggregate API. | measured |
| `compat::apparent::{jde_tt_frm_epoch, jde_tt_frm_utc, geocent_apparent_ecl_pos, is_retrograde}` | Anonymous-scalar compatibility wrappers for the 0.1 API. | Covered against the typed path; deprecated contract shape. | measured |
| `verify::JplVerifier::{open, geocent_apparent_ecl_pos, geocent_apparent_state}` and the `verify_events` binary | Opt-in ANISE/SPK reader with SOFA-derived IAU 1976/1980 frame transforms; returns scalar coordinates or a typed state and implements the event provider contract. The binary executes the complete landed event cohort over analytical and live providers. | The 2026-08-24 DE440s receipt compares 26 paired event results with full class/contact agreement, intervals at most one second wide, and measured worst differences of 12.891 seconds for roots, 0.659 seconds for stations, 8.537 seconds for greatest eclipse, and 22.455 seconds for contacts. Kernel digest and ANISE revision are retained. | tooling |

## Angles and coordinate transforms

| Public symbol | Units, frame, and convention | Source, range, and evidence | Status |
| --- | --- | --- | --- |
| `angle::anglr_sepr` | Two spherical longitude/latitude pairs in radians; result in radians. | Elementary spherical relation; inherited example only. | example |
| `angle::deg_frm_dms`, `angle::dms_frm_deg` | Sexagesimal degrees to decimal degrees and inverse. | Algebraic conversion; sign and overflow edge behavior are not measured. | example |
| `angle::deg_frm_hms`, `angle::hms_frm_deg` | Sexagesimal hours to decimal degrees and inverse. | Algebraic conversion; sign and overflow edge behavior are not measured. | example |
| `angle::limit_to_360`, `angle::limit_to_two_PI` | Decimal degrees to the nominal 0-360 interval; radians to the nominal 0-2pi interval. | Algebraic conversion; exact endpoint and non-finite behavior remain legacy behavior. | example |
| `coords::GeographPoint::anglr_sepr`, `coords::EqPoint::anglr_sepr`, `coords::EclPoint::anglr_sepr` | Stored coordinates and result are radians. Geographic longitude is east-positive; latitude north-positive. Equatorial and ecliptic epoch/equinox travel with the caller. | Delegates to `angle::anglr_sepr`; no frame conversion. | example |
| `coords::hr_angl_frm_observer_long` | Greenwich sidereal time, east-positive geographic longitude, and right ascension in radians; local hour angle in radians. | East-longitude sign repaired from upstream issue 18; direct convention vector in `tests/coords.rs`. | corrected |
| `coords::hr_angl_frm_loc_sidr` | Local sidereal time and right ascension in radians; hour angle in radians. | Algebraic relation; inherited evidence only. | example |
| `coords::ecl_long_frm_eq`, `coords::ecl_lat_frm_eq`, `ecl_frm_eq!` | Equatorial to ecliptic spherical transform, radians. Mean inputs require mean obliquity; nutation-corrected inputs require true obliquity. Epoch/equinox are caller-owned. | Inherited relation and examples; no SOFA vector yet. | unverified |
| `coords::asc_frm_ecl`, `coords::dec_frm_ecl`, `eq_frm_ecl!` | Ecliptic to equatorial spherical transform, radians, with the same caller-owned epoch/equinox rule. | Inherited relation and examples; no SOFA vector yet. | unverified |
| `coords::az_frm_eq`, `coords::alt_frm_eq`, `loc_hz_frm_eq!` | Local hour angle, declination, and latitude in radians. Azimuth is the inherited south-zero, east-positive convention in `[-pi, pi]`; altitude is in `[-pi/2, pi/2]`. | IAU SOFA 2023 `iauHd2ae` vector translated only at the azimuth origin. | corrected |
| `coords::hr_angl_frm_hz`, `coords::dec_frm_hz` | Inverse of the preceding transform, using south-zero, east-positive azimuth; radians. | Declination defect from upstream issues 13 and 19 repaired; IAU SOFA 2023 `iauAe2hd` vector translated only at the azimuth origin. | corrected |
| `coords::gal_long_frm_eq`, `coords::gal_lat_frm_eq`, `gal_frm_eq!` | B1950 equatorial radians to the inherited galactic system in radians. | Fixed historical constants; no modern ICRS/Galactic conformance vector. | unverified |
| `coords::asc_frm_gal`, `coords::dec_frm_gal`, `eq_frm_gal!` | Inverse inherited galactic transform, returning B1950 equatorial radians. | Fixed historical constants; no modern conformance vector. | unverified |

## Time, Earth, and atmosphere

| Public symbol | Units, frame, and convention | Source, range, and evidence | Status |
| --- | --- | --- | --- |
| `time::weekday_frm_date`, `time::decimal_day`, `time::decimal_year`, `time::is_leap_year` | Civil Julian or Gregorian calendar fields. Time-zone offset is hours east of UTC in `DayOfMonth`; decimal results are days or years. | Meeus calendar algorithms; inherited examples. Invalid-field behavior is not uniformly checked. | example |
| `time::julian_cent`, `time::julian_mill` | Julian day to elapsed Julian centuries or millennia from J2000. | Algebraic conversion. The input time scale is caller-owned and unstated by the type. | unverified |
| `time::julian_day`, `time::date_frm_julian_day` | Civil calendar to Julian day and inverse. | Meeus calendar algorithms and examples. The returned JD is calendar-based; the type does not encode UTC, UT1, TT, or TDB. | unverified |
| `time::julian_ephemeris_day` | Julian day plus delta-T seconds to JDE. | Algebraic conversion. Callers must supply a compatible UT/TT interpretation. | unverified |
| `time::apprnt_sidr`, `apprnt_sidr!`, `time::mn_sidr` | Sidereal angles in radians; mean sidereal input is UT-based, while nutation and obliquity must share an epoch. | Meeus-era formulae and examples; no SOFA conformance cohort. | unverified |
| `time::delta_t` | Gregorian year/month to estimated TT-UT seconds. | Espenak/Meeus piecewise polynomial; valid range and uncertainty are not encoded or documented in the function. | unverified |
| `planet::earth::flat_fac`, `eq_rad`, `pol_rad`, `ecc_of_meridian`, `rot_angular_velocity` | WGS84 dimensionless factor/eccentricity, radii in metres, rotation in radians per second. | WGS84 constants; exact constant regressions only. | example |
| `planet::earth::approx_geodesic_dist`, `geodesic_dist` | East-positive longitude and north-positive latitude in radians; distance in metres on WGS84. | Inherited approximation and iterative solution; examples only, with no antipodal failure contract. | unverified |
| `planet::earth::rho_sin_cos_phi`, `rho`, `rad_of_parll_lat`, `linear_velocity_at_lat`, `rad_curv_of_meridian`, `geograph_geocent_lat_diff` | Geodetic latitude in radians, height/metres where present; outputs are Earth-radius factors, metres, metres per second, or radians as documented in rustdoc. | WGS84 formulae; inherited examples only. | unverified |
| `planet::earth::equation_of_time` | Apparent/mean solar quantities in radians; result in radians of hour angle. JD time scale and input frame are caller-owned. | Meeus formula; inherited example only. | unverified |
| `planet::earth::angl_betwn_diurnal_path_and_hz` | Declination and observer latitude in radians; angle in radians. | Spherical relation; inherited example only. | example |
| `atmos::refrac_frm_apprnt_alt_15`, `refrac_frm_true_alt_15` | Apparent or true altitude in radians; standard-atmosphere refraction in radians for the low-altitude formula. | Meeus empirical formula; source claims are not independently measured. | unverified |
| `atmos::refrac_frm_apprnt_alt`, `refrac_frm_true_alt` | Apparent or true altitude in radians; standard-atmosphere refraction in radians. | Meeus empirical formula, source-stated accuracy up to 0.07 arcminute; no independent cohort. | unverified |
| `atmos::refrac_by_pressr`, `refrac_by_temp` | Pressure in millibars or temperature in Celsius to a dimensionless refraction multiplier. | Meeus scaling relation; inherited example only. | example |

## Solar-system positions and corrections

| Public symbol | Units, frame, and convention | Source, range, and evidence | Status |
| --- | --- | --- | --- |
| `aberr::sol_aberr` | Sun-Earth distance in AU to solar aberration in radians. | Meeus relation; inherited example only. | example |
| `aberr::stell_aberr_in_eq_coords` | Equatorial radians plus solar true ecliptic longitude, eccentricity, perihelion longitude, and true obliquity; correction in equatorial radians. Epoch/equinox are caller-owned. | Meeus relation; one inherited example. | unverified |
| `nutation::nutation`, `nutation::nutation_in_eq_coords` | JD to IAU 1980 nutation in longitude/obliquity radians; optional equatorial correction in radians. Input scale and equinox are caller-owned. | Truncated IAU 1980/Meeus series; inherited examples, and indirectly exercised by the DE440s apparent cohort. | unverified |
| `ecliptic::mn_oblq_laskar` | JD to mean obliquity in radians. | Laskar polynomial; source states 0.01 arcsecond accuracy without a documented interval. | unverified |
| `ecliptic::mn_oblq_IAU` | JD to mean obliquity in radians. | Legacy IAU polynomial; exact model revision and interval are not named in rustdoc. | unverified |
| `ecliptic::eclip_points_on_hz`, `ecliptic::angl_betwn_eclip_and_hz` | Obliquity, observer latitude, and sidereal angle in radians; ecliptic/horizon relation in radians. | Meeus spherical relation; inherited examples only. | unverified |
| `precess::annual_precess`, `precess::precess_eq_coords`, `precess::precess_eq_coords_FK5`, `precess::precess_ecl_coords`, `precess::precess_orb_elements` | Equatorial, ecliptic, or orbital angles in radians; epochs supplied as JD where documented. FK5 correction is explicitly named only for one function. | Meeus-era IAU 1976/FK5 formulae; examples only. No SOFA cohort. | unverified |
| `parallax::eq_hz_parallax` | Earth distance in AU to equatorial horizontal parallax in radians. | Meeus relation; inherited example only. | example |
| `parallax::topocent_eq_coords`, `parallax::topopcent_ecl_coords` | Geocentric equatorial/ecliptic radians, observer latitude/height, distance, and sidereal quantities to topocentric radians. Longitude convention and epoch travel with caller. | Meeus formulae; inherited examples only. The ecliptic function retains its misspelled public name. | unverified |
| `sun::semidiameter` | Sun-Earth distance in AU to angular semidiameter in radians. | Meeus relation; inherited example. | example |
| `sun::geocent_ecl_pos`, `sun::ecl_coords_to_FK5`, `sun::geocent_rect_coords`, `sun::ephemeris`, `sun::synodic_rot` | Legacy solar geometry: JD/JDE-like anonymous days, ecliptic/equatorial radians, AU, and Carrington rotation number as documented locally. Mean/apparent and epoch conventions vary per function. | VSOP87D Earth plus Meeus corrections; solar position is measured only through `apparent`, not as each raw stage. | unverified |
| `lunar::eq_hz_parllx`, `lunar::semidiameter`, `lunar::inc_of_mn_lunar_eq` | Earth-Moon kilometres to angular radians, or fixed inclination radians. | Meeus constants and relations; inherited examples only. | example |
| `lunar::optical_libr`, `physical_libr`, `total_libr`, `pos_angl_of_axis_of_rot`, `topocent_libr_by_diff_corrections` | JD plus lunar ecliptic/equatorial and observer quantities in radians; librations and position angles in radians. Frame and time-scale compatibility are caller-owned. | Meeus formulae; selected examples only. | unverified |
| `lunar::geocent_ecl_pos` | Anonymous JD/JDE input; geocentric ecliptic longitude/latitude of date in radians and distance in kilometres. | Partial Chapront ELP-2000/82. Measured only through the composed `apparent` pipeline: worst 5 millidegrees against DE440s over 1885-2099. | unverified |
| `lunar::mn_ascend_node`, `true_ascend_node`, `mn_perigee` | Julian centuries from J2000 to ecliptic longitude radians. Mean/true and equinox follow the named Meeus formulae. | Inherited examples only; validity interval unstated. | unverified |
| `lunar::bright_limb`, `illum_frac_frm_eq_coords`, `illum_frac_frm_ecl_coords` | Solar/lunar equatorial or ecliptic radians to bright-limb angle radians or illuminated fraction. Inputs must share frame and epoch. | Meeus geometry; inherited examples only. | unverified |
| `lunar::time_of_passage_through_nodes` | Approximate civil date to ascending and descending passage JDE values. | Meeus chapter 51-style series; one inherited example, validity interval unstated. | unverified |
| `lunar::time_of_phase` | Approximate civil date and phase to closest phase JDE TT. | Meeus chapter 49; source-stated mean error 3.8 seconds for 1980 to mid-2020. Discarded quarter correction repaired and checked against PyMeeus. | corrected |
| `planet::illum_frac_frm_phase_angl`, `illum_frac_frm_dist`, `phase_angl`, `pos_angle_of_bright_limb` | Phase geometry in radians and AU, returning fraction or angle radians. | Meeus relations; inherited examples only. | example |
| `planet::semidiameter` | Planet identity and Earth distance in AU to angular semidiameter radians; optional polar/equatorial choice. | Meeus tabulated constants; inherited examples only. | unverified |
| `planet::orb_elements`, `heliocent_coords`, `geocent_geomet_ecl_coords`, `geocent_apprnt_ecl_coords`, `ecl_coords_to_FK5`, `geocent_eq_coords`, `heliocent_coords_frm_orb_elements` | Anonymous JD/JDE inputs; VSOP87D heliocentric ecliptic-of-date radians/AU and staged geocentric/apparent transforms. Exact frame varies by stage and is documented only in rustdoc. | VSOP87D plus Meeus corrections. The composed result is measured through `apparent`; raw stages are not independently certified. | unverified |
| `planet::apprnt_mag_muller`, `planet::apprnt_mag_84` | Planet identity, distances in AU, and phase angle in radians to apparent magnitude. | G. Muller or *Astronomical Almanac* 1984 formulae; inherited examples only. | unverified |
| `pluto::semdiameter`, `pluto::apprnt_mag_84` | Earth distance and heliocentric distance in AU to angular semidiameter radians or magnitude. | Meeus/*Astronomical Almanac* relations; inherited examples. | unverified |
| `pluto::heliocent_pos` | Anonymous JD to J2000 heliocentric ecliptic longitude/latitude radians and AU. | Meeus analytical series, explicitly 1885-2099; measured after precession in `apparent`. | unverified |
| `pluto::mn_orb_elements_2000AD` | Fixed J2000 mean orbital elements in radians and AU. | Meeus tabulation; exact-value regression only. | example |
| `planet::mars::north_pol_eq_coords_J1950`, `north_pol_eq_coords_J2000`, `north_pol_ecl_coords`, `ephemeris` | Mars pole/physical ephemeris quantities in equatorial/ecliptic radians; the fixed equinox is named where applicable. | Meeus physical ephemeris; inherited example only. | unverified |
| `planet::jupiter::eq_semidiameter`, `pol_semidiameter`, `ephemeris` | Earth distance in AU and observer/solar geometry to angular radii and physical ephemeris radians. | Meeus physical ephemeris; inherited example only. | unverified |
| `planet::jupiter::moon::apprnt_rect_coords` | Anonymous JD and Galilean moon identity to apparent rectangular coordinates in Jupiter equatorial radii. | Meeus low-accuracy method; inherited example only. | unverified |
| `planet::saturn::apprnt_mag_muller`, `apprnt_mag_84`, `pol_semidiameter`, `eq_semidiameter` | Distances in AU and ring angles in radians to magnitude or angular radius radians. | Muller/*Astronomical Almanac* and Meeus relations; inherited examples only. | unverified |
| `planet::saturn::moon::apprnt_rect_coords` | Anonymous JD and moon identity to apparent rectangular coordinates in Saturn equatorial radii. | Meeus/Dourneau-style analytical theory; inherited example only. | unverified |
| `planet::saturn::ring::inc`, `ascend_node`, `elements`, `inn_edge_outer_ring`, `out_edge_inner_ing`, `inn_edge_inner_ring`, `inn_edge_dusk_ring` | Julian centuries/JD and ring angles in radians; edge helpers return projected radii in the caller's units. | Meeus ring geometry; inherited examples only. Misspelled public name retained. | unverified |

## Stellar, orbital, and event helpers

| Public symbol | Units, frame, and convention | Source, range, and evidence | Status |
| --- | --- | --- | --- |
| `asteroid::diameter`, `asteroid::apparent_diameter` | Absolute magnitude/albedo to kilometres; true diameter kilometres and Earth distance kilometres to angular radians. | Meeus empirical relations; inherited examples only. | unverified |
| `star::combined_mag`, `combined_mag_of_many`, `brightness_ratio`, `mag_diff`, `abs_mag_frm_parallax`, `abs_mag_frm_dist` | Astronomical magnitudes, parsecs, and arcsecond parallax as documented; scalar results. | Photometric definitions; inherited examples only, with domain errors not typed. | example |
| `star::angl_between_north_celes_and_eclip_pole`, `eq_coords_frm_motion`, `proper_motion_in_eq_coords` | Equatorial/ecliptic angles and proper motions in radians or radians per year as documented; epochs/equinoxes are caller-owned. | Meeus stellar-motion relations; inherited examples only. | unverified |
| `binary_star::mn_ann_motion_of_compan`, `mn_anom_of_compan`, `rad_vec`, `true_anom`, `apprnt_coords_angl`, `anglr_sepr`, `ecc_of_apprnt_orb` | Period/time quantities in years, angles in radians, and orbital axes in arcseconds as documented. | Meeus binary-star formulae; inherited examples only. | unverified |
| `orbit::elliptic::true_anom`, `rad_vec_frm_ecc_anom`, `rad_vec_frm_true_anom`, `ecc_anom`, `vel`, `perih_vel`, `aph_vel`, `length_ramanujan`, `length`, `semimaj_axis`, `mn_motion`, `passage_through_node` | Angles in radians, axes/distances in AU, elapsed time in days, and velocity in km/s where documented. | Meeus/two-body relations; inherited examples only. Iteration/domain failures are not typed. | unverified |
| `orbit::parabolic::true_anom_and_rad_vec`, `passage_through_node` | Times/JD in days, perihelion distance in AU, angle in radians, distance in AU. | Meeus parabolic-orbit relations; inherited examples only. | unverified |
| `orbit::near_parabolic::true_anom_and_rad_vec` | Times/JD in days, perihelion distance in AU, eccentricity, caller-selected numerical tolerance; radians/AU result or string error. | Meeus iterative relation; inherited example only. | unverified |
| `interpol::three_values`, `interpol::five_values` | Dimensionless interpolation parameter and caller-unit ordinates; result preserves ordinate unit. | Meeus interpolation formulae; inherited examples only. | example |
| `misc::parllc_angl`, `misc::parllc_angl_on_hz` | Observer latitude, hour angle, and declination in radians; parallactic angle in radians. | Spherical relation; inherited examples only. | example |
| `transit::time` | Transit/rise/set selector, three equatorial samples, apparent Greenwich sidereal angle, delta-T seconds, optional lunar parallax; result is a fraction of the civil day. Inputs must share epoch/frame. | Meeus interpolation algorithm; inherited example only. High-latitude and no-event behavior are not represented by a typed result. | unverified |
| `util::round_upto_digits`, `Horner_eval!` | Dimension-preserving numerical helpers. | Algebraic helpers; unit tests only. | example |

## Public constants and types

The typed foundation's public constructors, conversions, and accessors are:
`JulianDate::{from_parts, from_julian_day, from_epoch, to_epoch,
from_utc_epoch, parts, day, offset_days}`; `TimeOffset::{from_seconds, seconds,
days}`;
`Angle::{from_radians, from_degrees, from_arcseconds, radians,
degrees, arcseconds, abs}`; `Longitude`, `EastLongitude`, and `Latitude`
constructors and angle/radian/degree accessors; `Length::{from_meters, meters,
kilometers}`; `Distance::{from_meters, from_kilometers,
from_astronomical_units, meters, kilometers, astronomical_units}`;
`Observer::{new, longitude, latitude, height}`; `Direction::{new, longitude,
latitude, to_unit_vector}`; `UnitVector::{new, components, to_direction}`;
`Rotation::{matrix, apply, inverse}`; `Model::{new, name, revision}`;
`Accuracy::{new, max_angular_error, evidence, authority, scope}`;
`Modelled::{new, value, into_value, model, accuracy}`; and `State::{new,
epoch, direction, distance}`. These are contract-preserving value operations,
not additional astronomical models.

`angle::TWO_PI`; `consts::{GAUSS_GRAV, SPEED_OF_LIGHT,
EARTH_MOON_MASS_RATIO, SUN_EARTH_MASS_RATIO}`; all exported WGS72 and WGS84
constants; and the public enums/records in `coords`, `time`, `orbit`, `lunar`,
`planet`, `planet::mars`, `planet::jupiter`, `planet::jupiter::moon`,
`planet::saturn::moon`, `planet::saturn::ring`, `transit`, `apparent`, and
`verify` are part of the 0.1.x compatibility surface. Their fields retain the
units stated in local rustdoc. Where a type is only a bundle of anonymous
`f64` values, it does not add frame, time-scale, model, or range safety.

## T1 through T3 conclusion and T4 event addenda

The three known upstream numerical defects are repaired by the accompanying
T1 change. The inventory is complete enough to expose the real boundary:
`apparent` is the measured production path; the inherited modules remain a
documented compatibility surface and are not promoted to verified
calculations by their example tests. T1's disclosure gate is met when this
map, the repairs, and their tests land. T2 moves that anonymous catalogue
behind `compat`. Primary calculations now carry time scale, frame, units,
model revision, and evidence in their types or returned state. Invalid TT/UT1
or GCRS/of-date combinations cannot enter the orientation and apparent APIs as
interchangeable floating-point values.

T3 completes the analytical position gate: the ten-body geocentric path has
explicit corrections and broad date evidence, while the observer path binds
Earth rotation and site geometry to separately typed inputs and retains the
external Earth-orientation snapshot. Event searches, atmospheric policy, and
interpretation remain outside this position-provider boundary.

T4a makes that boundary executable for more than one implementation and lands
the first typed event search. T4b adds provider-neutral stationary-point roots
with an explicit numerical definition. T4c adds all four lunar quarter-phase
roots while retaining latitude through the reported center separation. T4d
adds provider-neutral global solar candidates and geocentric lunar shadow
classes. T4e refines lunar greatest events and atmosphere-free shadow contacts
without claiming observer visibility or local solar circumstances. The live
DE440s receipt then executes every landed family over the real verifier. T4f
and T4g add airless observer-altitude facts; T4h names only caller-threshold
airless crossings and separately establishes topocentric meridian transits.
T4i adds conventional rise/set policy without converting it into a visibility
claim. The first Sky-home consumer then forced the TT-to-epoch inverse and an
optional provider-wide accuracy disclosure. Its cross-repository plan is owned
at `turnstone/design_docs/2026-08-26_sky_home_timeline_plan.md`. Observer solar
contacts, visibility, and further extrema remain explicit in `ROADMAP.md`.
