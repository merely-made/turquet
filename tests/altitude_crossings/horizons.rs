// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn both_position_providers_match_direct_horizons_altitude_crossings() {
    for expected in &[
        "oracle: NASA/JPL Horizons API 1.2, DE441",
        "quantities 4,20,31,42,49",
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
        let references = direct_altitude_crossings(&rows, 0.0);
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
fn both_position_providers_match_direct_horizons_meridian_transits() {
    let fixture = HorizonsAltitudeFixture::parse();
    for &(case, expected_body) in &[
        ("boston_sun", ApparentBody::Sun),
        ("sydney_moon", ApparentBody::Moon),
        ("tromso_sun_empty", ApparentBody::Sun),
    ] {
        let rows = fixture.case_rows(case);
        let first = rows[0];
        let last = rows[rows.len() - 1];
        let site = observer(first.longitude, first.latitude, first.height_meters);
        let window = SearchWindow::new(
            JulianDate::from_julian_day(first.tt_day).unwrap(),
            JulianDate::from_julian_day(last.tt_day).unwrap(),
            1.0 / 24.0,
            1.0 / 86_400.0,
        )
        .unwrap();
        let search = MeridianTransitSearch::new(window).unwrap();
        let references = direct_meridian_transits(&rows);
        assert_eq!(references.len(), 2, "{} direct transits", case);

        let fixture_case = fixture.for_case(case);
        let fixture_events =
            meridian_transits(&fixture_case, &fixture_case, site, expected_body, search)
                .expect("fixture-provider meridian transits");
        let analytical_eop = fixture.for_case(case);
        let analytical_events = meridian_transits(
            &AnalyticalEphemeris,
            &analytical_eop,
            site,
            expected_body,
            search,
        )
        .expect("analytical-provider meridian transits");

        assert_eq!(fixture_events.len(), references.len());
        assert_eq!(analytical_events.len(), references.len());
        for (index, reference) in references.iter().enumerate() {
            let fixture_event = &fixture_events[index];
            let analytical_event = &analytical_events[index];
            assert_eq!(fixture_event.kind(), reference.kind);
            assert_eq!(analytical_event.kind(), reference.kind);
            for (lane, event) in [("fixture", fixture_event), ("analytical", analytical_event)] {
                let seconds =
                    (event.interval().midpoint().day() - reference.tt_day).abs() * 86_400.0;
                eprintln!(
                    "{} {:?} {} / direct: {:.3} s",
                    case, reference.kind, lane, seconds
                );
                assert!(seconds <= 1.0);
                assert!(event.interval().width_days() <= 1.0 / 86_400.0);
                assert_eq!(event.body(), expected_body);
                assert_eq!(event.observer(), site);
                assert_eq!(event.transit_model(), TOPOCENTRIC_MERIDIAN_TRANSIT_MODEL);
                assert_eq!(event.transform_model(), AIRLESS_TOPOCENTRIC_TRANSFORM);
                assert_eq!(event.earth_orientation_authority(), HORIZONS_EOP_AUTHORITY);
                assert_eq!(event.earth_orientation_snapshot(), HORIZONS_EOP_SNAPSHOT);
            }
            assert_eq!(fixture_event.provider_model(), HORIZONS_FIXTURE_MODEL);
            assert_eq!(
                fixture_event.provider_snapshot(),
                Some(HORIZONS_FIXTURE_SNAPSHOT)
            );
            assert_eq!(analytical_event.provider_model(), ANALYTICAL_APPARENT);
            assert_eq!(analytical_event.provider_snapshot(), None);
        }

        if case == "boston_sun" {
            let lower = fixture_events
                .iter()
                .find(|event| event.kind() == MeridianTransitKind::Lower)
                .expect("Boston lower transit");
            assert!(lower.midpoint_altitude().degrees() < 0.0);
        }

        if case == "sydney_moon" {
            let extrema_window = SearchWindow::new(
                JulianDate::from_julian_day(first.tt_day + 5.0 / 1_440.0).unwrap(),
                JulianDate::from_julian_day(last.tt_day - 5.0 / 1_440.0).unwrap(),
                1.0 / 24.0,
                1.0 / 86_400.0,
            )
            .unwrap();
            let extrema_case = fixture.for_case(case);
            let extrema = airless_altitude_extrema(
                &extrema_case,
                &extrema_case,
                site,
                expected_body,
                AltitudeExtremumSearch::new(extrema_window, 10.0 / 1_440.0).unwrap(),
            )
            .expect("Sydney Moon altitude extrema");
            let upper = fixture_events
                .iter()
                .find(|event| event.kind() == MeridianTransitKind::Upper)
                .expect("Sydney upper transit");
            let maximum = extrema
                .iter()
                .find(|event| event.kind() == AltitudeExtremumKind::Maximum)
                .expect("Sydney altitude maximum");
            let difference_seconds =
                (upper.interval().midpoint().day() - maximum.interval().midpoint().day()).abs()
                    * 86_400.0;
            eprintln!(
                "Sydney Moon upper transit differs from airless altitude maximum by {:.3} s",
                difference_seconds
            );
            assert!(difference_seconds > 1.0);
        }

        if case == "tromso_sun_empty" {
            let named_case = fixture.for_case(case);
            let rise_set = airless_rise_set_events(
                &named_case,
                &named_case,
                site,
                expected_body,
                AltitudeCrossingSearch::new(window, Angle::from_degrees(0.0).unwrap()).unwrap(),
            )
            .expect("Tromso named airless rise/set");
            assert!(rise_set.is_empty());
            assert_eq!(fixture_events.len(), 2);
        }
    }
}

#[test]
fn both_position_providers_match_direct_horizons_altitude_extrema() {
    let fixture = HorizonsAltitudeFixture::parse();
    for &(case, expected_body, expected_state) in &[
        ("boston_sun", ApparentBody::Sun, "crosses"),
        ("sydney_moon", ApparentBody::Moon, "crosses"),
        ("tromso_sun_empty", ApparentBody::Sun, "above"),
    ] {
        let rows = fixture.case_rows(case);
        let first = rows[0];
        let last = rows[rows.len() - 1];
        let observer = observer(first.longitude, first.latitude, first.height_meters);
        let window = SearchWindow::new(
            JulianDate::from_julian_day(first.tt_day + 5.0 / 1_440.0).unwrap(),
            JulianDate::from_julian_day(last.tt_day - 5.0 / 1_440.0).unwrap(),
            1.0 / 24.0,
            1.0 / 86_400.0,
        )
        .unwrap();
        let extrema_search = AltitudeExtremumSearch::new(window, 10.0 / 1_440.0).unwrap();
        let search = AltitudeCircumstanceSearch::new(
            extrema_search,
            Angle::from_degrees(0.0).unwrap(),
            Angle::from_degrees(0.01).unwrap(),
        )
        .unwrap();
        let references = direct_altitude_extrema(&rows);
        assert_eq!(references.len(), 2);

        let fixture_case = fixture.for_case(case);
        let fixture_result = airless_altitude_circumstances(
            &fixture_case,
            &fixture_case,
            observer,
            expected_body,
            search,
        )
        .expect("fixture-provider altitude circumstances");
        let analytical_eop = fixture.for_case(case);
        let analytical_result = airless_altitude_circumstances(
            &AnalyticalEphemeris,
            &analytical_eop,
            observer,
            expected_body,
            search,
        )
        .expect("analytical-provider altitude circumstances");

        assert_eq!(fixture_result.extrema().len(), 2);
        assert_eq!(analytical_result.extrema().len(), 2);
        assert_threshold_state(fixture_result.state(), expected_state);
        assert_threshold_state(analytical_result.state(), expected_state);
        for (index, reference) in references.iter().enumerate() {
            for (lane, event) in [
                ("fixture", &fixture_result.extrema()[index]),
                ("analytical", &analytical_result.extrema()[index]),
            ] {
                assert_eq!(event.kind(), reference.kind);
                assert!(event.interval().width_days() <= 1.0 / 86_400.0);
                let time_residual =
                    (event.interval().midpoint().day() - reference.tt_day).abs() * 86_400.0;
                let altitude_residual =
                    (event.midpoint_altitude().degrees() - reference.altitude_degrees).abs();
                eprintln!(
                    "{} {:?} {}: direct time {:.3} s; altitude {:.6} deg",
                    case, reference.kind, lane, time_residual, altitude_residual
                );
                assert!(time_residual <= 1.0);
                assert!(altitude_residual <= 0.001);
                assert_eq!(event.derivative_span_days(), 10.0 / 1_440.0);
                assert_eq!(event.transform_model(), AIRLESS_TOPOCENTRIC_TRANSFORM);
                assert_eq!(event.earth_orientation_authority(), HORIZONS_EOP_AUTHORITY);
                assert_eq!(event.earth_orientation_snapshot(), HORIZONS_EOP_SNAPSHOT);
            }
        }
        assert_eq!(fixture_result.provider_model(), HORIZONS_FIXTURE_MODEL);
        assert_eq!(
            fixture_result.provider_snapshot(),
            Some(HORIZONS_FIXTURE_SNAPSHOT)
        );
        assert_eq!(analytical_result.provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(analytical_result.provider_snapshot(), None);

        if case == "boston_sun" {
            let standalone_case = fixture.for_case(case);
            let standalone = airless_altitude_extrema(
                &standalone_case,
                &standalone_case,
                observer,
                expected_body,
                extrema_search,
            )
            .expect("standalone altitude extrema");
            assert_eq!(standalone, fixture_result.extrema());
        }
    }
}
