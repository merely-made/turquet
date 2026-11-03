// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn altitude_search_rejects_coarse_steps_and_nonphysical_thresholds() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_460_000.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let coarse = SearchWindow::new(start, end, 2.0 / 24.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeCrossingSearch::new(coarse, Angle::from_degrees(0.0).unwrap()).unwrap_err(),
        AltitudeCrossingSearchError::StepTooLarge
    );
    assert_eq!(
        ConventionalRiseSetSearch::new(coarse).unwrap_err(),
        ConventionalRiseSetSearchError::StepTooLarge
    );

    let hourly = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeCrossingSearch::new(hourly, Angle::from_degrees(90.1).unwrap()).unwrap_err(),
        AltitudeCrossingSearchError::ThresholdOutOfRange
    );
    assert_eq!(
        MeridianTransitSearch::new(coarse).unwrap_err(),
        MeridianTransitSearchError::StepTooLarge
    );
    assert_eq!(MeridianTransitSearch::new(hourly).unwrap().window(), hourly);
}

#[test]
fn altitude_extremum_and_circumstance_searches_reject_unsafe_controls() {
    let start = JulianDate::<TerrestrialTime>::from_julian_day(2_460_000.0).unwrap();
    let end = start.offset_days(1.0).unwrap();
    let coarse = SearchWindow::new(start, end, 2.0 / 24.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeExtremumSearch::new(coarse, 10.0 / 1_440.0).unwrap_err(),
        AltitudeExtremumSearchError::StepTooLarge
    );
    let hourly = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    assert_eq!(
        AltitudeExtremumSearch::new(hourly, f64::NAN).unwrap_err(),
        AltitudeExtremumSearchError::DerivativeSpanNotFinite
    );
    assert_eq!(
        AltitudeExtremumSearch::new(hourly, 0.0).unwrap_err(),
        AltitudeExtremumSearchError::DerivativeSpanNotPositive
    );
    assert_eq!(
        AltitudeExtremumSearch::new(hourly, 2.0 / 24.0).unwrap_err(),
        AltitudeExtremumSearchError::DerivativeSpanTooLarge
    );
    let extrema = AltitudeExtremumSearch::new(hourly, 10.0 / 1_440.0).unwrap();
    assert_eq!(
        AltitudeCircumstanceSearch::new(
            extrema,
            Angle::from_degrees(90.1).unwrap(),
            Angle::from_degrees(0.01).unwrap(),
        )
        .unwrap_err(),
        AltitudeCircumstanceSearchError::ThresholdOutOfRange
    );
    assert_eq!(
        AltitudeCircumstanceSearch::new(
            extrema,
            Angle::from_degrees(0.0).unwrap(),
            Angle::from_degrees(0.0).unwrap(),
        )
        .unwrap_err(),
        AltitudeCircumstanceSearchError::AltitudeToleranceNotPositive
    );
}

#[test]
fn sampled_state_distinguishes_below_and_grazing_candidate() {
    let fixture = HorizonsAltitudeFixture::parse();
    let case = "tromso_sun_empty";
    let rows = fixture.case_rows(case);
    let first = rows[0];
    let last = rows[rows.len() - 1];
    let site = observer(first.longitude, first.latitude, first.height_meters);
    let window = SearchWindow::new(
        JulianDate::from_julian_day(first.tt_day + 5.0 / 1_440.0).unwrap(),
        JulianDate::from_julian_day(last.tt_day - 5.0 / 1_440.0).unwrap(),
        1.0 / 24.0,
        1.0 / 86_400.0,
    )
    .unwrap();
    let extrema = AltitudeExtremumSearch::new(window, 10.0 / 1_440.0).unwrap();

    let below_case = fixture.for_case(case);
    let below = airless_altitude_circumstances(
        &below_case,
        &below_case,
        site,
        ApparentBody::Sun,
        AltitudeCircumstanceSearch::new(
            extrema,
            Angle::from_degrees(80.0).unwrap(),
            Angle::from_degrees(0.01).unwrap(),
        )
        .unwrap(),
    )
    .expect("sampled-below Tromso case");
    assert!(matches!(
        below.state(),
        AltitudeThresholdState::BelowAtAllSamples { .. }
    ));

    let direct_minimum = direct_altitude_extrema(&rows)
        .into_iter()
        .find(|event| event.kind == AltitudeExtremumKind::Minimum)
        .expect("direct minimum");
    let grazing_case = fixture.for_case(case);
    let grazing = airless_altitude_circumstances(
        &grazing_case,
        &grazing_case,
        site,
        ApparentBody::Sun,
        AltitudeCircumstanceSearch::new(
            extrema,
            Angle::from_degrees(direct_minimum.altitude_degrees).unwrap(),
            Angle::from_degrees(0.01).unwrap(),
        )
        .unwrap(),
    )
    .expect("Tromso grazing candidate");
    match grazing.state() {
        AltitudeThresholdState::GrazingCandidate { extremum, offset } => {
            assert_eq!(extremum.kind(), AltitudeExtremumKind::Minimum);
            assert!(offset.degrees().abs() <= 0.01);
        },
        other => panic!("expected grazing candidate, got {:?}", other),
    }
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
        },
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
        },
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
        },
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
        AltitudeCrossingError::Transform { .. } => {},
        other => panic!("unexpected transform error: {:?}", other),
    }

    let extrema = AltitudeExtremumSearch::new(window, 10.0 / 1_440.0).unwrap();
    match airless_altitude_extrema(
        &FailingPosition,
        &orientation,
        site,
        ApparentBody::Sun,
        extrema,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::Position { source, .. } => {
            assert_eq!(source, TestError("position"))
        },
        other => panic!("unexpected extremum position error: {:?}", other),
    }
    match airless_altitude_extrema(
        &AnalyticalEphemeris,
        &FailingOrientation,
        site,
        ApparentBody::Sun,
        extrema,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::EarthOrientation { source, .. } => {
            assert_eq!(source, TestError("orientation"))
        },
        other => panic!("unexpected extremum orientation error: {:?}", other),
    }
    match airless_altitude_extrema(
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
        extrema,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::EarthOrientationIdentityMismatch { .. } => {},
        other => panic!("unexpected extremum identity error: {:?}", other),
    }
    match airless_altitude_extrema(
        &WrongEpochPosition,
        &orientation,
        site,
        ApparentBody::Sun,
        extrema,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::Transform { .. } => {},
        other => panic!("unexpected extremum transform error: {:?}", other),
    }

    let circumstances = AltitudeCircumstanceSearch::new(
        extrema,
        Angle::from_degrees(0.0).unwrap(),
        Angle::from_degrees(0.01).unwrap(),
    )
    .unwrap();
    match airless_altitude_circumstances(
        &FailingPosition,
        &orientation,
        site,
        ApparentBody::Sun,
        circumstances,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::Position { source, .. } => {
            assert_eq!(source, TestError("position"))
        },
        other => panic!("unexpected circumstance position error: {:?}", other),
    }
    match airless_altitude_circumstances(
        &AnalyticalEphemeris,
        &FailingOrientation,
        site,
        ApparentBody::Sun,
        circumstances,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::EarthOrientation { source, .. } => {
            assert_eq!(source, TestError("orientation"))
        },
        other => panic!("unexpected circumstance orientation error: {:?}", other),
    }
    match airless_altitude_circumstances(
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
        circumstances,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::EarthOrientationIdentityMismatch { .. } => {},
        other => panic!("unexpected circumstance identity error: {:?}", other),
    }
    match airless_altitude_circumstances(
        &WrongEpochPosition,
        &orientation,
        site,
        ApparentBody::Sun,
        circumstances,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::Transform { .. } => {},
        other => panic!("unexpected circumstance transform error: {:?}", other),
    }
}

#[test]
fn meridian_transit_preserves_each_error_boundary() {
    let utc = ScaleAwareEpoch::from_gregorian_utc(2024, 4, 8, 0, 0, 0, 0);
    let start = JulianDate::<TerrestrialTime>::from_epoch(utc);
    let end = start.offset_days(1.0 / 24.0).unwrap();
    let search = MeridianTransitSearch::new(
        SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap(),
    )
    .unwrap();
    let site = observer(0.0, 0.0, 0.0);
    let orientation = constant_orientation(start, utc, 0.0);

    match meridian_transits(
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
        },
        other => panic!("unexpected transit position error: {:?}", other),
    }
    match meridian_transits(
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
        },
        other => panic!("unexpected transit orientation error: {:?}", other),
    }
    match meridian_transits(
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
        },
        other => panic!("unexpected transit identity error: {:?}", other),
    }
    match meridian_transits(
        &WrongEpochPosition,
        &orientation,
        site,
        ApparentBody::Sun,
        search,
    )
    .unwrap_err()
    {
        AltitudeCrossingError::Transform { .. } => {},
        other => panic!("unexpected transit transform error: {:?}", other),
    }
}
