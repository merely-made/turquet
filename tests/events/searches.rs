// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use super::*;

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
fn station_search_rejects_unsafe_velocity_spans() {
    let start = tt_from_utc(2024, 4, 25, 0, 0, 0.0);
    let end = tt_from_utc(2024, 4, 26, 0, 0, 0.0);
    let window = SearchWindow::new(start, end, 0.25, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        StationSearch::new(window, f64::NAN),
        Err(StationSearchError::VelocitySpanNotFinite)
    );
    assert_eq!(
        StationSearch::new(window, 0.0),
        Err(StationSearchError::VelocitySpanNotPositive)
    );
    assert_eq!(
        StationSearch::new(window, MAX_STATION_VELOCITY_SPAN_DAYS + 0.1),
        Err(StationSearchError::VelocitySpanTooLarge)
    );
}

#[test]
fn lunar_eclipse_search_rejects_unsafe_spans() {
    let start = tt_from_utc(2024, 3, 25, 6, 50, 0.0);
    let end = tt_from_utc(2024, 3, 25, 7, 10, 0.0);
    let window = SearchWindow::new(start, end, 10.0 / 1_440.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        LunarEclipseSearch::new(window, f64::NAN),
        Err(LunarEclipseSearchError::SpanNotFinite)
    );
    assert_eq!(
        LunarEclipseSearch::new(window, 0.0),
        Err(LunarEclipseSearchError::SpanNotPositive)
    );
    assert_eq!(
        LunarEclipseSearch::new(window, MAX_LUNAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS + 0.1),
        Err(LunarEclipseSearchError::SpanTooLarge)
    );
    assert_eq!(
        LunarEclipseSearch::new(window, 1.0 / 86_400.0),
        Err(LunarEclipseSearchError::ToleranceExceedsHalfSpan)
    );
}

#[test]
fn mercury_direct_station_matches_horizons_provider() {
    let start = tt_from_utc(2024, 4, 25, 0, 0, 0.0);
    let end = tt_from_utc(2024, 4, 26, 0, 0, 0.0);
    let window = SearchWindow::new(start, end, 3.0 / 24.0, 1.0 / 86_400.0)
        .expect("valid station search window");
    let search = StationSearch::new(window, 6.0 / 24.0).expect("valid six-hour velocity interval");
    let horizons_reference = tt_from_utc(2024, 4, 25, 12, 54, 10.1);

    let fixture = HorizonsFixtureProvider::mercury_station();
    let fixture_events = ecliptic_longitude_stations(&fixture, ApparentBody::Mercury, search)
        .expect("fixture station search succeeds");
    let analytical_events =
        ecliptic_longitude_stations(&AnalyticalEphemeris, ApparentBody::Mercury, search)
            .expect("analytical station search succeeds");

    assert_eq!(fixture_events.len(), 1);
    assert_eq!(analytical_events.len(), 1);
    let fixture_event = &fixture_events[0];
    let analytical_event = &analytical_events[0];
    eprintln!(
        "Mercury direct station: Horizons fixture TT JD {:.9}; analytical TT JD {:.9}; provider delta {:.3}s; longitudes {:.7}/{:.7} deg",
        fixture_event.interval().midpoint().day(),
        analytical_event.interval().midpoint().day(),
        seconds_apart(
            fixture_event.interval().midpoint(),
            analytical_event.interval().midpoint(),
        ),
        fixture_event.longitude().degrees(),
        analytical_event.longitude().degrees(),
    );
    for event in &[fixture_event, analytical_event] {
        assert_eq!(event.body(), ApparentBody::Mercury);
        assert_eq!(event.motion_before(), LongitudeMotion::Retrograde);
        assert_eq!(event.motion_after(), LongitudeMotion::Direct);
        assert_eq!(event.velocity_span_days(), 0.25);
        assert!(event.interval().width_days() * 86_400.0 <= 1.001);
    }
    assert!(
        seconds_apart(
            fixture_event.interval().midpoint(),
            analytical_event.interval().midpoint(),
        ) < 60.0
    );
    assert!(seconds_apart(fixture_event.interval().midpoint(), horizons_reference) < 2.0);
    assert!(seconds_apart(analytical_event.interval().midpoint(), horizons_reference) < 60.0);
    assert!(
        (fixture_event.longitude().degrees() - analytical_event.longitude().degrees()).abs()
            < 0.001
    );
    assert_eq!(fixture_event.provider_model(), HORIZONS_FIXTURE);
    assert_eq!(analytical_event.provider_model(), ANALYTICAL_APPARENT);
    assert_eq!(
        fixture_event.provider_snapshot(),
        Some("Horizons API 1.2 / DE441 / station fixture generated 2026-08-24")
    );
    assert_eq!(analytical_event.provider_snapshot(), None);
}

#[test]
fn april_lunar_phases_match_horizons_and_nasa() {
    let samples = [
        (LunarPhase::LastQuarter, (2024, 4, 2, 3, 15)),
        (LunarPhase::NewMoon, (2024, 4, 8, 18, 21)),
        (LunarPhase::FirstQuarter, (2024, 4, 15, 19, 13)),
        (LunarPhase::FullMoon, (2024, 4, 23, 23, 49)),
    ];
    let fixture = HorizonsFixtureProvider::lunar_phases();

    for &(phase, (year, month, day, hour, minute)) in &samples {
        let reference = tt_from_utc(year, month, day, hour, minute, 0.0);
        let start = reference.offset_days(-10.0 / 1_440.0).unwrap();
        let end = reference.offset_days(10.0 / 1_440.0).unwrap();
        let window = SearchWindow::new(start, end, 10.0 / 1_440.0, 1.0 / 86_400.0)
            .expect("valid lunar phase window");
        let fixture_events = ecliptic_longitude_lunar_phases(&fixture, window)
            .expect("fixture phase search succeeds");
        let analytical_events = ecliptic_longitude_lunar_phases(&AnalyticalEphemeris, window)
            .expect("analytical phase search succeeds");

        assert_eq!(fixture_events.len(), 1);
        assert_eq!(analytical_events.len(), 1);
        let fixture_event = &fixture_events[0];
        let analytical_event = &analytical_events[0];
        eprintln!(
            "{}: Horizons {:+.3}s from NASA minute; analytical {:+.3}s; provider delta {:.3}s; separations {:.6}/{:.6} deg",
            phase.name(),
            signed_seconds(fixture_event.interval().midpoint(), reference),
            signed_seconds(analytical_event.interval().midpoint(), reference),
            seconds_apart(
                fixture_event.interval().midpoint(),
                analytical_event.interval().midpoint(),
            ),
            fixture_event.angular_separation().degrees(),
            analytical_event.angular_separation().degrees(),
        );
        for event in &[fixture_event, analytical_event] {
            assert_eq!(event.phase(), phase);
            assert!(event.interval().width_days() * 86_400.0 <= 1.001);
            assert!(seconds_apart(event.interval().midpoint(), reference) < 20.0);
        }
        assert!(
            seconds_apart(
                fixture_event.interval().midpoint(),
                analytical_event.interval().midpoint(),
            ) < 10.0
        );
        assert_eq!(fixture_event.provider_model(), HORIZONS_FIXTURE);
        assert_eq!(analytical_event.provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(
            fixture_event.provider_snapshot(),
            Some("Horizons API 1.2 / DE441 / phase fixture generated 2026-08-24")
        );
        assert_eq!(analytical_event.provider_snapshot(), None);
    }
}

#[test]
fn lunar_phase_sequence_crosses_the_longitude_wrap() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let window = SearchWindow::new(start, end, 0.2, 1.0 / 86_400.0).unwrap();
    let provider = LinearProvider {
        origin: start,
        initial_degrees: 45.0,
        degrees_per_day: 360.0,
    };
    let events = ecliptic_longitude_lunar_phases(&provider, window).unwrap();
    let phases: Vec<LunarPhase> = events.iter().map(|event| event.phase()).collect();
    assert_eq!(
        phases,
        vec![
            LunarPhase::FirstQuarter,
            LunarPhase::FullMoon,
            LunarPhase::LastQuarter,
            LunarPhase::NewMoon,
        ]
    );
    assert_eq!(LunarPhase::NewMoon.target_elongation().degrees(), 0.0);
    assert_eq!(LunarPhase::LastQuarter.target_elongation().degrees(), 270.0);
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
    let fixture = HorizonsFixtureProvider::eclipse();
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

    let station_fixture = HorizonsFixtureProvider::mercury_station();
    let station_search = StationSearch::new(window, 0.25).unwrap();
    assert!(match ecliptic_longitude_stations(
        &station_fixture,
        ApparentBody::Mercury,
        station_search,
    ) {
        Err(EventError::Position {
            source: FixtureError::OutsideFixture,
            ..
        }) => true,
        _ => false,
    });
    assert!(matches!(
        ecliptic_longitude_lunar_phases(&fixture, window),
        Err(EventError::Position {
            source: FixtureError::OutsideFixture,
            ..
        })
    ));
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

#[test]
fn quadratic_longitude_reversal_is_refined_across_the_wrap() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let window = SearchWindow::new(start, end, 0.2, 1.0 / 86_400.0).unwrap();
    let search = StationSearch::new(window, 0.1).unwrap();
    for &station_day in &[0.4, 0.45] {
        let provider = QuadraticProvider {
            origin: start,
            station_day,
        };
        let events = ecliptic_longitude_stations(&provider, ApparentBody::Mercury, search).unwrap();
        assert_eq!(events.len(), 1);
        assert!(
            seconds_apart(
                events[0].interval().midpoint(),
                start.offset_days(station_day).unwrap(),
            ) < 1.0
        );
        assert_eq!(events[0].motion_before(), LongitudeMotion::Retrograde);
        assert_eq!(events[0].motion_after(), LongitudeMotion::Direct);
        assert!(events[0].longitude().degrees() > 359.8);
    }
}
