// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use std::fmt;

use turquet::apparent::{ApparentBody, ANALYTICAL_APPARENT};
use turquet::events::{
    lunar_illumination_at, LunarIlluminationError, GEOCENTRIC_LUNAR_ILLUMINATION,
};
use turquet::foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, ScaleAwareEpoch, State,
    TerrestrialTime, TrueEclipticEquinoxOfDate,
};
use turquet::provider::{AnalyticalEphemeris, GeocentricPositionProvider};

const HORIZONS_VECTORS: &str = include_str!("vectors/lunar_illumination_horizons.tsv");
const HORIZONS_FIXTURE: Model = Model::new("NASA/JPL Horizons DE441 fixture", "2026-08-26");

#[test]
fn analytical_lunar_illumination_has_expected_phase_order() {
    let cases = [
        ("new", tt_from_utc(2024, 4, 8, 18, 21, 0.0), 0.0, 0.01),
        (
            "first quarter",
            tt_from_utc(2024, 4, 15, 19, 13, 0.0),
            0.45,
            0.55,
        ),
        ("full", tt_from_utc(2024, 4, 23, 23, 49, 0.0), 0.99, 1.0),
    ];

    for &(name, epoch, minimum, maximum) in &cases {
        let illumination = lunar_illumination_at(&AnalyticalEphemeris, epoch)
            .expect("analytical illumination succeeds");
        eprintln!(
            "{}: fraction {:.8}; elongation {:.6} deg; phase angle {:.6} deg",
            name,
            illumination.illuminated_fraction(),
            illumination.elongation().degrees(),
            illumination.phase_angle().degrees(),
        );
        assert!(
            illumination.illuminated_fraction() >= minimum
                && illumination.illuminated_fraction() <= maximum,
            "{} fraction {} was outside [{}, {}]",
            name,
            illumination.illuminated_fraction(),
            minimum,
            maximum,
        );
        assert_eq!(illumination.epoch(), epoch);
        assert_eq!(
            illumination.illumination_model(),
            GEOCENTRIC_LUNAR_ILLUMINATION
        );
        assert_eq!(illumination.provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(illumination.provider_snapshot(), None);
        assert!(illumination.moon_distance().meters() > 0.0);
        assert!(illumination.sun_distance().meters() > 0.0);
        assert!(illumination.moon_sun_distance().meters() > 0.0);
        assert!(illumination.elongation().radians() >= 0.0);
        assert!(illumination.phase_angle().radians() >= 0.0);
    }
}

#[test]
fn lunar_illumination_retains_provider_provenance() {
    let epoch = tt_from_utc(2024, 4, 23, 23, 49, 0.0);
    let provider = ControlledProvider::new(
        PositionResponse::State(state(epoch, 180.0, 2.0)),
        PositionResponse::State(state(epoch, 0.0, 10.0)),
    );
    let illumination = lunar_illumination_at(&provider, epoch).expect("controlled full moon");

    assert_eq!(illumination.illuminated_fraction(), 1.0);
    assert_eq!(illumination.elongation().degrees(), 180.0);
    assert!(illumination.phase_angle().degrees().abs() < 1e-12);
    assert_eq!(
        illumination.provider_model(),
        Model::new("controlled fixture", "1")
    );
    assert_eq!(
        illumination.provider_snapshot(),
        Some("controlled snapshot")
    );
}

#[test]
fn lunar_illumination_preserves_position_and_geometry_errors() {
    let epoch = tt_from_utc(2024, 4, 8, 18, 21, 0.0);
    let normal_moon = PositionResponse::State(state(epoch, 0.0, 1.0));
    let normal_sun = PositionResponse::State(state(epoch, 180.0, 10.0));

    let moon_error = lunar_illumination_at(
        &ControlledProvider::new(PositionResponse::Error, normal_sun),
        epoch,
    )
    .unwrap_err();
    assert_position_error(moon_error, ApparentBody::Moon, epoch);

    let sun_error = lunar_illumination_at(
        &ControlledProvider::new(normal_moon, PositionResponse::Error),
        epoch,
    )
    .unwrap_err();
    assert_position_error(sun_error, ApparentBody::Sun, epoch);

    let wrong_epoch = epoch.offset_days(1.0).unwrap();
    let mismatch = lunar_illumination_at(
        &ControlledProvider::new(
            PositionResponse::State(state(wrong_epoch, 0.0, 1.0)),
            normal_sun,
        ),
        epoch,
    )
    .unwrap_err();
    match mismatch {
        LunarIlluminationError::StateEpochMismatch {
            body,
            expected_epoch,
            actual_epoch,
        } => {
            assert_eq!(body, ApparentBody::Moon);
            assert_eq!(expected_epoch, epoch);
            assert_eq!(actual_epoch, wrong_epoch);
        }
        _ => panic!("expected returned-state epoch mismatch"),
    }

    let zero_distance = lunar_illumination_at(
        &ControlledProvider::new(PositionResponse::State(state(epoch, 0.0, 0.0)), normal_sun),
        epoch,
    )
    .unwrap_err();
    match zero_distance {
        LunarIlluminationError::BodyAtObserver {
            body,
            epoch: actual,
            ..
        } => {
            assert_eq!(body, ApparentBody::Moon);
            assert_eq!(actual, epoch);
        }
        _ => panic!("expected zero-distance Moon error"),
    }

    let coincident = lunar_illumination_at(
        &ControlledProvider::new(
            PositionResponse::State(state(epoch, 0.0, 1.0)),
            PositionResponse::State(state(epoch, 0.0, 1.0)),
        ),
        epoch,
    )
    .unwrap_err();
    match coincident {
        LunarIlluminationError::CoincidentSunAndMoon { epoch: actual } => {
            assert_eq!(actual, epoch)
        }
        _ => panic!("expected coincident Sun-Moon error"),
    }

    let nonfinite = lunar_illumination_at(
        &ControlledProvider::new(
            PositionResponse::State(state_meters(epoch, 0.0, f64::MAX)),
            PositionResponse::State(state_meters(epoch, 180.0, f64::MAX)),
        ),
        epoch,
    )
    .unwrap_err();
    match nonfinite {
        LunarIlluminationError::NonFiniteTriangle { epoch: actual } => assert_eq!(actual, epoch),
        _ => panic!("expected non-finite triangle error"),
    }
}

#[test]
fn horizons_fixture_and_analytical_provider_match_reported_lunar_illumination() {
    let rows = parse_horizons_vectors();
    let cases = [
        ("new", tt_from_utc(2024, 4, 8, 18, 21, 0.0)),
        ("first-quarter", tt_from_utc(2024, 4, 15, 19, 13, 0.0)),
        ("full", tt_from_utc(2024, 4, 23, 23, 49, 0.0)),
    ];
    let offsets_hours = [-12.0, -6.0, 0.0, 6.0, 12.0];

    for &(case, phase_epoch) in &cases {
        for (sample_index, offset_hours) in offsets_hours.iter().enumerate() {
            let epoch = phase_epoch.offset_days(*offset_hours / 24.0).unwrap();
            let moon = fixture_row(&rows, case, "moon", sample_index);
            let sun = fixture_row(&rows, case, "sun", sample_index);
            let reference_fraction = moon.illumination_percent.unwrap() / 100.0;
            let fixture = LunarHorizonsProvider::new(
                epoch,
                fixture_state(epoch, moon),
                fixture_state(epoch, sun),
            );
            let fixture_result = lunar_illumination_at(&fixture, epoch)
                .expect("Horizons fixture illumination succeeds");
            let analytical_result = lunar_illumination_at(&AnalyticalEphemeris, epoch)
                .expect("analytical illumination succeeds");

            eprintln!(
                "{} {:+.0}h: Horizons {:.8}; fixture {:.8}; analytical {:.8}; phase {:.6}/{:.6}/{:.6} deg",
                case,
                offset_hours,
                reference_fraction,
                fixture_result.illuminated_fraction(),
                analytical_result.illuminated_fraction(),
                moon.phase_angle_degrees.unwrap(),
                fixture_result.phase_angle().degrees(),
                analytical_result.phase_angle().degrees(),
            );
            assert!(
                (fixture_result.illuminated_fraction() - reference_fraction).abs() < 0.000_01,
                "fixture fraction differs from Horizons for {} sample {}",
                case,
                sample_index,
            );
            assert!(
                (fixture_result.phase_angle().degrees() - moon.phase_angle_degrees.unwrap()).abs()
                    < 0.001,
                "fixture phase angle differs from Horizons for {} sample {}",
                case,
                sample_index,
            );
            assert!(
                (fixture_result.elongation().degrees() - moon.solar_elongation_degrees.unwrap())
                    .abs()
                    < 0.001,
                "fixture elongation differs from Horizons for {} sample {}",
                case,
                sample_index,
            );
            assert!(
                (analytical_result.illuminated_fraction() - reference_fraction).abs() < 0.000_015,
                "analytical fraction differs from Horizons for {} sample {}",
                case,
                sample_index,
            );
            assert_eq!(fixture_result.provider_model(), HORIZONS_FIXTURE);
            assert_eq!(
                fixture_result.provider_snapshot(),
                Some("Horizons API 1.2 / DE441 / generated 2026-08-26")
            );
        }
    }
}

fn assert_position_error(
    error: LunarIlluminationError<ControlledError>,
    expected_body: ApparentBody,
    expected_epoch: JulianDate<TerrestrialTime>,
) {
    match error {
        LunarIlluminationError::Position {
            body,
            epoch,
            source,
        } => {
            assert_eq!(body, expected_body);
            assert_eq!(epoch, expected_epoch);
            assert_eq!(source, ControlledError::RequestedFailure);
        }
        _ => panic!("expected provider position error"),
    }
}

#[derive(Clone, Copy)]
enum PositionResponse {
    State(State<TrueEclipticEquinoxOfDate>),
    Error,
}

struct ControlledProvider {
    moon: PositionResponse,
    sun: PositionResponse,
}

impl ControlledProvider {
    fn new(moon: PositionResponse, sun: PositionResponse) -> Self {
        Self { moon, sun }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlledError {
    RequestedFailure,
}

impl fmt::Display for ControlledError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("controlled provider failure")
    }
}

impl std::error::Error for ControlledError {}

impl GeocentricPositionProvider for ControlledProvider {
    type Error = ControlledError;

    fn model(&self) -> Model {
        Model::new("controlled fixture", "1")
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some("controlled snapshot")
    }

    fn position(
        &self,
        body: ApparentBody,
        _epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let response = match body {
            ApparentBody::Moon => self.moon,
            ApparentBody::Sun => self.sun,
            _ => panic!("unexpected controlled body"),
        };
        match response {
            PositionResponse::State(state) => Ok(state),
            PositionResponse::Error => Err(ControlledError::RequestedFailure),
        }
    }
}

struct LunarHorizonsProvider {
    epoch: JulianDate<TerrestrialTime>,
    moon: State<TrueEclipticEquinoxOfDate>,
    sun: State<TrueEclipticEquinoxOfDate>,
}

impl LunarHorizonsProvider {
    fn new(
        epoch: JulianDate<TerrestrialTime>,
        moon: State<TrueEclipticEquinoxOfDate>,
        sun: State<TrueEclipticEquinoxOfDate>,
    ) -> Self {
        Self { epoch, moon, sun }
    }
}

impl GeocentricPositionProvider for LunarHorizonsProvider {
    type Error = ();

    fn model(&self) -> Model {
        HORIZONS_FIXTURE
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some("Horizons API 1.2 / DE441 / generated 2026-08-26")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        assert_eq!(
            epoch, self.epoch,
            "fixture request must retain its TT epoch"
        );
        match body {
            ApparentBody::Moon => Ok(self.moon),
            ApparentBody::Sun => Ok(self.sun),
            _ => panic!("unexpected Horizons fixture body"),
        }
    }
}

#[derive(Clone, Copy)]
struct HorizonsRow<'a> {
    case: &'a str,
    body: &'a str,
    illumination_percent: Option<f64>,
    range_astronomical_units: f64,
    phase_angle_degrees: Option<f64>,
    solar_elongation_degrees: Option<f64>,
    ecliptic_longitude_degrees: f64,
    ecliptic_latitude_degrees: f64,
}

fn parse_horizons_vectors() -> Vec<HorizonsRow<'static>> {
    HORIZONS_VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 11, "expected eleven Horizons fields");
            HorizonsRow {
                case: fields[0],
                body: fields[1],
                illumination_percent: optional_number(fields[5]),
                range_astronomical_units: fields[6].parse().expect("Horizons range AU"),
                phase_angle_degrees: optional_number(fields[7]),
                solar_elongation_degrees: optional_number(fields[8]),
                ecliptic_longitude_degrees: fields[9].parse().expect("Horizons ecliptic longitude"),
                ecliptic_latitude_degrees: fields[10].parse().expect("Horizons ecliptic latitude"),
            }
        })
        .collect()
}

fn optional_number(value: &str) -> Option<f64> {
    if value.is_empty() {
        None
    } else {
        Some(value.parse().expect("Horizons numeric field"))
    }
}

fn fixture_row<'a>(
    rows: &'a [HorizonsRow<'static>],
    case: &str,
    body: &str,
    sample_index: usize,
) -> &'a HorizonsRow<'static> {
    rows.iter()
        .filter(|row| row.case == case && row.body == body)
        .nth(sample_index)
        .expect("committed Horizons fixture row")
}

fn fixture_state(
    epoch: JulianDate<TerrestrialTime>,
    row: &HorizonsRow<'static>,
) -> State<TrueEclipticEquinoxOfDate> {
    State::new(
        epoch,
        Direction::new(
            Longitude::from_degrees(row.ecliptic_longitude_degrees)
                .expect("finite Horizons ecliptic longitude"),
            Latitude::from_degrees(row.ecliptic_latitude_degrees)
                .expect("finite Horizons ecliptic latitude"),
        ),
        Distance::from_astronomical_units(row.range_astronomical_units)
            .expect("positive Horizons range"),
    )
}

fn state(
    epoch: JulianDate<TerrestrialTime>,
    longitude_degrees: f64,
    distance_astronomical_units: f64,
) -> State<TrueEclipticEquinoxOfDate> {
    State::new(
        epoch,
        Direction::new(
            Longitude::from_degrees(longitude_degrees).unwrap(),
            Latitude::from_degrees(0.0).unwrap(),
        ),
        Distance::from_astronomical_units(distance_astronomical_units).unwrap(),
    )
}

fn state_meters(
    epoch: JulianDate<TerrestrialTime>,
    longitude_degrees: f64,
    distance_meters: f64,
) -> State<TrueEclipticEquinoxOfDate> {
    State::new(
        epoch,
        Direction::new(
            Longitude::from_degrees(longitude_degrees).unwrap(),
            Latitude::from_degrees(0.0).unwrap(),
        ),
        Distance::from_meters(distance_meters).unwrap(),
    )
}

fn tt_from_utc(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
) -> JulianDate<TerrestrialTime> {
    let whole = second.trunc() as u8;
    let nanos = ((second - f64::from(whole)) * 1e9).round() as u32;
    JulianDate::from_epoch(ScaleAwareEpoch::from_gregorian_utc(
        year, month, day, hour, minute, whole, nanos,
    ))
}
