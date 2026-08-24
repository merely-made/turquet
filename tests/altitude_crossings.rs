// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use std::fmt;

use turquet::apparent::{ApparentBody, ApparentError, ANALYTICAL_APPARENT};
use turquet::events::{
    airless_altitude_crossings, AltitudeCrossingError, AltitudeCrossingKind,
    AltitudeCrossingSearch, AltitudeCrossingSearchError, SearchWindow,
};
use turquet::foundation::{
    Angle, Direction, Distance, EastLongitude, JulianDate, Latitude, Length, Longitude, Model,
    Observer, ScaleAwareEpoch, State, TerrestrialTime, TimeOffset, TrueEclipticEquinoxOfDate,
    UniversalTime1,
};
use turquet::observer::{EarthOrientation, AIRLESS_TOPOCENTRIC_TRANSFORM};
use turquet::provider::{
    AnalyticalEphemeris, ConstantOffsetEarthOrientation, EarthOrientationProvider,
    GeocentricPositionProvider,
};

const HORIZONS_VECTORS: &str = include_str!("vectors/altitude_crossings_horizons.tsv");
const HORIZONS_FIXTURE_MODEL: Model =
    Model::new("NASA/JPL Horizons DE441 altitude fixture", "2026-08-24");
const HORIZONS_FIXTURE_SNAPSHOT: &str =
    "Horizons API 1.2 / DE441 / altitude fixture generated 2026-08-24";
const HORIZONS_EOP_AUTHORITY: &str = "NASA/JPL Horizons quantity 49";
const HORIZONS_EOP_SNAPSHOT: &str = "eop.260824.p261120; polar motion approximated as zero";

#[test]
fn boston_sun_has_one_ascending_and_one_descending_crossing() {
    let utc = ScaleAwareEpoch::from_gregorian_utc(2024, 4, 8, 0, 0, 0, 0);
    let start = JulianDate::<TerrestrialTime>::from_epoch(utc);
    let end = start.offset_days(1.0).unwrap();
    let observer = observer(-71.0589, 42.3601, 43.0);
    let orientation = constant_orientation(start, utc, -0.01669);
    let window = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    let search = AltitudeCrossingSearch::new(window, Angle::from_degrees(0.0).unwrap()).unwrap();

    let events = airless_altitude_crossings(
        &AnalyticalEphemeris,
        &orientation,
        observer,
        ApparentBody::Sun,
        search,
    )
    .expect("Boston Sun crossings");

    assert_eq!(events.len(), 2);
    assert_eq!(events[0].kind(), AltitudeCrossingKind::Ascending);
    assert_eq!(events[1].kind(), AltitudeCrossingKind::Descending);
    assert!(events[0].interval().end().day() < events[1].interval().start().day());
    for event in events {
        assert!(event.interval().width_days() <= 1.0 / 86_400.0);
        assert_eq!(event.body(), ApparentBody::Sun);
        assert_eq!(event.observer(), observer);
        assert_eq!(event.threshold(), Angle::from_degrees(0.0).unwrap());
        assert_eq!(event.provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(event.provider_snapshot(), None);
        assert_eq!(event.transform_model(), AIRLESS_TOPOCENTRIC_TRANSFORM);
        assert_eq!(event.earth_orientation_authority(), "test EOP");
        assert_eq!(
            event.earth_orientation_snapshot(),
            "constant DUT1 and zero pole"
        );
    }
}

#[test]
fn high_latitude_empty_result_makes_no_state_classification() {
    let utc = ScaleAwareEpoch::from_gregorian_utc(2024, 6, 21, 0, 0, 0, 0);
    let start = JulianDate::<TerrestrialTime>::from_epoch(utc);
    let end = start.offset_days(1.0).unwrap();
    let window = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    let search = AltitudeCrossingSearch::new(window, Angle::from_degrees(0.0).unwrap()).unwrap();
    let orientation = constant_orientation(start, utc, -0.012);

    let events = airless_altitude_crossings(
        &AnalyticalEphemeris,
        &orientation,
        observer(18.9553, 69.6492, 0.0),
        ApparentBody::Sun,
        search,
    )
    .expect("Tromso search remains calculable");

    assert!(events.is_empty());
}

#[test]
fn both_position_providers_match_direct_horizons_altitude_crossings() {
    for expected in &[
        "oracle: NASA/JPL Horizons API 1.2, DE441",
        "five-minute UTC grid",
        "Boston Sun ordinary pair",
        "Sydney Moon ordinary pair",
        "Tromso Sun midsummer empty control",
        "EOP eop.260824.p261120",
        "fetch_horizons_altitude_crossing_vectors.ps1",
    ] {
        assert!(
            HORIZONS_VECTORS.lines().any(|line| line.contains(expected)),
            "altitude fixture header must record {}",
            expected
        );
    }
    let fixture = HorizonsAltitudeFixture::parse();
    assert_eq!(fixture.rows.len(), 867);

    for &(case, expected_body, expected_crossings) in &[
        ("boston_sun", ApparentBody::Sun, 2_usize),
        ("sydney_moon", ApparentBody::Moon, 2_usize),
        ("tromso_sun_empty", ApparentBody::Sun, 0_usize),
    ] {
        let rows = fixture.case_rows(case);
        assert_eq!(rows.len(), 289);
        assert!(rows.iter().all(|row| row.body == expected_body));
        let first = rows[0];
        let last = rows[rows.len() - 1];
        let observer = observer(first.longitude, first.latitude, first.height_meters);
        let window = SearchWindow::new(
            JulianDate::from_julian_day(first.tt_day).unwrap(),
            JulianDate::from_julian_day(last.tt_day).unwrap(),
            1.0 / 24.0,
            1.0 / 86_400.0,
        )
        .unwrap();
        let search =
            AltitudeCrossingSearch::new(window, Angle::from_degrees(0.0).unwrap()).unwrap();
        let references = direct_altitude_crossings(&rows);
        assert_eq!(references.len(), expected_crossings);

        let fixture_events = airless_altitude_crossings(
            &fixture.for_case(case),
            &fixture.for_case(case),
            observer,
            expected_body,
            search,
        )
        .expect("fixture-provider altitude search");
        let analytical_events = airless_altitude_crossings(
            &AnalyticalEphemeris,
            &fixture.for_case(case),
            observer,
            expected_body,
            search,
        )
        .expect("analytical-provider altitude search");
        assert_eq!(fixture_events.len(), expected_crossings);
        assert_eq!(analytical_events.len(), expected_crossings);

        for (index, reference) in references.iter().enumerate() {
            let fixture_event = &fixture_events[index];
            let analytical_event = &analytical_events[index];
            assert_eq!(fixture_event.kind(), reference.kind);
            assert_eq!(analytical_event.kind(), reference.kind);
            let fixture_seconds =
                (fixture_event.interval().midpoint().day() - reference.tt_day).abs() * 86_400.0;
            let analytical_seconds =
                (analytical_event.interval().midpoint().day() - reference.tt_day).abs() * 86_400.0;
            eprintln!(
                "{} {:?}: fixture/direct {:.3} s; analytical/direct {:.3} s",
                case, reference.kind, fixture_seconds, analytical_seconds
            );
            assert!(fixture_seconds <= 2.0);
            assert!(analytical_seconds <= 2.0);
            assert_eq!(fixture_event.provider_model(), HORIZONS_FIXTURE_MODEL);
            assert_eq!(
                fixture_event.provider_snapshot(),
                Some(HORIZONS_FIXTURE_SNAPSHOT)
            );
            assert_eq!(
                fixture_event.earth_orientation_authority(),
                HORIZONS_EOP_AUTHORITY
            );
            assert_eq!(
                fixture_event.earth_orientation_snapshot(),
                HORIZONS_EOP_SNAPSHOT
            );
        }
    }
}

#[test]
fn altitude_search_rejects_coarse_steps_and_nonphysical_thresholds() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_460_000.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let coarse = SearchWindow::new(start, end, 2.0 / 24.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeCrossingSearch::new(coarse, Angle::from_degrees(0.0).unwrap()).unwrap_err(),
        AltitudeCrossingSearchError::StepTooLarge
    );

    let hourly = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeCrossingSearch::new(hourly, Angle::from_degrees(90.1).unwrap()).unwrap_err(),
        AltitudeCrossingSearchError::ThresholdOutOfRange
    );
}

#[test]
fn altitude_search_preserves_each_error_boundary() {
    let utc = ScaleAwareEpoch::from_gregorian_utc(2024, 4, 8, 0, 0, 0, 0);
    let start = JulianDate::<TerrestrialTime>::from_epoch(utc);
    let end = start.offset_days(1.0 / 24.0).unwrap();
    let window = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    let search = AltitudeCrossingSearch::new(window, Angle::from_degrees(0.0).unwrap()).unwrap();
    let site = observer(0.0, 0.0, 0.0);
    let orientation = constant_orientation(start, utc, 0.0);

    match airless_altitude_crossings(
        &FailingPosition,
        &orientation,
        site,
        ApparentBody::Sun,
        search,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::Position { source, .. } => {
            assert_eq!(source, TestError("position"))
        }
        other => panic!("unexpected position error: {:?}", other),
    }

    match airless_altitude_crossings(
        &AnalyticalEphemeris,
        &FailingOrientation,
        site,
        ApparentBody::Sun,
        search,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::EarthOrientation { source, .. } => {
            assert_eq!(source, TestError("orientation"))
        }
        other => panic!("unexpected Earth-orientation error: {:?}", other),
    }

    match airless_altitude_crossings(
        &AnalyticalEphemeris,
        &WrongIdentityOrientation {
            reference_epoch: start,
            reference_ut1: JulianDate::<UniversalTime1>::from_utc_epoch(
                utc,
                TimeOffset::from_seconds(0.0).unwrap(),
            ),
        },
        site,
        ApparentBody::Sun,
        search,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::EarthOrientationIdentityMismatch {
            expected_authority,
            actual_authority,
            ..
        } => {
            assert_eq!(expected_authority, "declared");
            assert_eq!(actual_authority, "returned");
        }
        other => panic!("unexpected identity error: {:?}", other),
    }

    match airless_altitude_crossings(
        &WrongEpochPosition,
        &orientation,
        site,
        ApparentBody::Sun,
        search,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::Transform { .. } => {}
        other => panic!("unexpected transform error: {:?}", other),
    }
}

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
            assert_eq!(fields.len(), 9, "altitude vector column count");
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
            let dut1: f64 = fields[8].parse().expect("DUT1 seconds");
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

fn direct_altitude_crossings(rows: &[&HorizonsAltitudeRow]) -> Vec<DirectAltitudeCrossing> {
    let mut crossings = Vec::new();
    for pair in rows.windows(2) {
        let left = pair[0];
        let right = pair[1];
        if left.direct_altitude.signum() == right.direct_altitude.signum() {
            continue;
        }
        let fraction = -left.direct_altitude / (right.direct_altitude - left.direct_altitude);
        crossings.push(DirectAltitudeCrossing {
            kind: if right.direct_altitude > 0.0 {
                AltitudeCrossingKind::Ascending
            } else {
                AltitudeCrossingKind::Descending
            },
            tt_day: left.tt_day + fraction * (right.tt_day - left.tt_day),
        });
    }
    crossings
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
