// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use std::f64::consts::PI;
use std::fmt;

use turquet::apparent::{ANALYTICAL_APPARENT, ApparentBody, ApparentError};
use turquet::events::{
    AIRLESS_RISE_SET_NAMING, AirlessRiseSetKind, AltitudeCircumstanceSearch,
    AltitudeCircumstanceSearchError, AltitudeCrossingError, AltitudeCrossingKind,
    AltitudeCrossingSearch, AltitudeCrossingSearchError, AltitudeExtremumKind,
    AltitudeExtremumSearch, AltitudeExtremumSearchError, AltitudeThresholdState,
    CONVENTIONAL_RISE_SET_CIRCUMSTANCES, ConventionalRiseSetError, ConventionalRiseSetKind,
    ConventionalRiseSetPolicy, ConventionalRiseSetPolicyError, ConventionalRiseSetSearch,
    ConventionalRiseSetSearchError, HorizonDipModel, LimbModel, MeridianTransitKind,
    MeridianTransitSearch, MeridianTransitSearchError, RefractionModel, SearchWindow,
    TOPOCENTRIC_MERIDIAN_TRANSIT_MODEL, USNO_STANDARD_REFRACTION, USNO_STANDARD_SOLAR_LIMB,
    airless_altitude_circumstances, airless_altitude_crossings, airless_altitude_extrema,
    airless_rise_set_events, conventional_rise_set_events, meridian_transits,
};
use turquet::foundation::{
    Angle, Direction, Distance, EastLongitude, JulianDate, Latitude, Length, Longitude, Model,
    Observer, ScaleAwareEpoch, State, TerrestrialTime, TimeOffset, TrueEclipticEquinoxOfDate,
    UniversalTime1,
};
use turquet::observer::{AIRLESS_TOPOCENTRIC_TRANSFORM, EarthOrientation};
use turquet::provider::{
    AnalyticalEphemeris, ConstantOffsetEarthOrientation, EarthOrientationProvider,
    GeocentricPositionProvider,
};

const HORIZONS_VECTORS: &str = include_str!("vectors/altitude_crossings_horizons.tsv");
const HORIZONS_FIXTURE_MODEL: Model = Model::new(
    "NASA/JPL Horizons DE441 altitude and transit fixture",
    "2026-08-25",
);
const HORIZONS_FIXTURE_SNAPSHOT: &str =
    "Horizons API 1.2 / DE441 / altitude and transit fixture generated 2026-08-25";
const HORIZONS_EOP_AUTHORITY: &str = "NASA/JPL Horizons quantity 49";
const HORIZONS_EOP_SNAPSHOT: &str = "eop.260824.p261120; polar motion approximated as zero";
const MEAN_LUNAR_LIMB: Model = Model::new("mean lunar physical-radius limb", "1");

#[derive(Clone, Copy, Debug)]
struct HorizonsAltitudeRow {
    case: &'static str,
    longitude: f64,
    latitude: f64,
    height_meters: f64,
    body: ApparentBody,
    tt_day: f64,
    ut1_day: f64,
    ecliptic_longitude: f64,
    ecliptic_latitude: f64,
    distance_au: f64,
    direct_altitude: f64,
    direct_local_hour_angle_hours: f64,
}

struct HorizonsAltitudeFixture {
    rows: Vec<HorizonsAltitudeRow>,
}

impl HorizonsAltitudeFixture {
    fn parse() -> Self {
        let mut rows = Vec::new();
        for line in HORIZONS_VECTORS.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 10, "altitude and transit vector column count");
            let case: &'static str = match fields[0] {
                "boston_sun" => "boston_sun",
                "sydney_moon" => "sydney_moon",
                "tromso_sun_empty" => "tromso_sun_empty",
                other => panic!("unknown altitude fixture case {}", other),
            };
            let coordinates: Vec<f64> = fields[1]
                .split(',')
                .map(|field| field.parse().expect("site coordinate"))
                .collect();
            let utc_day: f64 = fields[2].parse().expect("UTC Julian day");
            let body = match fields[3] {
                "sun" => ApparentBody::Sun,
                "moon" => ApparentBody::Moon,
                other => panic!("unknown altitude fixture body {}", other),
            };
            let utc = ScaleAwareEpoch::from_jde_utc(utc_day);
            let tt = JulianDate::<TerrestrialTime>::from_epoch(utc);
            let dut1: f64 = fields[9].parse().expect("DUT1 seconds");
            let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
                utc,
                TimeOffset::from_seconds(dut1).expect("finite DUT1"),
            );
            rows.push(HorizonsAltitudeRow {
                case,
                longitude: coordinates[0],
                latitude: coordinates[1],
                height_meters: coordinates[2] * 1_000.0,
                body,
                tt_day: tt.day(),
                ut1_day: ut1.day(),
                ecliptic_longitude: fields[4].parse().expect("ecliptic longitude"),
                ecliptic_latitude: fields[5].parse().expect("ecliptic latitude"),
                distance_au: fields[6].parse().expect("range AU"),
                direct_altitude: fields[7].parse().expect("direct altitude"),
                direct_local_hour_angle_hours: fields[8]
                    .parse()
                    .expect("direct local apparent hour angle"),
            });
        }
        Self { rows }
    }

    fn case_rows(&self, case: &str) -> Vec<&HorizonsAltitudeRow> {
        self.rows.iter().filter(|row| row.case == case).collect()
    }

    fn for_case<'a>(&'a self, case: &'a str) -> HorizonsAltitudeCase<'a> {
        HorizonsAltitudeCase {
            fixture: self,
            case,
        }
    }
}

struct HorizonsAltitudeCase<'a> {
    fixture: &'a HorizonsAltitudeFixture,
    case: &'a str,
}

impl<'a> HorizonsAltitudeCase<'a> {
    fn bracket(
        &self,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<(&HorizonsAltitudeRow, &HorizonsAltitudeRow, f64), FixtureError> {
        let rows = self.fixture.case_rows(self.case);
        for pair in rows.windows(2) {
            let left = pair[0];
            let right = pair[1];
            let rounding_margin_days = 1e-8;
            if epoch.day() >= left.tt_day - rounding_margin_days
                && epoch.day() <= right.tt_day + rounding_margin_days
            {
                let fraction = ((epoch.day() - left.tt_day) / (right.tt_day - left.tt_day))
                    .max(0.0)
                    .min(1.0);
                return Ok((left, right, fraction));
            }
        }
        Err(FixtureError::OutsideFixture)
    }
}

impl<'a> GeocentricPositionProvider for HorizonsAltitudeCase<'a> {
    type Error = FixtureError;

    fn model(&self) -> Model {
        HORIZONS_FIXTURE_MODEL
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some(HORIZONS_FIXTURE_SNAPSHOT)
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let (left, right, fraction) = self.bracket(epoch)?;
        if left.body != body || right.body != body {
            return Err(FixtureError::UnsupportedBody);
        }
        let longitude = left.ecliptic_longitude
            + fraction * signed_degrees(right.ecliptic_longitude - left.ecliptic_longitude);
        let latitude =
            left.ecliptic_latitude + fraction * (right.ecliptic_latitude - left.ecliptic_latitude);
        let distance = left.distance_au + fraction * (right.distance_au - left.distance_au);
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).expect("fixture longitude"),
                Latitude::from_degrees(latitude).expect("fixture latitude"),
            ),
            Distance::from_astronomical_units(distance).expect("fixture distance"),
        ))
    }
}

impl<'a> EarthOrientationProvider for HorizonsAltitudeCase<'a> {
    type Error = FixtureError;

    fn authority(&self) -> &str {
        HORIZONS_EOP_AUTHORITY
    }

    fn data_snapshot(&self) -> &str {
        HORIZONS_EOP_SNAPSHOT
    }

    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        let (left, right, fraction) = self.bracket(epoch)?;
        let ut1_day = left.ut1_day + fraction * (right.ut1_day - left.ut1_day);
        Ok(EarthOrientation::zero_polar_motion(
            JulianDate::from_julian_day(ut1_day).expect("fixture UT1"),
            HORIZONS_EOP_AUTHORITY,
            HORIZONS_EOP_SNAPSHOT,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixtureError {
    UnsupportedBody,
    OutsideFixture,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FixtureError::UnsupportedBody => formatter.write_str("unsupported fixture body"),
            FixtureError::OutsideFixture => formatter.write_str("epoch outside fixture"),
        }
    }
}

impl std::error::Error for FixtureError {}

#[derive(Clone, Copy)]
struct DirectAltitudeCrossing {
    kind: AltitudeCrossingKind,
    tt_day: f64,
}

#[derive(Clone, Copy)]
struct DirectAltitudeExtremum {
    kind: AltitudeExtremumKind,
    tt_day: f64,
    altitude_degrees: f64,
}

#[derive(Clone, Copy)]
struct DirectMeridianTransit {
    kind: MeridianTransitKind,
    tt_day: f64,
}

fn direct_altitude_crossings(
    rows: &[&HorizonsAltitudeRow],
    threshold_degrees: f64,
) -> Vec<DirectAltitudeCrossing> {
    let mut crossings = Vec::new();
    for pair in rows.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let left_value = left.direct_altitude - threshold_degrees;
        let right_value = right.direct_altitude - threshold_degrees;
        if left_value.signum() == right_value.signum() {
            continue;
        }
        let fraction = -left_value / (right_value - left_value);
        crossings.push(DirectAltitudeCrossing {
            kind: if right_value > 0.0 {
                AltitudeCrossingKind::Ascending
            } else {
                AltitudeCrossingKind::Descending
            },
            tt_day: left.tt_day + fraction * (right.tt_day - left.tt_day),
        });
    }
    crossings
}

fn direct_meridian_transits(rows: &[&HorizonsAltitudeRow]) -> Vec<DirectMeridianTransit> {
    let mut transits = Vec::new();
    for pair in rows.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let left_hour_angle = left.direct_local_hour_angle_hours * PI / 12.0;
        let right_hour_angle = right.direct_local_hour_angle_hours * PI / 12.0;
        let left_value = left_hour_angle.sin();
        let right_value = right_hour_angle.sin();
        if left_value.signum() == right_value.signum() {
            continue;
        }
        let fraction = -left_value / (right_value - left_value);
        let hour_angle_delta = (right_hour_angle - left_hour_angle + PI).rem_euclid(2.0 * PI) - PI;
        let root_hour_angle = left_hour_angle + fraction * hour_angle_delta;
        transits.push(DirectMeridianTransit {
            kind: if root_hour_angle.cos() > 0.0 {
                MeridianTransitKind::Upper
            } else {
                MeridianTransitKind::Lower
            },
            tt_day: left.tt_day + fraction * (right.tt_day - left.tt_day),
        });
    }
    transits
}

fn direct_altitude_extrema(rows: &[&HorizonsAltitudeRow]) -> Vec<DirectAltitudeExtremum> {
    let mut extrema = Vec::new();
    for triple in rows.windows(3) {
        let left = triple[0];
        let middle = triple[1];
        let right = triple[2];
        let kind = if middle.direct_altitude > left.direct_altitude
            && middle.direct_altitude > right.direct_altitude
        {
            AltitudeExtremumKind::Maximum
        } else if middle.direct_altitude < left.direct_altitude
            && middle.direct_altitude < right.direct_altitude
        {
            AltitudeExtremumKind::Minimum
        } else {
            continue;
        };
        let denominator =
            left.direct_altitude + right.direct_altitude - 2.0 * middle.direct_altitude;
        let offset_samples = (left.direct_altitude - right.direct_altitude) / (2.0 * denominator);
        let sample_days = right.tt_day - middle.tt_day;
        let altitude = middle.direct_altitude
            - (right.direct_altitude - left.direct_altitude)
                * (right.direct_altitude - left.direct_altitude)
                / (8.0 * denominator);
        extrema.push(DirectAltitudeExtremum {
            kind,
            tt_day: middle.tt_day + offset_samples * sample_days,
            altitude_degrees: altitude,
        });
    }
    extrema
}

fn assert_threshold_state(state: &AltitudeThresholdState, expected: &str) {
    match (state, expected) {
        (AltitudeThresholdState::Crosses, "crosses") => {},
        (AltitudeThresholdState::AboveAtAllSamples { .. }, "above") => {},
        _ => panic!("expected {} sampled state, got {:?}", expected, state),
    }
}

fn signed_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

fn observer(longitude: f64, latitude: f64, height_meters: f64) -> Observer {
    Observer::new(
        EastLongitude::from_degrees(longitude).unwrap(),
        Latitude::from_degrees(latitude).unwrap(),
        Length::from_meters(height_meters).unwrap(),
    )
}

fn tt_from_utc(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> JulianDate<TerrestrialTime> {
    JulianDate::from_epoch(ScaleAwareEpoch::from_gregorian_utc(
        year, month, day, hour, minute, 0, 0,
    ))
}

fn constant_orientation(
    tt: JulianDate<TerrestrialTime>,
    utc: hifitime::Epoch,
    dut1_seconds: f64,
) -> ConstantOffsetEarthOrientation {
    let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
        utc,
        TimeOffset::from_seconds(dut1_seconds).unwrap(),
    );
    ConstantOffsetEarthOrientation::new(
        tt,
        EarthOrientation::zero_polar_motion(ut1, "test EOP", "constant DUT1 and zero pole"),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestError(&'static str);

impl fmt::Display for TestError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for TestError {}

struct FailingPosition;

impl GeocentricPositionProvider for FailingPosition {
    type Error = TestError;

    fn model(&self) -> Model {
        Model::new("failing position", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        _epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        Err(TestError("position"))
    }
}

struct FailingOrientation;

impl EarthOrientationProvider for FailingOrientation {
    type Error = TestError;

    fn authority(&self) -> &str {
        "failing"
    }

    fn data_snapshot(&self) -> &str {
        "failure"
    }

    fn at(&self, _epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        Err(TestError("orientation"))
    }
}

struct WrongIdentityOrientation {
    reference_epoch: JulianDate<TerrestrialTime>,
    reference_ut1: JulianDate<UniversalTime1>,
}

impl EarthOrientationProvider for WrongIdentityOrientation {
    type Error = TestError;

    fn authority(&self) -> &str {
        "declared"
    }

    fn data_snapshot(&self) -> &str {
        "declared snapshot"
    }

    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        let elapsed = epoch.day() - self.reference_epoch.day();
        Ok(EarthOrientation::zero_polar_motion(
            self.reference_ut1.offset_days(elapsed).unwrap(),
            "returned",
            "returned snapshot",
        ))
    }
}

struct WrongEpochPosition;

impl GeocentricPositionProvider for WrongEpochPosition {
    type Error = ApparentError;

    fn model(&self) -> Model {
        Model::new("wrong epoch", "1")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        AnalyticalEphemeris.position(body, epoch.offset_days(1.0).unwrap())
    }
}

#[path = "altitude_crossings/airless.rs"]
mod airless;

#[path = "altitude_crossings/circumstances.rs"]
mod circumstances;

#[path = "altitude_crossings/horizons.rs"]
mod horizons;

#[path = "altitude_crossings/boundaries.rs"]
mod boundaries;
