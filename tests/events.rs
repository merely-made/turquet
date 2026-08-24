// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use turquet::apparent::{ApparentBody, ANALYTICAL_APPARENT};
use turquet::events::{
    eclipse_candidates, ecliptic_longitude_conjunctions, ecliptic_longitude_lunar_phases,
    ecliptic_longitude_stations, lunar_eclipse_circumstances, EclipseCandidateGeometry,
    EclipseCandidateKind, EventError, LongitudeMotion, LunarEclipseContactKind, LunarEclipseKind,
    LunarEclipseSearch, LunarEclipseSearchError, LunarPhase, SearchWindow, SearchWindowError,
    StationSearch, StationSearchError, MAX_CONJUNCTION_STEP_DAYS,
    MAX_LUNAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS, MAX_STATION_VELOCITY_SPAN_DAYS,
    SPHERICAL_ECLIPSE_GEOMETRY,
};
use turquet::foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, ScaleAwareEpoch, State,
    TerrestrialTime, TrueEclipticEquinoxOfDate,
};
use turquet::provider::{AnalyticalEphemeris, GeocentricPositionProvider};

const VECTORS: &str = include_str!("vectors/eclipse_conjunction_horizons.tsv");
const STATION_VECTORS: &str = include_str!("vectors/mercury_station_horizons.tsv");
const PHASE_VECTORS: &str = include_str!("vectors/lunar_phases_horizons.tsv");
const ECLIPSE_GEOMETRY_VECTORS: &str = include_str!("vectors/eclipse_geometry_horizons.tsv");
const LUNAR_ECLIPSE_CIRCUMSTANCE_VECTORS: &str =
    include_str!("vectors/lunar_eclipse_circumstances_horizons.tsv");
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
fn eclipse_conjunction_matches_horizons_and_nasa_interval() {
    let start = tt_from_utc(2024, 4, 8, 18, 10, 0.0);
    let end = tt_from_utc(2024, 4, 8, 18, 30, 0.0);
    let reference = tt_from_utc(2024, 4, 8, 18, 20, 46.8);
    let window = SearchWindow::new(start, end, 10.0 / 1_440.0, 1.0 / 86_400.0)
        .expect("valid eclipse search window");

    let fixture = HorizonsFixtureProvider::eclipse();
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
fn eclipse_candidates_match_horizons_geometry_and_nasa_classes() {
    let cases = [
        (
            (2024, 3, 25, 7, 0),
            Some(EclipseCandidateKind::PenumbralLunar),
        ),
        ((2024, 4, 8, 18, 21), Some(EclipseCandidateKind::Solar)),
        ((2024, 4, 23, 23, 49), None),
        ((2024, 5, 8, 3, 22), None),
        (
            (2024, 9, 18, 2, 34),
            Some(EclipseCandidateKind::PartialLunar),
        ),
        ((2025, 3, 14, 6, 55), Some(EclipseCandidateKind::TotalLunar)),
        ((2025, 3, 29, 10, 58), Some(EclipseCandidateKind::Solar)),
    ];
    let fixture = HorizonsFixtureProvider::eclipse_geometry();

    for &((year, month, day, hour, minute), expected_kind) in &cases {
        let reference = tt_from_utc(year, month, day, hour, minute, 0.0);
        let start = reference.offset_days(-10.0 / 1_440.0).unwrap();
        let end = reference.offset_days(10.0 / 1_440.0).unwrap();
        let window = SearchWindow::new(start, end, 10.0 / 1_440.0, 1.0 / 86_400.0)
            .expect("valid eclipse candidate window");
        assert_eq!(
            ecliptic_longitude_lunar_phases(&fixture, window)
                .expect("fixture phase search succeeds")
                .len(),
            1
        );
        assert_eq!(
            ecliptic_longitude_lunar_phases(&AnalyticalEphemeris, window)
                .expect("analytical phase search succeeds")
                .len(),
            1
        );
        let fixture_candidates =
            eclipse_candidates(&fixture, window).expect("fixture eclipse search succeeds");
        let analytical_candidates = eclipse_candidates(&AnalyticalEphemeris, window)
            .expect("analytical eclipse search succeeds");

        match expected_kind {
            None => {
                assert!(fixture_candidates.is_empty());
                assert!(analytical_candidates.is_empty());
            }
            Some(kind) => {
                assert_eq!(fixture_candidates.len(), 1);
                assert_eq!(analytical_candidates.len(), 1);
                let fixture_candidate = &fixture_candidates[0];
                let analytical_candidate = &analytical_candidates[0];
                assert_eq!(fixture_candidate.kind(), kind);
                assert_eq!(analytical_candidate.kind(), kind);
                assert_eq!(
                    fixture_candidate.geometry_model(),
                    SPHERICAL_ECLIPSE_GEOMETRY
                );
                assert_eq!(
                    analytical_candidate.geometry_model(),
                    SPHERICAL_ECLIPSE_GEOMETRY
                );
                assert_eq!(fixture_candidate.provider_model(), HORIZONS_FIXTURE);
                assert_eq!(analytical_candidate.provider_model(), ANALYTICAL_APPARENT);
                assert_eq!(
                    fixture_candidate.provider_snapshot(),
                    Some(
                        "Horizons API 1.2 / DE441 / eclipse geometry fixture generated 2026-08-24"
                    )
                );
                assert_eq!(analytical_candidate.provider_snapshot(), None);
                assert!(fixture_candidate.interval().width_days() * 86_400.0 <= 1.001);
                assert!(analytical_candidate.interval().width_days() * 86_400.0 <= 1.001);
                let provider_delta = seconds_apart(
                    fixture_candidate.interval().midpoint(),
                    analytical_candidate.interval().midpoint(),
                );
                eprintln!("provider phase-root delta: {:.3}s", provider_delta);
                assert!(provider_delta < 20.0);
                print_eclipse_geometry(
                    year,
                    month,
                    day,
                    fixture_candidate.geometry(),
                    analytical_candidate.geometry(),
                );
            }
        }
    }
}

#[test]
fn lunar_eclipse_circumstances_match_horizons_and_nasa() {
    let cases = [
        (
            (2024, 3, 25, 7, 0, 14.6),
            (2024, 3, 25, 7, 12, 45.2),
            0.25,
            LunarEclipseKind::Penumbral,
            vec![
                (
                    LunarEclipseContactKind::PenumbralIngress,
                    (2024, 3, 25, 4, 53, 11.0),
                ),
                (
                    LunarEclipseContactKind::PenumbralEgress,
                    (2024, 3, 25, 9, 32, 18.0),
                ),
            ],
        ),
        (
            (2024, 9, 18, 2, 34, 22.9),
            (2024, 9, 18, 2, 44, 10.5),
            0.2,
            LunarEclipseKind::Partial,
            vec![
                (
                    LunarEclipseContactKind::PenumbralIngress,
                    (2024, 9, 18, 0, 41, 2.0),
                ),
                (
                    LunarEclipseContactKind::UmbralIngress,
                    (2024, 9, 18, 2, 12, 48.0),
                ),
                (
                    LunarEclipseContactKind::UmbralEgress,
                    (2024, 9, 18, 3, 15, 35.0),
                ),
                (
                    LunarEclipseContactKind::PenumbralEgress,
                    (2024, 9, 18, 4, 47, 18.0),
                ),
            ],
        ),
        (
            (2025, 3, 14, 6, 54, 33.5),
            (2025, 3, 14, 6, 58, 41.7),
            0.3,
            LunarEclipseKind::Total,
            vec![
                (
                    LunarEclipseContactKind::PenumbralIngress,
                    (2025, 3, 14, 3, 57, 24.0),
                ),
                (
                    LunarEclipseContactKind::UmbralIngress,
                    (2025, 3, 14, 5, 9, 33.0),
                ),
                (
                    LunarEclipseContactKind::TotalityBegins,
                    (2025, 3, 14, 6, 25, 59.0),
                ),
                (
                    LunarEclipseContactKind::TotalityEnds,
                    (2025, 3, 14, 7, 31, 23.0),
                ),
                (
                    LunarEclipseContactKind::UmbralEgress,
                    (2025, 3, 14, 8, 47, 48.0),
                ),
                (
                    LunarEclipseContactKind::PenumbralEgress,
                    (2025, 3, 14, 10, 0, 1.0),
                ),
            ],
        ),
    ];
    let fixture = HorizonsFixtureProvider::lunar_eclipse_circumstances();

    for &(phase_parts, greatest_parts, span_days, expected_kind, ref nasa_contacts) in &cases {
        let phase_reference = tt_from_parts(phase_parts);
        let greatest_reference = tt_from_parts(greatest_parts);
        let phase_window = SearchWindow::new(
            phase_reference.offset_days(-10.0 / 1_440.0).unwrap(),
            phase_reference.offset_days(10.0 / 1_440.0).unwrap(),
            10.0 / 1_440.0,
            1.0 / 86_400.0,
        )
        .unwrap();
        let search = LunarEclipseSearch::new(phase_window, span_days).unwrap();
        let fixture_events = lunar_eclipse_circumstances(&fixture, search)
            .expect("fixture circumstance search succeeds");
        let analytical_events = lunar_eclipse_circumstances(&AnalyticalEphemeris, search)
            .expect("analytical circumstance search succeeds");

        assert_eq!(fixture_events.len(), 1);
        assert_eq!(analytical_events.len(), 1);
        let fixture_event = &fixture_events[0];
        let analytical_event = &analytical_events[0];
        assert_eq!(fixture_event.kind(), expected_kind);
        assert_eq!(analytical_event.kind(), expected_kind);
        assert_eq!(fixture_event.contacts().len(), nasa_contacts.len());
        assert_eq!(analytical_event.contacts().len(), nasa_contacts.len());
        assert_eq!(fixture_event.geometry_model(), SPHERICAL_ECLIPSE_GEOMETRY);
        assert_eq!(
            analytical_event.geometry_model(),
            SPHERICAL_ECLIPSE_GEOMETRY
        );
        assert_eq!(fixture_event.provider_model(), HORIZONS_FIXTURE);
        assert_eq!(analytical_event.provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(
            fixture_event.provider_snapshot(),
            Some(
                "Horizons API 1.2 / DE441 / lunar eclipse circumstance fixture generated 2026-08-24"
            )
        );
        assert_eq!(analytical_event.provider_snapshot(), None);
        assert_eq!(fixture_event.circumstance_span_days(), span_days);
        assert_eq!(analytical_event.circumstance_span_days(), span_days);
        assert!(fixture_event.greatest_interval().width_days() * 86_400.0 <= 1.001);
        assert!(analytical_event.greatest_interval().width_days() * 86_400.0 <= 1.001);
        assert!(
            seconds_apart(
                fixture_event.greatest_interval().midpoint(),
                greatest_reference,
            ) < 20.0
        );
        assert!(
            seconds_apart(
                analytical_event.greatest_interval().midpoint(),
                greatest_reference,
            ) < 30.0
        );
        assert!(
            seconds_apart(
                fixture_event.greatest_interval().midpoint(),
                analytical_event.greatest_interval().midpoint(),
            ) < 15.0
        );
        for geometry in &[
            fixture_event.greatest_geometry(),
            analytical_event.greatest_geometry(),
        ] {
            assert!(geometry.shadow_axis_offset().kilometers() >= 0.0);
            assert!(geometry.umbra_radius().kilometers() > 0.0);
            assert!(geometry.penumbra_radius().kilometers() > geometry.umbra_radius().kilometers());
            assert!(geometry.moon_angular_radius().radians() > 0.0);
            assert!(geometry.umbra_angular_radius().radians() > 0.0);
            assert!(geometry.penumbra_angular_radius().radians() > 0.0);
        }
        eprintln!(
            "{:?}: greatest Horizons {:+.3}s from NASA; analytical {:+.3}s; provider delta {:.3}s; axis offsets {:.3}/{:.3} km",
            expected_kind,
            signed_seconds(fixture_event.greatest_interval().midpoint(), greatest_reference),
            signed_seconds(analytical_event.greatest_interval().midpoint(), greatest_reference),
            seconds_apart(
                fixture_event.greatest_interval().midpoint(),
                analytical_event.greatest_interval().midpoint(),
            ),
            fixture_event.greatest_geometry().shadow_axis_offset().kilometers(),
            analytical_event.greatest_geometry().shadow_axis_offset().kilometers(),
        );

        for (index, &(kind, reference_parts)) in nasa_contacts.iter().enumerate() {
            let reference = tt_from_parts(reference_parts);
            let fixture_contact = fixture_event.contacts()[index];
            let analytical_contact = analytical_event.contacts()[index];
            assert_eq!(fixture_contact.kind(), kind);
            assert_eq!(analytical_contact.kind(), kind);
            assert!(fixture_contact.interval().width_days() * 86_400.0 <= 1.001);
            assert!(analytical_contact.interval().width_days() * 86_400.0 <= 1.001);
            assert!(seconds_apart(fixture_contact.interval().midpoint(), reference) < 300.0);
            assert!(seconds_apart(analytical_contact.interval().midpoint(), reference) < 300.0);
            assert!(
                seconds_apart(
                    fixture_contact.interval().midpoint(),
                    analytical_contact.interval().midpoint(),
                ) < 30.0
            );
            eprintln!(
                "{}: Horizons {:+.3}s from NASA; analytical {:+.3}s; provider delta {:.3}s",
                kind.abbreviation(),
                signed_seconds(fixture_contact.interval().midpoint(), reference),
                signed_seconds(analytical_contact.interval().midpoint(), reference),
                seconds_apart(
                    fixture_contact.interval().midpoint(),
                    analytical_contact.interval().midpoint(),
                ),
            );
        }
    }
}

#[test]
fn lunar_eclipse_circumstances_require_exterior_span_endpoints() {
    let fixture = HorizonsFixtureProvider::lunar_eclipse_circumstances();
    let reference = tt_from_utc(2024, 3, 25, 7, 0, 14.6);
    let window = SearchWindow::new(
        reference.offset_days(-10.0 / 1_440.0).unwrap(),
        reference.offset_days(10.0 / 1_440.0).unwrap(),
        10.0 / 1_440.0,
        1.0 / 86_400.0,
    )
    .unwrap();
    let search = LunarEclipseSearch::new(window, 0.05).unwrap();
    assert!(matches!(
        lunar_eclipse_circumstances(&fixture, search),
        Err(EventError::CircumstanceSpanTooShort { .. })
    ));
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
fn eclipse_geometry_refuses_a_body_inside_its_required_radius() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let window = SearchWindow::new(start, end, 0.25, 1.0 / 86_400.0).unwrap();
    let provider = ZeroDistanceProvider { origin: start };
    assert!(matches!(
        eclipse_candidates(&provider, window),
        Err(EventError::DistanceTooSmall {
            body: ApparentBody::Moon,
            distance_km: 0.0,
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
    snapshot: &'static str,
}

impl HorizonsFixtureProvider {
    fn eclipse() -> Self {
        Self::from_vectors(
            VECTORS,
            6,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "reference: NASA GSFC 2024-04-08 ecliptic conjunction 18:20:46.8 UT",
                "fetch_horizons_conjunction_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / generated 2026-08-23",
        )
    }

    fn mercury_station() -> Self {
        Self::from_vectors(
            STATION_VECTORS,
            37,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "hourly apparent Mercury positions",
                "fetch_horizons_station_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / station fixture generated 2026-08-24",
        )
    }

    fn lunar_phases() -> Self {
        Self::from_vectors(
            PHASE_VECTORS,
            24,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "NASA GSFC April 2024 quarter-phase minute",
                "fetch_horizons_phase_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / phase fixture generated 2026-08-24",
        )
    }

    fn eclipse_geometry() -> Self {
        Self::from_vectors(
            ECLIPSE_GEOMETRY_VECTORS,
            42,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "2024-03-25 penumbral lunar",
                "09-18 partial lunar",
                "2025-03-14 total lunar",
                "03-29 partial solar",
                "fetch_horizons_eclipse_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / eclipse geometry fixture generated 2026-08-24",
        )
    }

    fn lunar_eclipse_circumstances() -> Self {
        Self::from_vectors(
            LUNAR_ECLIPSE_CIRCUMSTANCE_VECTORS,
            162,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "fifteen-minute Sun and Moon states",
                "NASA Danjon shadow enlargement",
                "fetch_horizons_lunar_eclipse_circumstances.ps1",
            ],
            "Horizons API 1.2 / DE441 / lunar eclipse circumstance fixture generated 2026-08-24",
        )
    }

    fn from_vectors(
        vectors: &str,
        expected_rows: usize,
        expected_headers: &[&str],
        snapshot: &'static str,
    ) -> Self {
        for expected in expected_headers {
            assert!(
                vectors.lines().any(|line| line.contains(expected)),
                "Horizons fixture header must record {}",
                expected
            );
        }
        let mut rows = Vec::new();
        for line in vectors.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 5);
            let body = match fields[0] {
                "sun" => ApparentBody::Sun,
                "moon" => ApparentBody::Moon,
                "mercury" => ApparentBody::Mercury,
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
        assert_eq!(rows.len(), expected_rows);
        Self { rows, snapshot }
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
        Some(self.snapshot)
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

struct QuadraticProvider {
    origin: JulianDate<TerrestrialTime>,
    station_day: f64,
}

struct ZeroDistanceProvider {
    origin: JulianDate<TerrestrialTime>,
}

impl GeocentricPositionProvider for ZeroDistanceProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("zero-distance test provider", "1")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let longitude = if body == ApparentBody::Moon {
            350.0 + 20.0 * (epoch.day() - self.origin.day())
        } else {
            0.0
        };
        let distance = if body == ApparentBody::Moon {
            Distance::from_kilometers(0.0).unwrap()
        } else {
            Distance::from_astronomical_units(1.0).unwrap()
        };
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).unwrap(),
                Latitude::from_degrees(0.0).unwrap(),
            ),
            distance,
        ))
    }
}

impl GeocentricPositionProvider for QuadraticProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("quadratic test provider", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let offset = epoch.day() - self.origin.day() - self.station_day;
        let longitude = 359.9 + 20.0 * offset * offset;
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

fn tt_from_parts(parts: (i32, u8, u8, u8, u8, f64)) -> JulianDate<TerrestrialTime> {
    tt_from_utc(parts.0, parts.1, parts.2, parts.3, parts.4, parts.5)
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

fn print_eclipse_geometry(
    year: i32,
    month: u8,
    day: u8,
    fixture: EclipseCandidateGeometry,
    analytical: EclipseCandidateGeometry,
) {
    match (fixture, analytical) {
        (
            EclipseCandidateGeometry::Solar {
                center_separation: fixture_separation,
                sun_angular_radius: fixture_sun,
                moon_angular_radius: fixture_moon,
                observer_parallax_allowance: fixture_parallax,
            },
            EclipseCandidateGeometry::Solar {
                center_separation: analytical_separation,
                sun_angular_radius: analytical_sun,
                moon_angular_radius: analytical_moon,
                observer_parallax_allowance: analytical_parallax,
            },
        ) => {
            eprintln!(
                "{year:04}-{month:02}-{day:02} solar: separation {:.6}/{:.6} deg; disk radii {:.6}+{:.6}/{:.6}+{:.6} deg; parallax allowance {:.6}/{:.6} deg",
                fixture_separation.degrees(),
                analytical_separation.degrees(),
                fixture_sun.degrees(),
                fixture_moon.degrees(),
                analytical_sun.degrees(),
                analytical_moon.degrees(),
                fixture_parallax.degrees(),
                analytical_parallax.degrees(),
            );
            assert!((fixture_separation.degrees() - analytical_separation.degrees()).abs() < 0.01);
            assert!((fixture_sun.degrees() - analytical_sun.degrees()).abs() < 0.001);
            assert!((fixture_moon.degrees() - analytical_moon.degrees()).abs() < 0.001);
            assert!((fixture_parallax.degrees() - analytical_parallax.degrees()).abs() < 0.001);
            if (year, month, day) == (2025, 3, 29) {
                assert!(
                    fixture_separation.radians() > fixture_sun.radians() + fixture_moon.radians()
                );
                assert!(
                    fixture_separation.radians()
                        <= fixture_sun.radians()
                            + fixture_moon.radians()
                            + fixture_parallax.radians()
                );
            }
        }
        (
            EclipseCandidateGeometry::Lunar {
                shadow_axis_separation: fixture_axis,
                moon_angular_radius: fixture_moon,
                umbra_angular_radius: fixture_umbra,
                penumbra_angular_radius: fixture_penumbra,
            },
            EclipseCandidateGeometry::Lunar {
                shadow_axis_separation: analytical_axis,
                moon_angular_radius: analytical_moon,
                umbra_angular_radius: analytical_umbra,
                penumbra_angular_radius: analytical_penumbra,
            },
        ) => {
            eprintln!(
                "{year:04}-{month:02}-{day:02} lunar: axis {:.6}/{:.6} deg; Moon radius {:.6}/{:.6} deg; umbra {:.6}/{:.6} deg; penumbra {:.6}/{:.6} deg",
                fixture_axis.degrees(),
                analytical_axis.degrees(),
                fixture_moon.degrees(),
                analytical_moon.degrees(),
                fixture_umbra.degrees(),
                analytical_umbra.degrees(),
                fixture_penumbra.degrees(),
                analytical_penumbra.degrees(),
            );
            assert!((fixture_axis.degrees() - analytical_axis.degrees()).abs() < 0.01);
            assert!((fixture_moon.degrees() - analytical_moon.degrees()).abs() < 0.001);
            assert!((fixture_umbra.degrees() - analytical_umbra.degrees()).abs() < 0.001);
            assert!((fixture_penumbra.degrees() - analytical_penumbra.degrees()).abs() < 0.001);
        }
        _ => panic!("providers returned different eclipse geometry variants"),
    }
}
