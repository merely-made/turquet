// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use turquet::apparent::{ApparentBody, ANALYTICAL_APPARENT};
use turquet::events::{
    ecliptic_longitude_conjunctions, EventError, SearchWindow, SearchWindowError,
    MAX_CONJUNCTION_STEP_DAYS,
};
use turquet::foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, ScaleAwareEpoch, State,
    TerrestrialTime, TrueEclipticEquinoxOfDate,
};
use turquet::provider::{AnalyticalEphemeris, GeocentricPositionProvider};

const VECTORS: &str = include_str!("vectors/eclipse_conjunction_horizons.tsv");
const HORIZONS_FIXTURE: Model = Model::new("NASA/JPL Horizons DE441 fixture", "2026-08-23");

#[test]
fn search_window_rejects_unsafe_controls() {
    let start = tt_from_utc(2024, 4, 8, 18, 0, 0.0);
    let end = tt_from_utc(2024, 4, 8, 19, 0, 0.0);
    assert_eq!(
        SearchWindow::new(end, start, 0.25, 1.0 / 86_400.0),
        Err(SearchWindowError::NonIncreasingInterval)
    );
    assert_eq!(
        SearchWindow::new(start, end, MAX_CONJUNCTION_STEP_DAYS + 0.1, 1.0 / 86_400.0),
        Err(SearchWindowError::StepTooLarge)
    );
    assert_eq!(
        SearchWindow::new(start, end, 0.25, 0.5),
        Err(SearchWindowError::ToleranceExceedsStep)
    );
}

#[test]
fn eclipse_conjunction_matches_horizons_and_nasa_interval() {
    let start = tt_from_utc(2024, 4, 8, 18, 10, 0.0);
    let end = tt_from_utc(2024, 4, 8, 18, 30, 0.0);
    let reference = tt_from_utc(2024, 4, 8, 18, 20, 46.8);
    let window = SearchWindow::new(start, end, 10.0 / 1_440.0, 1.0 / 86_400.0)
        .expect("valid eclipse search window");

    let fixture = HorizonsFixtureProvider::from_vectors();
    let fixture_events =
        ecliptic_longitude_conjunctions(&fixture, ApparentBody::Sun, ApparentBody::Moon, window)
            .expect("fixture search succeeds");
    let analytical_events = ecliptic_longitude_conjunctions(
        &AnalyticalEphemeris,
        ApparentBody::Sun,
        ApparentBody::Moon,
        window,
    )
    .expect("analytical search succeeds");

    assert_eq!(fixture_events.len(), 1);
    assert_eq!(analytical_events.len(), 1);
    let fixture_event = &fixture_events[0];
    let analytical_event = &analytical_events[0];
    eprintln!(
        "eclipse conjunction: Horizons fixture {:+.3}s from NASA; analytical {:+.3}s; provider delta {:.3}s; separations {:.6}/{:.6} deg",
        signed_seconds(fixture_event.interval().midpoint(), reference),
        signed_seconds(analytical_event.interval().midpoint(), reference),
        seconds_apart(
            fixture_event.interval().midpoint(),
            analytical_event.interval().midpoint(),
        ),
        fixture_event.angular_separation().degrees(),
        analytical_event.angular_separation().degrees(),
    );
    assert!(fixture_event.interval().width_days() * 86_400.0 <= 1.001);
    assert!(analytical_event.interval().width_days() * 86_400.0 <= 1.001);
    assert!(seconds_apart(fixture_event.interval().midpoint(), reference) < 10.0);
    assert!(seconds_apart(analytical_event.interval().midpoint(), reference) < 120.0);
    assert!(
        seconds_apart(
            fixture_event.interval().midpoint(),
            analytical_event.interval().midpoint(),
        ) < 120.0
    );
    assert_eq!(fixture_event.provider_model(), HORIZONS_FIXTURE);
    assert_eq!(analytical_event.provider_model(), ANALYTICAL_APPARENT);
    assert_eq!(
        fixture_event.provider_snapshot(),
        Some("Horizons API 1.2 / DE441 / generated 2026-08-23")
    );
    assert_eq!(analytical_event.provider_snapshot(), None);
    assert!(fixture_event.angular_separation().degrees() > 0.34);
    assert!(fixture_event.angular_separation().degrees() < 0.36);
}

#[test]
fn opposition_wrap_is_not_reported_as_a_conjunction() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let window = SearchWindow::new(start, end, 0.25, 1.0 / 86_400.0).unwrap();
    let provider = LinearProvider {
        origin: start,
        initial_degrees: 170.0,
        degrees_per_day: 20.0,
    };
    let events =
        ecliptic_longitude_conjunctions(&provider, ApparentBody::Moon, ApparentBody::Sun, window)
            .unwrap();
    assert!(events.is_empty());
}

#[test]
fn provider_failures_are_not_reported_as_an_empty_search() {
    let fixture = HorizonsFixtureProvider::from_vectors();
    let start = tt_from_utc(2024, 4, 8, 18, 0, 0.0);
    let end = tt_from_utc(2024, 4, 8, 18, 5, 0.0);
    let window = SearchWindow::new(start, end, 5.0 / 1_440.0, 1.0 / 86_400.0).unwrap();
    assert!(match ecliptic_longitude_conjunctions(
        &fixture,
        ApparentBody::Sun,
        ApparentBody::Moon,
        window,
    ) {
        Err(EventError::Position {
            source: FixtureError::OutsideFixture,
            ..
        }) => true,
        _ => false,
    });
}

#[test]
fn longitude_wrap_is_refined_and_same_body_is_refused() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let window = SearchWindow::new(start, end, 0.25, 1.0 / 86_400.0).unwrap();
    let provider = LinearProvider {
        origin: start,
        initial_degrees: 350.0,
        degrees_per_day: 20.0,
    };
    let events =
        ecliptic_longitude_conjunctions(&provider, ApparentBody::Moon, ApparentBody::Sun, window)
            .unwrap();
    assert_eq!(events.len(), 1);
    assert!(
        seconds_apart(
            events[0].interval().midpoint(),
            start.offset_days(0.5).unwrap()
        ) < 1.0
    );
    assert!(match ecliptic_longitude_conjunctions(
        &provider,
        ApparentBody::Sun,
        ApparentBody::Sun,
        window,
    ) {
        Err(EventError::SameBody) => true,
        _ => false,
    });
}

#[derive(Clone, Copy)]
struct FixtureRow {
    body: ApparentBody,
    tt_day: f64,
    longitude_degrees: f64,
    latitude_degrees: f64,
    distance_au: f64,
}

struct HorizonsFixtureProvider {
    rows: Vec<FixtureRow>,
}

impl HorizonsFixtureProvider {
    fn from_vectors() -> Self {
        for expected in &[
            "oracle: NASA/JPL Horizons API 1.2, DE441",
            "reference: NASA GSFC 2024-04-08 ecliptic conjunction 18:20:46.8 UT",
            "fetch_horizons_conjunction_vectors.ps1",
        ] {
            assert!(
                VECTORS.lines().any(|line| line.contains(expected)),
                "conjunction fixture header must record {}",
                expected
            );
        }
        let mut rows = Vec::new();
        for line in VECTORS.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 5);
            let body = match fields[0] {
                "sun" => ApparentBody::Sun,
                "moon" => ApparentBody::Moon,
                _ => panic!("unknown fixture body {}", fields[0]),
            };
            let utc_day = fields[1].parse::<f64>().expect("UTC Julian day");
            let epoch = ScaleAwareEpoch::from_jde_utc(utc_day);
            rows.push(FixtureRow {
                body,
                tt_day: JulianDate::<TerrestrialTime>::from_epoch(epoch).day(),
                longitude_degrees: fields[2].parse().expect("longitude"),
                latitude_degrees: fields[3].parse().expect("latitude"),
                distance_au: fields[4].parse().expect("distance"),
            });
        }
        assert_eq!(rows.len(), 6);
        Self { rows }
    }
}

#[derive(Clone, Copy, Debug)]
enum FixtureError {
    UnsupportedBody,
    OutsideFixture,
}

impl GeocentricPositionProvider for HorizonsFixtureProvider {
    type Error = FixtureError;

    fn model(&self) -> Model {
        HORIZONS_FIXTURE
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some("Horizons API 1.2 / DE441 / generated 2026-08-23")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let rows: Vec<&FixtureRow> = self.rows.iter().filter(|row| row.body == body).collect();
        if rows.is_empty() {
            return Err(FixtureError::UnsupportedBody);
        }
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
                let longitude = left.longitude_degrees
                    + fraction * signed_degrees(right.longitude_degrees - left.longitude_degrees);
                let latitude = left.latitude_degrees
                    + fraction * (right.latitude_degrees - left.latitude_degrees);
                let distance = left.distance_au + fraction * (right.distance_au - left.distance_au);
                return Ok(State::new(
                    epoch,
                    Direction::new(
                        Longitude::from_degrees(longitude).expect("fixture longitude"),
                        Latitude::from_degrees(latitude).expect("fixture latitude"),
                    ),
                    Distance::from_astronomical_units(distance).expect("fixture distance"),
                ));
            }
        }
        Err(FixtureError::OutsideFixture)
    }
}

struct LinearProvider {
    origin: JulianDate<TerrestrialTime>,
    initial_degrees: f64,
    degrees_per_day: f64,
}

impl GeocentricPositionProvider for LinearProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("linear test provider", "1")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let longitude = if body == ApparentBody::Moon {
            self.initial_degrees + self.degrees_per_day * (epoch.day() - self.origin.day())
        } else {
            0.0
        };
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).unwrap(),
                Latitude::from_degrees(0.0).unwrap(),
            ),
            Distance::from_astronomical_units(1.0).unwrap(),
        ))
    }
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

fn seconds_apart(first: JulianDate<TerrestrialTime>, second: JulianDate<TerrestrialTime>) -> f64 {
    (first.day() - second.day()).abs() * 86_400.0
}

fn signed_seconds(first: JulianDate<TerrestrialTime>, second: JulianDate<TerrestrialTime>) -> f64 {
    (first.day() - second.day()) * 86_400.0
}

fn signed_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}
