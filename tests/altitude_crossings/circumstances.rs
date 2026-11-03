// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn usno_standard_sun_and_moon_circumstances_use_topocentric_limbs() {
    // USNO Astronomical Applications API v4.0.1, captured 2026-08-25:
    // https://aa.usno.navy.mil/api/rstt/oneday?date=2024-04-08&coords=42.3601,-71.0589&tz=0&dst=false
    // https://aa.usno.navy.mil/api/rstt/oneday?date=2024-04-08&coords=-33.8688,151.2093&tz=0&dst=false
    let fixture = HorizonsAltitudeFixture::parse();
    for &(case, body, longitude, latitude, radius_km, limb_model, expected) in &[
        (
            "boston_sun",
            ApparentBody::Sun,
            -71.0589,
            42.3601,
            695_700.0,
            USNO_STANDARD_SOLAR_LIMB,
            [
                (ConventionalRiseSetKind::Rise, (10, 14)),
                (ConventionalRiseSetKind::Set, (23, 19)),
            ],
        ),
        (
            "sydney_moon",
            ApparentBody::Moon,
            151.2093,
            -33.8688,
            1_737.4,
            MEAN_LUNAR_LIMB,
            [
                (ConventionalRiseSetKind::Set, (7, 20)),
                (ConventionalRiseSetKind::Rise, (20, 24)),
            ],
        ),
    ] {
        let rows = fixture.case_rows(case);
        let first = rows[0];
        let last = rows[rows.len() - 1];
        let window = SearchWindow::new(
            JulianDate::from_julian_day(first.tt_day).unwrap(),
            JulianDate::from_julian_day(last.tt_day).unwrap(),
            1.0 / 24.0,
            1.0 / 86_400.0,
        )
        .unwrap();
        let policy = ConventionalRiseSetPolicy::new(
            RefractionModel::usno_standard(),
            if body == ApparentBody::Sun {
                LimbModel::usno_standard_solar()
            } else {
                LimbModel::upper_physical_radius(
                    Distance::from_kilometers(radius_km).unwrap(),
                    limb_model,
                )
                .unwrap()
            },
            HorizonDipModel::level(),
        );
        let fixture_case = fixture.for_case(case);
        let fixture_events = conventional_rise_set_events(
            &fixture_case,
            &fixture_case,
            observer(longitude, latitude, 0.0),
            body,
            ConventionalRiseSetSearch::new(window).unwrap(),
            policy,
        )
        .expect("USNO conventional fixture events");
        let analytical_eop = fixture.for_case(case);
        let analytical_events = conventional_rise_set_events(
            &AnalyticalEphemeris,
            &analytical_eop,
            observer(longitude, latitude, 0.0),
            body,
            ConventionalRiseSetSearch::new(window).unwrap(),
            policy,
        )
        .expect("USNO conventional analytical events");

        assert_eq!(
            fixture_events.len(),
            expected.len(),
            "{} fixture event count",
            case
        );
        assert_eq!(
            analytical_events.len(),
            expected.len(),
            "{} analytical event count",
            case
        );
        for ((fixture_event, analytical_event), &(kind, (hour, minute))) in fixture_events
            .iter()
            .zip(analytical_events.iter())
            .zip(expected.iter())
        {
            let reference = tt_from_utc(2024, 4, 8, hour, minute);
            let fixture_residual_seconds =
                (fixture_event.interval().midpoint().day() - reference.day()).abs() * 86_400.0;
            let analytical_residual_seconds =
                (analytical_event.interval().midpoint().day() - reference.day()).abs() * 86_400.0;
            eprintln!(
                "{} {:?}: fixture/USNO {:.3} s; analytical/USNO {:.3} s; center {:.5} deg; refraction {:.3} arcmin; limb {:.3} arcmin",
                case,
                fixture_event.kind(),
                fixture_residual_seconds,
                analytical_residual_seconds,
                fixture_event.airless_center_altitude().degrees(),
                fixture_event.refraction_offset().arcseconds() / 60.0,
                fixture_event.limb_offset().arcseconds() / 60.0,
            );
            assert_eq!(fixture_event.kind(), kind);
            assert_eq!(analytical_event.kind(), kind);
            assert!(
                fixture_residual_seconds <= 60.0,
                "{} {:?} fixture residual",
                case,
                kind
            );
            assert!(
                analytical_residual_seconds <= 120.0,
                "{} {:?} analytical residual",
                case,
                kind
            );
            assert_eq!(fixture_event.policy(), policy);
            assert_eq!(
                fixture_event.circumstance_model(),
                CONVENTIONAL_RISE_SET_CIRCUMSTANCES
            );
            assert_eq!(fixture_event.refraction_model(), USNO_STANDARD_REFRACTION);
            assert_eq!(fixture_event.limb_model(), limb_model);
            if body == ApparentBody::Sun {
                assert_eq!(
                    fixture_event.policy().limb().angular_radius(),
                    Some(Angle::from_arcseconds(16.0 * 60.0).unwrap())
                );
            } else {
                assert_eq!(
                    fixture_event.policy().limb().physical_radius(),
                    Some(Distance::from_kilometers(radius_km).unwrap())
                );
            }
            assert_eq!(
                fixture_event.horizon_dip_model(),
                turquet::events::LEVEL_HORIZON_DIP
            );
            assert_eq!(
                fixture_event.refraction_offset(),
                Angle::from_arcseconds(34.0 * 60.0).unwrap()
            );
            assert_eq!(
                fixture_event.horizon_dip_offset(),
                Angle::from_degrees(0.0).unwrap()
            );
            assert!(fixture_event.limb_offset().degrees() > 0.0);
            assert!(fixture_event.interval().width_days() <= 1.0 / 86_400.0);
        }
    }
}

#[test]
fn conventional_policy_models_preserve_airless_parity_and_explicit_offsets() {
    let fixture = HorizonsAltitudeFixture::parse();
    let rows = fixture.case_rows("boston_sun");
    let first = rows[0];
    let last = rows[rows.len() - 1];
    let window = SearchWindow::new(
        JulianDate::from_julian_day(first.tt_day).unwrap(),
        JulianDate::from_julian_day(last.tt_day).unwrap(),
        1.0 / 24.0,
        1.0 / 86_400.0,
    )
    .unwrap();
    let search = ConventionalRiseSetSearch::new(window).unwrap();
    let site = observer(first.longitude, first.latitude, first.height_meters);
    let airless_case = fixture.for_case("boston_sun");
    let airless = airless_rise_set_events(
        &airless_case,
        &airless_case,
        site,
        ApparentBody::Sun,
        AltitudeCrossingSearch::new(window, Angle::from_degrees(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let baseline_policy = ConventionalRiseSetPolicy::new(
        RefractionModel::none(),
        LimbModel::center(),
        HorizonDipModel::level(),
    );
    let baseline_case = fixture.for_case("boston_sun");
    let baseline = conventional_rise_set_events(
        &baseline_case,
        &baseline_case,
        site,
        ApparentBody::Sun,
        search,
        baseline_policy,
    )
    .unwrap();
    assert_eq!(baseline.len(), airless.len());
    for (conventional, airless) in baseline.iter().zip(airless.iter()) {
        assert_eq!(
            conventional.kind(),
            match airless.kind() {
                AirlessRiseSetKind::Rise => ConventionalRiseSetKind::Rise,
                AirlessRiseSetKind::Set => ConventionalRiseSetKind::Set,
            }
        );
        assert_eq!(conventional.interval(), airless.crossing().interval());
        assert_eq!(
            conventional.refraction_offset(),
            Angle::from_degrees(0.0).unwrap()
        );
        assert_eq!(
            conventional.limb_offset(),
            Angle::from_degrees(0.0).unwrap()
        );
        assert_eq!(
            conventional.horizon_dip_offset(),
            Angle::from_degrees(0.0).unwrap()
        );
    }

    let shifted_policy = ConventionalRiseSetPolicy::new(
        RefractionModel::constant(
            Angle::from_degrees(1.0).unwrap(),
            Model::new("test fixed refraction", "1"),
        )
        .unwrap(),
        LimbModel::center(),
        HorizonDipModel::constant(
            Angle::from_degrees(0.5).unwrap(),
            Model::new("test constant horizon dip", "1"),
        )
        .unwrap(),
    );
    let shifted_case = fixture.for_case("boston_sun");
    let shifted = conventional_rise_set_events(
        &shifted_case,
        &shifted_case,
        site,
        ApparentBody::Sun,
        search,
        shifted_policy,
    )
    .unwrap();
    assert_eq!(shifted.len(), 2);
    assert_eq!(shifted[0].kind(), ConventionalRiseSetKind::Rise);
    assert_eq!(shifted[1].kind(), ConventionalRiseSetKind::Set);
    assert!(shifted[0].interval().midpoint().day() < baseline[0].interval().midpoint().day());
    assert!(shifted[1].interval().midpoint().day() > baseline[1].interval().midpoint().day());
    assert_eq!(
        shifted[0].refraction_offset(),
        Angle::from_degrees(1.0).unwrap()
    );
    assert_eq!(
        shifted[0].horizon_dip_offset(),
        Angle::from_degrees(0.5).unwrap()
    );
    assert_eq!(
        shifted[0].policy().horizon_dip().constant_dip(),
        Some(Angle::from_degrees(0.5).unwrap())
    );

    let spherical_radius = Distance::from_kilometers(6_378.137).unwrap();
    let spherical_policy = ConventionalRiseSetPolicy::new(
        RefractionModel::none(),
        LimbModel::center(),
        HorizonDipModel::spherical(spherical_radius, Model::new("test spherical horizon", "1"))
            .unwrap(),
    );
    let spherical_case = fixture.for_case("boston_sun");
    let spherical = conventional_rise_set_events(
        &spherical_case,
        &spherical_case,
        observer(first.longitude, first.latitude, 1_000.0),
        ApparentBody::Sun,
        search,
        spherical_policy,
    )
    .unwrap();
    let expected_dip = (spherical_radius.meters() / (spherical_radius.meters() + 1_000.0))
        .acos()
        .to_degrees();
    assert_eq!(spherical[0].policy(), spherical_policy);
    assert_eq!(
        spherical[0].policy().horizon_dip().spherical_radius(),
        Some(spherical_radius)
    );
    assert!((spherical[0].horizon_dip_offset().degrees() - expected_dip).abs() < 1e-12);

    assert_eq!(
        RefractionModel::constant(Angle::from_degrees(-0.1).unwrap(), Model::new("test", "1")),
        Err(ConventionalRiseSetPolicyError::RefractionOutOfRange)
    );
    assert_eq!(
        RefractionModel::constant(Angle::from_degrees(90.1).unwrap(), Model::new("test", "1")),
        Err(ConventionalRiseSetPolicyError::RefractionOutOfRange)
    );
    assert_eq!(
        LimbModel::upper_angular_radius(Angle::from_degrees(0.0).unwrap(), Model::new("test", "1")),
        Err(ConventionalRiseSetPolicyError::LimbAngularRadiusOutOfRange)
    );
    assert_eq!(
        LimbModel::upper_physical_radius(
            Distance::from_meters(0.0).unwrap(),
            Model::new("test", "1")
        ),
        Err(ConventionalRiseSetPolicyError::LimbRadiusNotPositive)
    );
    assert_eq!(
        HorizonDipModel::constant(Angle::from_degrees(-0.1).unwrap(), Model::new("test", "1")),
        Err(ConventionalRiseSetPolicyError::HorizonDipOutOfRange)
    );
    assert_eq!(
        HorizonDipModel::constant(Angle::from_degrees(90.1).unwrap(), Model::new("test", "1")),
        Err(ConventionalRiseSetPolicyError::HorizonDipOutOfRange)
    );
    assert_eq!(
        HorizonDipModel::spherical(Distance::from_meters(0.0).unwrap(), Model::new("test", "1")),
        Err(ConventionalRiseSetPolicyError::HorizonRadiusNotPositive)
    );

    let tromso_rows = fixture.case_rows("tromso_sun_empty");
    let tromso_first = tromso_rows[0];
    let tromso_last = tromso_rows[tromso_rows.len() - 1];
    let tromso_window = SearchWindow::new(
        JulianDate::from_julian_day(tromso_first.tt_day).unwrap(),
        JulianDate::from_julian_day(tromso_last.tt_day).unwrap(),
        1.0 / 24.0,
        1.0 / 86_400.0,
    )
    .unwrap();
    let tromso_case = fixture.for_case("tromso_sun_empty");
    assert!(
        conventional_rise_set_events(
            &tromso_case,
            &tromso_case,
            observer(
                tromso_first.longitude,
                tromso_first.latitude,
                tromso_first.height_meters,
            ),
            ApparentBody::Sun,
            ConventionalRiseSetSearch::new(tromso_window).unwrap(),
            baseline_policy,
        )
        .unwrap()
        .is_empty()
    );

    let limb_error_case = fixture.for_case("boston_sun");
    match conventional_rise_set_events(
        &limb_error_case,
        &limb_error_case,
        site,
        ApparentBody::Sun,
        search,
        ConventionalRiseSetPolicy::new(
            RefractionModel::none(),
            LimbModel::upper_physical_radius(
                Distance::from_meters(1.0e15).unwrap(),
                Model::new("oversized test limb", "1"),
            )
            .unwrap(),
            HorizonDipModel::level(),
        ),
    ) {
        Err(ConventionalRiseSetError::LimbContainsObserver { .. }) => {},
        other => panic!("expected oversized-limb error, got {:?}", other),
    }

    let below_reference_case = fixture.for_case("boston_sun");
    match conventional_rise_set_events(
        &below_reference_case,
        &below_reference_case,
        observer(first.longitude, first.latitude, -1.0),
        ApparentBody::Sun,
        search,
        ConventionalRiseSetPolicy::new(
            RefractionModel::none(),
            LimbModel::center(),
            HorizonDipModel::spherical(spherical_radius, Model::new("test spherical horizon", "1"))
                .unwrap(),
        ),
    ) {
        Err(ConventionalRiseSetError::ObserverBelowHorizonReference { .. }) => {},
        other => panic!("expected below-reference error, got {:?}", other),
    }
}
