// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use std::fmt;

use turquet::apparent::{ApparentBody, ANALYTICAL_APPARENT};
use turquet::events::{
    geocentric_distance_extrema, GeocentricDistanceExtremumError, GeocentricDistanceExtremumKind,
    GeocentricDistanceExtremumSearch, GeocentricDistanceExtremumSearchError, SearchWindow,
    GEOCENTRIC_APPARENT_DISTANCE_EXTREMA, MAX_GEOCENTRIC_DISTANCE_EXTREMUM_DERIVATIVE_SPAN_DAYS,
};
use turquet::foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, State, TerrestrialTime,
    TrueEclipticEquinoxOfDate,
};
use turquet::provider::{AnalyticalEphemeris, GeocentricPositionProvider};

#[path = "support/distance_extrema_fixture.rs"]
mod fixture;

#[test]
fn synthetic_off_grid_extrema_are_ordered_and_provenanced() {
    let origin = JulianDate::from_julian_day(2_460_000.0).unwrap();
    let provider = PeriodicRangeProvider {
        origin,
        shift_days: 0.1,
    };
    let events = geocentric_distance_extrema(
        &provider,
        ApparentBody::Moon,
        distance_search(
            origin.offset_days(-1.5).unwrap(),
            origin.offset_days(1.5).unwrap(),
        ),
    )
    .expect("periodic extrema");

    assert_eq!(events.len(), 3);
    let expected = [
        (GeocentricDistanceExtremumKind::Minimum, -0.9),
        (GeocentricDistanceExtremumKind::Maximum, 0.1),
        (GeocentricDistanceExtremumKind::Minimum, 1.1),
    ];
    for (event, &(kind, offset_days)) in events.iter().zip(&expected) {
        let expected_epoch = origin.offset_days(offset_days).unwrap();
        assert_eq!(event.body(), ApparentBody::Moon);
        assert_eq!(event.kind(), kind);
        assert!(event.interval().width_days() <= 1.0 / 86_400.0);
        assert!(seconds_apart(event.interval().midpoint(), expected_epoch) <= 1.0);
        assert!(event.midpoint_distance().meters() > 0.0);
        assert_eq!(event.derivative_span_days(), 0.1);
        assert_eq!(event.extrema_model(), GEOCENTRIC_APPARENT_DISTANCE_EXTREMA);
        assert_eq!(
            event.provider_model(),
            Model::new("periodic range fixture", "1")
        );
        assert_eq!(event.provider_snapshot(), Some("off-grid extrema"));
    }
}

#[test]
fn exact_samples_are_retained_but_boundaries_and_plateaus_are_omitted() {
    let origin = JulianDate::from_julian_day(2_460_000.0).unwrap();
    let exact = QuadraticRangeProvider { origin };
    let exact_events = geocentric_distance_extrema(
        &exact,
        ApparentBody::Moon,
        distance_search(
            origin.offset_days(-1.25).unwrap(),
            origin.offset_days(1.25).unwrap(),
        ),
    )
    .expect("exact extrema");
    assert_eq!(exact_events.len(), 1);
    assert_eq!(
        exact_events[0].kind(),
        GeocentricDistanceExtremumKind::Minimum
    );
    assert_eq!(exact_events[0].interval().start(), origin);
    assert_eq!(exact_events[0].interval().end(), origin);

    let boundary_events = geocentric_distance_extrema(
        &exact,
        ApparentBody::Moon,
        distance_search(origin, origin.offset_days(0.75).unwrap()),
    )
    .expect("boundary extrema");
    assert!(boundary_events.is_empty());

    let plateau = ConstantProvider { origin };
    let plateau_events = geocentric_distance_extrema(
        &plateau,
        ApparentBody::Moon,
        distance_search(
            origin.offset_days(-1.0).unwrap(),
            origin.offset_days(1.0).unwrap(),
        ),
    )
    .expect("constant range");
    assert!(plateau_events.is_empty());

    let interior_plateau = FlatMinimumProvider { origin };
    let interior_window = SearchWindow::new(
        origin.offset_days(-0.5).unwrap(),
        origin.offset_days(0.5).unwrap(),
        1.0,
        1.0 / 86_400.0,
    )
    .unwrap();
    let interior_search = GeocentricDistanceExtremumSearch::new(interior_window, 0.1).unwrap();
    let interior_events =
        geocentric_distance_extrema(&interior_plateau, ApparentBody::Moon, interior_search)
            .expect("interior plateau");
    assert!(interior_events.is_empty());
}

#[test]
fn distance_extremum_search_validates_its_derivative_span() {
    let start = JulianDate::from_julian_day(2_460_000.0).unwrap();
    let window = SearchWindow::new(start, start.offset_days(1.0).unwrap(), 0.25, 0.01).unwrap();
    assert_eq!(
        GeocentricDistanceExtremumSearch::new(window, f64::NAN).unwrap_err(),
        GeocentricDistanceExtremumSearchError::DerivativeSpanNotFinite
    );
    assert_eq!(
        GeocentricDistanceExtremumSearch::new(window, 0.0).unwrap_err(),
        GeocentricDistanceExtremumSearchError::DerivativeSpanNotPositive
    );
    assert_eq!(
        GeocentricDistanceExtremumSearch::new(
            window,
            MAX_GEOCENTRIC_DISTANCE_EXTREMUM_DERIVATIVE_SPAN_DAYS + f64::EPSILON,
        )
        .unwrap_err(),
        GeocentricDistanceExtremumSearchError::DerivativeSpanTooLarge
    );
    let search = GeocentricDistanceExtremumSearch::new(window, 0.1).unwrap();
    assert_eq!(search.window(), window);
    assert_eq!(search.derivative_span_days(), 0.1);
}

#[test]
fn distance_extrema_preserve_provider_errors_and_returned_epochs() {
    let origin = JulianDate::from_julian_day(2_460_000.0).unwrap();
    let search = distance_search(origin, origin.offset_days(1.0).unwrap());
    let position_error = geocentric_distance_extrema(
        &FaultProvider {
            origin,
            response: FaultResponse::Error,
        },
        ApparentBody::Mars,
        search,
    )
    .unwrap_err();
    match position_error {
        GeocentricDistanceExtremumError::Position {
            body,
            epoch,
            source,
        } => {
            assert_eq!(body, ApparentBody::Mars);
            assert_eq!(epoch, origin.offset_days(0.05).unwrap());
            assert_eq!(source, FaultError::RequestedFailure);
        }
        _ => panic!("expected provider position failure"),
    }

    let mismatch = geocentric_distance_extrema(
        &FaultProvider {
            origin,
            response: FaultResponse::WrongEpoch,
        },
        ApparentBody::Mars,
        search,
    )
    .unwrap_err();
    match mismatch {
        GeocentricDistanceExtremumError::StateEpochMismatch {
            body,
            expected_epoch,
            actual_epoch,
        } => {
            assert_eq!(body, ApparentBody::Mars);
            assert_eq!(expected_epoch, origin.offset_days(0.05).unwrap());
            assert_eq!(actual_epoch, origin.offset_days(1.05).unwrap());
        }
        _ => panic!("expected returned-state epoch mismatch"),
    }
}

#[test]
fn horizons_fixture_and_analytical_provider_find_captured_distance_extrema() {
    for &(name, maximum, expected_rows) in &[
        ("moon_perigee", false, 11),
        ("moon_apogee", true, 11),
        ("mars_close", false, 55),
    ] {
        let case = fixture::horizons_case(name);
        assert_eq!(case.rows.len(), expected_rows, "{} fixture row count", name);
        let search = fixture_search(&case);
        let (reference_epoch, reference_range) = fixture::parabolic_reference(&case, maximum);
        let fixture_provider = case.provider();
        let fixture_events = geocentric_distance_extrema(&fixture_provider, case.body, search)
            .expect("Horizons fixture extrema");
        let analytical_events =
            geocentric_distance_extrema(&AnalyticalEphemeris, case.body, search)
                .expect("analytical extrema");

        assert_eq!(fixture_events.len(), 1, "{} fixture extrema", name);
        assert_eq!(analytical_events.len(), 1, "{} analytical extrema", name);
        let expected_kind = if maximum {
            GeocentricDistanceExtremumKind::Maximum
        } else {
            GeocentricDistanceExtremumKind::Minimum
        };
        for (lane, event) in [
            ("fixture", &fixture_events[0]),
            ("analytical", &analytical_events[0]),
        ] {
            let seconds = seconds_apart(event.interval().midpoint(), reference_epoch);
            let range_kilometers =
                (event.midpoint_distance().meters() - reference_range.meters()).abs() / 1_000.0;
            eprintln!(
                "{} {}: {:.3} s from three-point Horizons reference; {:.3} km range",
                name, lane, seconds, range_kilometers
            );
            assert_eq!(event.body(), case.body);
            assert_eq!(event.kind(), expected_kind);
            assert!(event.interval().width_days() <= 1.0 / 86_400.0);
        }
        assert!(
            seconds_apart(fixture_events[0].interval().midpoint(), reference_epoch)
                <= 2.0 * 3_600.0,
            "{} fixture time residual",
            name
        );
        assert!(
            seconds_apart(analytical_events[0].interval().midpoint(), reference_epoch) <= 600.0,
            "{} analytical time residual",
            name
        );
        assert!(
            (fixture_events[0].midpoint_distance().meters() - reference_range.meters()).abs()
                <= 200_000.0,
            "{} fixture range residual",
            name
        );
        assert!(
            (analytical_events[0].midpoint_distance().meters() - reference_range.meters()).abs()
                <= 50_000.0,
            "{} analytical range residual",
            name
        );
        assert_eq!(
            fixture_events[0].provider_model(),
            fixture::HORIZONS_FIXTURE
        );
        assert_eq!(
            fixture_events[0].provider_snapshot(),
            Some("Horizons API 1.2 / DE441 / generated 2026-08-26")
        );
        assert_eq!(analytical_events[0].provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(analytical_events[0].provider_snapshot(), None);
    }
}

fn distance_search(
    start: JulianDate<TerrestrialTime>,
    end: JulianDate<TerrestrialTime>,
) -> GeocentricDistanceExtremumSearch {
    let window = SearchWindow::new(start, end, 0.25, 1.0 / 86_400.0).unwrap();
    GeocentricDistanceExtremumSearch::new(window, 0.1).unwrap()
}

fn fixture_search(case: &fixture::HorizonsRangeCase) -> GeocentricDistanceExtremumSearch {
    let start = case.rows[1].epoch;
    let end = case.rows[case.rows.len() - 2].epoch;
    distance_search(start, end)
}

struct PeriodicRangeProvider {
    origin: JulianDate<TerrestrialTime>,
    shift_days: f64,
}

impl GeocentricPositionProvider for PeriodicRangeProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("periodic range fixture", "1")
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some("off-grid extrema")
    }

    fn position(
        &self,
        _body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let offset = epoch.day() - self.origin.day() - self.shift_days;
        let range_kilometers = 400_000.0 + 1_000.0 * (std::f64::consts::PI * offset).cos();
        Ok(range_state(epoch, range_kilometers))
    }
}

struct QuadraticRangeProvider {
    origin: JulianDate<TerrestrialTime>,
}

impl GeocentricPositionProvider for QuadraticRangeProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("quadratic range fixture", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let offset = epoch.day() - self.origin.day();
        Ok(range_state(epoch, 400_000.0 + 1_000.0 * offset * offset))
    }
}

struct ConstantProvider {
    origin: JulianDate<TerrestrialTime>,
}

struct FlatMinimumProvider {
    origin: JulianDate<TerrestrialTime>,
}

impl GeocentricPositionProvider for FlatMinimumProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("flat-minimum range fixture", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let offset = (epoch.day() - self.origin.day()).abs();
        let excess = if offset > 0.4 { offset - 0.4 } else { 0.0 };
        Ok(range_state(epoch, 400_000.0 + 1_000.0 * excess))
    }
}

impl GeocentricPositionProvider for ConstantProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("constant range fixture", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let _ = self.origin;
        Ok(range_state(epoch, 400_000.0))
    }
}

#[derive(Clone, Copy)]
enum FaultResponse {
    Error,
    WrongEpoch,
}

struct FaultProvider {
    origin: JulianDate<TerrestrialTime>,
    response: FaultResponse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaultError {
    RequestedFailure,
}

impl fmt::Display for FaultError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("controlled provider failure")
    }
}

impl std::error::Error for FaultError {}

impl GeocentricPositionProvider for FaultProvider {
    type Error = FaultError;

    fn model(&self) -> Model {
        Model::new("fault fixture", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        match self.response {
            FaultResponse::Error => Err(FaultError::RequestedFailure),
            FaultResponse::WrongEpoch => Ok(range_state(
                epoch
                    .offset_days(self.origin.day() - self.origin.day() + 1.0)
                    .unwrap(),
                400_000.0,
            )),
        }
    }
}

fn range_state(
    epoch: JulianDate<TerrestrialTime>,
    range_kilometers: f64,
) -> State<TrueEclipticEquinoxOfDate> {
    State::new(
        epoch,
        Direction::new(
            Longitude::from_degrees(0.0).unwrap(),
            Latitude::from_degrees(0.0).unwrap(),
        ),
        Distance::from_kilometers(range_kilometers).unwrap(),
    )
}

fn seconds_apart(first: JulianDate<TerrestrialTime>, second: JulianDate<TerrestrialTime>) -> f64 {
    (first.day() - second.day()).abs() * 86_400.0
}
