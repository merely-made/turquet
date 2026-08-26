// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

#[path = "support/horizons_altitude_fixture.rs"]
mod horizons_altitude_fixture;

use horizons_altitude_fixture::{
    DirectAltitudeCrossingKind, FixtureError, HorizonsAltitudeFixture, HORIZONS_EOP_AUTHORITY,
    HORIZONS_EOP_SNAPSHOT, HORIZONS_FIXTURE_MODEL, HORIZONS_FIXTURE_SNAPSHOT,
};
use turquet::apparent::{ApparentBody, ANALYTICAL_APPARENT};
use turquet::events::{
    airless_altitude_crossings, airless_solar_twilight_events, AltitudeCrossingError,
    AltitudeCrossingKind, AltitudeCrossingSearch, AltitudeCrossingSearchError, SearchWindow,
    SolarTwilightKind, AIRLESS_SOLAR_TWILIGHT_NAMING,
};
use turquet::foundation::{Angle, JulianDate};
use turquet::observer::AIRLESS_TOPOCENTRIC_TRANSFORM;
use turquet::provider::AnalyticalEphemeris;

const MAX_FIXTURE_ROOT_RESIDUAL_SECONDS: f64 = 0.5;
const MAX_ANALYTICAL_ROOT_RESIDUAL_SECONDS: f64 = 0.5;

fn search_for(
    case: &horizons_altitude_fixture::HorizonsAltitudeCase<'_>,
) -> AltitudeCrossingSearch {
    let rows = case.rows();
    let start = JulianDate::from_julian_day(rows[0].tt_day).expect("fixture TT start");
    let end = JulianDate::from_julian_day(rows[rows.len() - 1].tt_day).expect("fixture TT end");
    AltitudeCrossingSearch::new(
        SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap(),
        Angle::from_degrees(-6.0).unwrap(),
    )
    .unwrap()
}

#[test]
fn boston_sun_named_twilight_matches_direct_quantity_four_roots() {
    let fixture = HorizonsAltitudeFixture::parse();
    let observer = fixture.observer("boston_sun");
    for threshold in [-6.0, -12.0, -18.0] {
        let case = fixture.for_case("boston_sun");
        let rows = case.rows();
        let start = JulianDate::from_julian_day(rows[0].tt_day).unwrap();
        let end = JulianDate::from_julian_day(rows[rows.len() - 1].tt_day).unwrap();
        let angle = Angle::from_degrees(threshold).unwrap();
        let search = AltitudeCrossingSearch::new(
            SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap(),
            angle,
        )
        .unwrap();
        let direct = case.direct_threshold_crossings(threshold);
        let events = airless_solar_twilight_events(&case, &case, observer, search)
            .expect("fixture twilight search");
        let analytical_events =
            airless_solar_twilight_events(&AnalyticalEphemeris, &case, observer, search)
                .expect("analytical twilight search");
        assert_eq!(direct.len(), 2);
        assert_eq!(events.len(), direct.len());
        assert_eq!(analytical_events.len(), direct.len());
        for ((event, analytical_event), reference) in events
            .iter()
            .zip(analytical_events.iter())
            .zip(direct.iter())
        {
            let expected_kind = match reference.kind {
                DirectAltitudeCrossingKind::Ascending => SolarTwilightKind::Dawn,
                DirectAltitudeCrossingKind::Descending => SolarTwilightKind::Dusk,
            };
            assert_eq!(event.kind(), expected_kind);
            assert_eq!(
                event.crossing().kind(),
                match expected_kind {
                    SolarTwilightKind::Dawn => AltitudeCrossingKind::Ascending,
                    SolarTwilightKind::Dusk => AltitudeCrossingKind::Descending,
                }
            );
            assert_eq!(event.crossing().body(), ApparentBody::Sun);
            assert_eq!(event.crossing().observer(), observer);
            assert_eq!(event.crossing().threshold(), angle);
            assert_eq!(event.naming_model(), AIRLESS_SOLAR_TWILIGHT_NAMING);
            assert_eq!(event.crossing().provider_model(), HORIZONS_FIXTURE_MODEL);
            assert_eq!(
                event.crossing().provider_snapshot(),
                Some(HORIZONS_FIXTURE_SNAPSHOT)
            );
            assert_eq!(
                event.crossing().transform_model(),
                AIRLESS_TOPOCENTRIC_TRANSFORM
            );
            assert_eq!(
                event.crossing().earth_orientation_authority(),
                HORIZONS_EOP_AUTHORITY
            );
            assert_eq!(
                event.crossing().earth_orientation_snapshot(),
                HORIZONS_EOP_SNAPSHOT
            );
            let seconds =
                (event.crossing().interval().midpoint().day() - reference.tt_day).abs() * 86_400.0;
            let analytical_seconds =
                (analytical_event.crossing().interval().midpoint().day() - reference.tt_day).abs()
                    * 86_400.0;
            eprintln!(
                "Boston Sun {threshold} degree {expected_kind:?}: fixture {seconds:.3} s, analytical {analytical_seconds:.3} s"
            );
            assert!(
                seconds <= MAX_FIXTURE_ROOT_RESIDUAL_SECONDS,
                "fixture root residual {} s",
                seconds
            );
            assert!(
                analytical_seconds <= MAX_ANALYTICAL_ROOT_RESIDUAL_SECONDS,
                "analytical root residual {} s",
                analytical_seconds
            );
            assert!(event.crossing().interval().width_days() <= 1.0 / 86_400.0);
            assert!(analytical_event.crossing().interval().width_days() <= 1.0 / 86_400.0);
        }
    }
}

#[test]
fn solar_twilight_is_a_named_wrapper_over_airless_crossings() {
    let fixture = HorizonsAltitudeFixture::parse();
    let case = fixture.for_case("boston_sun");
    let observer = fixture.observer("boston_sun");
    let search = search_for(&case);
    let named = airless_solar_twilight_events(&case, &case, observer, search).unwrap();
    let raw =
        airless_altitude_crossings(&case, &case, observer, ApparentBody::Sun, search).unwrap();
    assert_eq!(named.len(), raw.len());
    for (event, crossing) in named.iter().zip(raw.iter()) {
        assert_eq!(event.crossing(), crossing);
    }
}

#[test]
fn analytical_and_fixture_lanes_preserve_twilight_provenance() {
    let fixture = HorizonsAltitudeFixture::parse();
    let case = fixture.for_case("boston_sun");
    let observer = fixture.observer("boston_sun");
    let search = search_for(&case);
    let fixture_events = airless_solar_twilight_events(&case, &case, observer, search).unwrap();
    let analytical_events =
        airless_solar_twilight_events(&AnalyticalEphemeris, &case, observer, search).unwrap();
    assert_eq!(fixture_events.len(), 2);
    assert_eq!(analytical_events.len(), fixture_events.len());
    for event in analytical_events {
        assert_eq!(event.crossing().provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(event.crossing().provider_snapshot(), None);
        assert_eq!(
            event.crossing().transform_model(),
            AIRLESS_TOPOCENTRIC_TRANSFORM
        );
    }
}

#[test]
fn high_latitude_solar_twilight_can_be_empty_without_classifying_visibility() {
    let fixture = HorizonsAltitudeFixture::parse();
    let case = fixture.for_case("tromso_sun_empty");
    let observer = fixture.observer("tromso_sun_empty");
    let search = search_for(&case);
    let fixture_events = airless_solar_twilight_events(&case, &case, observer, search).unwrap();
    let analytical_events =
        airless_solar_twilight_events(&AnalyticalEphemeris, &case, observer, search).unwrap();
    assert!(fixture_events.is_empty());
    assert!(analytical_events.is_empty());
}

#[test]
fn twilight_uses_existing_altitude_search_validation() {
    let fixture = HorizonsAltitudeFixture::parse();
    let case = fixture.for_case("boston_sun");
    let rows = case.rows();
    let start = JulianDate::from_julian_day(rows[0].tt_day).unwrap();
    let end = JulianDate::from_julian_day(rows[1].tt_day).unwrap();
    let window = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeCrossingSearch::new(window, Angle::from_degrees(91.0).unwrap()),
        Err(AltitudeCrossingSearchError::ThresholdOutOfRange)
    );
    let coarse = SearchWindow::new(start, end, 1.0 / 23.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeCrossingSearch::new(coarse, Angle::from_degrees(-6.0).unwrap()),
        Err(AltitudeCrossingSearchError::StepTooLarge)
    );
}

#[test]
fn twilight_propagates_fixture_position_errors() {
    let fixture = HorizonsAltitudeFixture::parse();
    let case = fixture.for_case("boston_sun");
    let rows = case.rows();
    let start = JulianDate::from_julian_day(rows[0].tt_day - 0.01).unwrap();
    let end = JulianDate::from_julian_day(rows[1].tt_day).unwrap();
    let search = AltitudeCrossingSearch::new(
        SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap(),
        Angle::from_degrees(-6.0).unwrap(),
    )
    .unwrap();
    let error = airless_solar_twilight_events(&case, &case, fixture.observer("boston_sun"), search)
        .unwrap_err();
    match error {
        AltitudeCrossingError::Position {
            source: FixtureError::OutsideFixture,
            ..
        } => {}
        other => panic!("unexpected twilight error: {:?}", other),
    }
}
