// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use super::*;

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
fn named_airless_rise_set_remains_a_caller_threshold_projection() {
    let fixture = HorizonsAltitudeFixture::parse();
    let case = "boston_sun";
    let rows = fixture.case_rows(case);
    let first = rows[0];
    let last = rows[rows.len() - 1];
    let site = observer(first.longitude, first.latitude, first.height_meters);
    let threshold = Angle::from_degrees(5.0).unwrap();
    let window = SearchWindow::new(
        JulianDate::from_julian_day(first.tt_day).unwrap(),
        JulianDate::from_julian_day(last.tt_day).unwrap(),
        1.0 / 24.0,
        1.0 / 86_400.0,
    )
    .unwrap();
    let search = AltitudeCrossingSearch::new(window, threshold).unwrap();
    let references = direct_altitude_crossings(&rows, threshold.degrees());
    assert_eq!(references.len(), 2);

    let raw_case = fixture.for_case(case);
    let raw = airless_altitude_crossings(&raw_case, &raw_case, site, ApparentBody::Sun, search)
        .expect("caller-threshold airless crossings");
    let named_case = fixture.for_case(case);
    let named = airless_rise_set_events(&named_case, &named_case, site, ApparentBody::Sun, search)
        .expect("caller-threshold named airless rise/set");

    assert_eq!(named.len(), references.len());
    assert_eq!(raw.len(), references.len());
    for (index, reference) in references.iter().enumerate() {
        let event = &named[index];
        assert_eq!(event.crossing(), &raw[index]);
        assert_eq!(
            event.kind(),
            match reference.kind {
                AltitudeCrossingKind::Ascending => AirlessRiseSetKind::Rise,
                AltitudeCrossingKind::Descending => AirlessRiseSetKind::Set,
            }
        );
        assert_eq!(event.naming_model(), AIRLESS_RISE_SET_NAMING);
        assert_eq!(event.crossing().threshold(), threshold);
        let seconds =
            (event.crossing().interval().midpoint().day() - reference.tt_day).abs() * 86_400.0;
        eprintln!(
            "Boston Sun {:?} at {} deg: fixture/direct {:.3} s",
            event.kind(),
            threshold.degrees(),
            seconds
        );
        assert!(seconds <= 2.0);
    }
}
