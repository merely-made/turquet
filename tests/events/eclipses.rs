// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use super::*;

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
            },
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
            },
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
            signed_seconds(
                fixture_event.greatest_interval().midpoint(),
                greatest_reference
            ),
            signed_seconds(
                analytical_event.greatest_interval().midpoint(),
                greatest_reference
            ),
            seconds_apart(
                fixture_event.greatest_interval().midpoint(),
                analytical_event.greatest_interval().midpoint(),
            ),
            fixture_event
                .greatest_geometry()
                .shadow_axis_offset()
                .kilometers(),
            analytical_event
                .greatest_geometry()
                .shadow_axis_offset()
                .kilometers(),
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
