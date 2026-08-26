// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Local solar-eclipse circumstances against published local-event references.
//!
//! The references below are NASA/GSFC local Besselian circumstances and the
//! USNO Solar Eclipse Computer, captured for fixed historical eclipses. They
//! are independent event oracles, not an ephemeris-state fixture. A committed
//! Horizons state-and-observation fixture adds that separate proof lane.

extern crate turquet;

#[path = "support/local_solar_eclipse_fixture.rs"]
mod fixture;

use std::fmt;

use turquet::apparent::ANALYTICAL_APPARENT;
use turquet::events::{
    local_solar_eclipse_circumstances, AltitudeCrossingError, EventError,
    LocalSolarEclipseContactKind, LocalSolarEclipseError, LocalSolarEclipseKind,
    LocalSolarEclipseSearch, LocalSolarEclipseSearchError, LocalSolarEclipseVisibility,
    SearchWindow, LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCES,
};
use turquet::foundation::{
    EastLongitude, JulianDate, Latitude, Length, Model, Observer, ScaleAwareEpoch, State,
    TerrestrialTime, TimeOffset, TrueEclipticEquinoxOfDate, UniversalTime1,
};
use turquet::observer::{EarthOrientation, AIRLESS_TOPOCENTRIC_TRANSFORM};
use turquet::provider::{
    AnalyticalEphemeris, ConstantOffsetEarthOrientation, EarthOrientationProvider,
    GeocentricPositionProvider,
};

use self::fixture::{
    HorizonsLocalFixture, HORIZONS_EOP_AUTHORITY, HORIZONS_EOP_SNAPSHOT, HORIZONS_MODEL,
    HORIZONS_SNAPSHOT,
};

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    date: (i32, u8, u8),
    longitude: f64,
    latitude: f64,
    height_meters: f64,
    dut1_seconds: f64,
    kind: LocalSolarEclipseKind,
    visibility: LocalSolarEclipseVisibility,
    greatest_utc: (u8, u8, u8),
}

#[test]
fn analytical_local_eclipses_match_three_published_classes() {
    // USNO Solar Eclipse Computer v4.0.1 and NASA GSFC local circumstances:
    // <https://aa.usno.navy.mil/api/eclipses/solar/date?date=2024-4-8&coords=42.3601,-71.0589&height=43>
    // <https://aa.usno.navy.mil/api/eclipses/solar/date?date=2024-4-8&coords=32.7767,-96.7970&height=0>
    // <https://aa.usno.navy.mil/api/eclipses/solar/date?date=2023-10-14&coords=35.0844,-106.6504&height=0>
    let cases = [
        Case {
            name: "Boston partial",
            date: (2024, 4, 8),
            longitude: -71.0589,
            latitude: 42.3601,
            height_meters: 43.0,
            dut1_seconds: -0.01669,
            kind: LocalSolarEclipseKind::Partial,
            visibility: LocalSolarEclipseVisibility::SunUpperLimbAboveAirlessHorizonAtGreatest,
            greatest_utc: (19, 29, 44),
        },
        Case {
            name: "Dallas total",
            date: (2024, 4, 8),
            longitude: -96.7970,
            latitude: 32.7767,
            height_meters: 0.0,
            dut1_seconds: -0.01669,
            kind: LocalSolarEclipseKind::Total,
            visibility: LocalSolarEclipseVisibility::SunUpperLimbAboveAirlessHorizonAtGreatest,
            greatest_utc: (18, 42, 33),
        },
        Case {
            name: "Albuquerque annular",
            date: (2023, 10, 14),
            longitude: -106.6504,
            latitude: 35.0844,
            height_meters: 0.0,
            dut1_seconds: -0.015,
            kind: LocalSolarEclipseKind::Annular,
            visibility: LocalSolarEclipseVisibility::SunUpperLimbAboveAirlessHorizonAtGreatest,
            greatest_utc: (16, 36, 54),
        },
    ];

    for case in &cases {
        let (start, utc) = midnight(case.date);
        let search = local_search(start);
        let orientation = constant_orientation(start, utc, case.dut1_seconds);
        let events = local_solar_eclipse_circumstances(
            &AnalyticalEphemeris,
            &orientation,
            observer(case.longitude, case.latitude, case.height_meters),
            search,
        )
        .expect(case.name);

        assert_eq!(events.len(), 1, "{} event count", case.name);
        let event = &events[0];
        assert_eq!(event.kind(), case.kind, "{} class", case.name);
        assert_eq!(
            event.visibility(),
            case.visibility,
            "{} horizon state",
            case.name
        );
        assert_eq!(event.geometry_model(), LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCES);
        assert_eq!(event.provider_model(), ANALYTICAL_APPARENT);
        assert_eq!(event.provider_snapshot(), None);
        assert_eq!(event.transform_model(), AIRLESS_TOPOCENTRIC_TRANSFORM);
        assert_eq!(event.earth_orientation_authority(), "test EOP");
        assert_eq!(
            event.earth_orientation_snapshot(),
            "constant DUT1 and zero pole"
        );
        assert!(event.greatest_interval().width_days() <= 1.0 / 86_400.0);
        assert!(event
            .contacts()
            .iter()
            .all(|contact| contact.interval().width_days() <= 1.0 / 86_400.0));
        for pair in event.contacts().windows(2) {
            assert!(
                pair[0].interval().end().day() < pair[1].interval().start().day(),
                "{} contacts must be chronological",
                case.name
            );
        }

        let expected_contacts = match case.kind {
            LocalSolarEclipseKind::Partial => [
                LocalSolarEclipseContactKind::First,
                LocalSolarEclipseContactKind::Fourth,
                LocalSolarEclipseContactKind::First,
                LocalSolarEclipseContactKind::Fourth,
            ],
            LocalSolarEclipseKind::Annular | LocalSolarEclipseKind::Total => [
                LocalSolarEclipseContactKind::First,
                LocalSolarEclipseContactKind::Second,
                LocalSolarEclipseContactKind::Third,
                LocalSolarEclipseContactKind::Fourth,
            ],
        };
        let actual_contacts: Vec<LocalSolarEclipseContactKind> = event
            .contacts()
            .iter()
            .map(|contact| contact.kind())
            .collect();
        let expected_contacts = if case.kind == LocalSolarEclipseKind::Partial {
            &expected_contacts[..2]
        } else {
            &expected_contacts[..]
        };
        assert_eq!(
            actual_contacts, expected_contacts,
            "{} contact order",
            case.name
        );

        let (year, month, day) = case.date;
        let (hour, minute, second) = case.greatest_utc;
        let published = utc_tt(year, month, day, hour, minute, second);
        let residual_seconds =
            (event.greatest_interval().midpoint().day() - published.day()).abs() * 86_400.0;
        eprintln!(
            "{} greatest analytical/published residual: {:.3} s",
            case.name, residual_seconds
        );
        assert!(
            residual_seconds <= 600.0,
            "{} greatest residual exceeds the initial analytical ceiling",
            case.name
        );
    }
}

#[test]
fn local_geometry_retains_an_eclipse_after_its_sun_sets() {
    // NASA GSFC gives Galway's 2024 maximum at 19:48:22.9 UTC, with the Sun
    // 4.2 degrees below the geometric horizon. The event remains geometric;
    // this slice deliberately does not suppress contacts by a visibility rule.
    let (start, utc) = midnight((2024, 4, 8));
    let orientation = constant_orientation(start, utc, -0.01669);
    let events = local_solar_eclipse_circumstances(
        &AnalyticalEphemeris,
        &orientation,
        observer(-9.0568, 53.2707, 0.0),
        local_search(start),
    )
    .expect("Galway local eclipse search");

    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.kind(), LocalSolarEclipseKind::Partial);
    assert_eq!(
        event.visibility(),
        LocalSolarEclipseVisibility::SunUpperLimbAtOrBelowAirlessHorizonAtGreatest
    );
    assert_eq!(event.contacts().len(), 2);
    assert!(
        event
            .greatest_geometry()
            .sun_upper_limb_altitude()
            .degrees()
            < 0.0
    );
}

#[test]
fn horizons_fixture_repeats_local_classes_and_direct_horizon_geometry() {
    let fixture = HorizonsLocalFixture::parse();
    let mut worst_altitude_residual = 0.0_f64;
    let cases = [
        (
            "boston_partial",
            Some(LocalSolarEclipseKind::Partial),
            Some(LocalSolarEclipseVisibility::SunUpperLimbAboveAirlessHorizonAtGreatest),
            2,
        ),
        (
            "dallas_total",
            Some(LocalSolarEclipseKind::Total),
            Some(LocalSolarEclipseVisibility::SunUpperLimbAboveAirlessHorizonAtGreatest),
            4,
        ),
        (
            "albuquerque_annular",
            Some(LocalSolarEclipseKind::Annular),
            Some(LocalSolarEclipseVisibility::SunUpperLimbAboveAirlessHorizonAtGreatest),
            4,
        ),
        (
            "galway_partial",
            Some(LocalSolarEclipseKind::Partial),
            Some(LocalSolarEclipseVisibility::SunUpperLimbAtOrBelowAirlessHorizonAtGreatest),
            2,
        ),
        ("cape_town_control", None, None, 0),
    ];

    for &(name, expected_kind, expected_visibility, expected_contacts) in &cases {
        let source = fixture.case(name);
        let search = LocalSolarEclipseSearch::new(source.window(), 0.5).unwrap();
        let events = local_solar_eclipse_circumstances(&source, &source, source.observer(), search)
            .expect(name);
        assert_eq!(
            events.len(),
            usize::from(expected_kind.is_some()),
            "{} event count",
            name
        );
        if let Some(kind) = expected_kind {
            let event = &events[0];
            assert_eq!(event.kind(), kind, "{} class", name);
            assert_eq!(
                event.visibility(),
                expected_visibility.unwrap(),
                "{} horizon state",
                name
            );
            assert_eq!(
                event.contacts().len(),
                expected_contacts,
                "{} contact count",
                name
            );
            for pair in event.contacts().windows(2) {
                assert!(
                    pair[0].interval().end().day() < pair[1].interval().start().day(),
                    "{} fixture contacts must be chronological",
                    name
                );
            }
            assert_eq!(event.provider_model(), HORIZONS_MODEL);
            assert_eq!(event.provider_snapshot(), Some(HORIZONS_SNAPSHOT));
            assert_eq!(event.earth_orientation_authority(), HORIZONS_EOP_AUTHORITY);
            assert_eq!(event.earth_orientation_snapshot(), HORIZONS_EOP_SNAPSHOT);
            let direct_altitude = source.direct_altitude(
                turquet::apparent::ApparentBody::Sun,
                event.greatest_interval().midpoint(),
            );
            let residual =
                (event.greatest_geometry().sun_center_altitude().degrees() - direct_altitude).abs();
            worst_altitude_residual = worst_altitude_residual.max(residual);
            eprintln!(
                "{} direct Horizon altitude residual: {:.5} deg",
                name, residual
            );
            assert!(residual <= 0.05, "{} direct altitude residual", name);
        }
    }
    eprintln!(
        "worst direct Horizons altitude residual: {:.5} deg",
        worst_altitude_residual
    );
}

#[test]
fn outside_the_local_footprint_returns_no_local_circumstances() {
    let (start, utc) = midnight((2024, 4, 8));
    let orientation = constant_orientation(start, utc, -0.01669);
    let events = local_solar_eclipse_circumstances(
        &AnalyticalEphemeris,
        &orientation,
        observer(18.4241, -33.9249, 0.0),
        local_search(start),
    )
    .expect("Cape Town local eclipse search");

    assert!(events.is_empty());
}

#[test]
fn local_search_rejects_ambiguous_circumstance_spans() {
    let (start, _) = midnight((2024, 4, 8));
    let end = start.offset_days(1.0).unwrap();
    let phase_window = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();

    assert_eq!(
        LocalSolarEclipseSearch::new(phase_window, f64::NAN).unwrap_err(),
        LocalSolarEclipseSearchError::SpanNotFinite
    );
    assert_eq!(
        LocalSolarEclipseSearch::new(phase_window, 0.0).unwrap_err(),
        LocalSolarEclipseSearchError::SpanNotPositive
    );
    assert_eq!(
        LocalSolarEclipseSearch::new(phase_window, 1.001).unwrap_err(),
        LocalSolarEclipseSearchError::SpanTooLarge
    );
    assert_eq!(
        LocalSolarEclipseSearch::new(phase_window, 2.0 / 86_400.0).unwrap_err(),
        LocalSolarEclipseSearchError::ToleranceExceedsHalfSpan
    );
}

#[test]
fn local_search_reports_when_its_contact_span_is_too_short() {
    let (start, utc) = midnight((2024, 4, 8));
    let orientation = constant_orientation(start, utc, -0.01669);
    let short_search =
        LocalSolarEclipseSearch::new(local_search(start).phase_window(), 0.1).unwrap();

    match local_solar_eclipse_circumstances(
        &AnalyticalEphemeris,
        &orientation,
        observer(-71.0589, 42.3601, 43.0),
        short_search,
    )
    .unwrap_err()
    {
        LocalSolarEclipseError::CircumstanceSpanTooShort { span_days, .. } => {
            assert_eq!(span_days, 0.1)
        }
        other => panic!("unexpected short-span error: {:?}", other),
    }
}

#[test]
fn local_circumstances_preserve_observer_error_boundaries() {
    let (start, _) = midnight((2024, 4, 8));
    let site = observer(-71.0589, 42.3601, 43.0);
    let (_, utc) = midnight((2024, 4, 8));
    let orientation = constant_orientation(start, utc, -0.01669);

    match local_solar_eclipse_circumstances(
        &FailingPosition,
        &orientation,
        site,
        local_search(start),
    )
    .unwrap_err()
    {
        LocalSolarEclipseError::Phase(EventError::Position { source, .. }) => {
            assert_eq!(source, TestError("position"))
        }
        other => panic!("unexpected local-eclipse position error: {:?}", other),
    }

    match local_solar_eclipse_circumstances(
        &AnalyticalEphemeris,
        &FailingOrientation,
        site,
        local_search(start),
    )
    .unwrap_err()
    {
        LocalSolarEclipseError::Observation(AltitudeCrossingError::EarthOrientation {
            source,
            ..
        }) => assert_eq!(source, TestError("orientation")),
        other => panic!("unexpected local-eclipse EOP error: {:?}", other),
    }

    match local_solar_eclipse_circumstances(
        &AnalyticalEphemeris,
        &WrongIdentityOrientation,
        site,
        local_search(start),
    )
    .unwrap_err()
    {
        LocalSolarEclipseError::Observation(
            AltitudeCrossingError::EarthOrientationIdentityMismatch {
                expected_authority,
                actual_authority,
                ..
            },
        ) => {
            assert_eq!(expected_authority, "declared");
            assert_eq!(actual_authority, "returned");
        }
        other => panic!("unexpected local-eclipse EOP identity error: {:?}", other),
    }

    match local_solar_eclipse_circumstances(
        &WrongEpochPosition,
        &orientation,
        site,
        local_search(start),
    )
    .unwrap_err()
    {
        LocalSolarEclipseError::Observation(AltitudeCrossingError::Transform { .. }) => {}
        other => panic!("unexpected local-eclipse transform error: {:?}", other),
    }
}

fn local_search(start: JulianDate<TerrestrialTime>) -> LocalSolarEclipseSearch {
    let end = start.offset_days(1.0).unwrap();
    let phase_window = SearchWindow::new(start, end, 1.0 / 24.0, 1.0 / 86_400.0).unwrap();
    LocalSolarEclipseSearch::new(phase_window, 0.5).unwrap()
}

fn midnight(date: (i32, u8, u8)) -> (JulianDate<TerrestrialTime>, hifitime::Epoch) {
    let (year, month, day) = date;
    let utc = ScaleAwareEpoch::from_gregorian_utc(year, month, day, 0, 0, 0, 0);
    (JulianDate::from_epoch(utc), utc)
}

fn utc_tt(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
) -> JulianDate<TerrestrialTime> {
    JulianDate::from_epoch(ScaleAwareEpoch::from_gregorian_utc(
        year, month, day, hour, minute, second, 0,
    ))
}

fn observer(longitude: f64, latitude: f64, height_meters: f64) -> Observer {
    Observer::new(
        EastLongitude::from_degrees(longitude).unwrap(),
        Latitude::from_degrees(latitude).unwrap(),
        Length::from_meters(height_meters).unwrap(),
    )
}

fn constant_orientation(
    tt: JulianDate<TerrestrialTime>,
    utc: hifitime::Epoch,
    dut1_seconds: f64,
) -> ConstantOffsetEarthOrientation {
    let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
        utc,
        TimeOffset::from_seconds(dut1_seconds).unwrap(),
    );
    ConstantOffsetEarthOrientation::new(
        tt,
        EarthOrientation::zero_polar_motion(ut1, "test EOP", "constant DUT1 and zero pole"),
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestError(&'static str);

impl fmt::Display for TestError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl ::std::error::Error for TestError {}

struct FailingOrientation;

impl EarthOrientationProvider for FailingOrientation {
    type Error = TestError;

    fn authority(&self) -> &str {
        "failing EOP"
    }

    fn data_snapshot(&self) -> &str {
        "failing snapshot"
    }

    fn at(&self, _: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        Err(TestError("orientation"))
    }
}

struct WrongIdentityOrientation;

impl EarthOrientationProvider for WrongIdentityOrientation {
    type Error = TestError;

    fn authority(&self) -> &str {
        "declared"
    }

    fn data_snapshot(&self) -> &str {
        "declared snapshot"
    }

    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        Ok(EarthOrientation::zero_polar_motion(
            JulianDate::<UniversalTime1>::from_julian_day(epoch.day()).unwrap(),
            "returned",
            "returned snapshot",
        ))
    }
}

struct WrongEpochPosition;

struct FailingPosition;

impl GeocentricPositionProvider for FailingPosition {
    type Error = TestError;

    fn model(&self) -> Model {
        Model::new("failing position", "1")
    }

    fn position(
        &self,
        _: turquet::apparent::ApparentBody,
        _: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        Err(TestError("position"))
    }
}

impl GeocentricPositionProvider for WrongEpochPosition {
    type Error = turquet::apparent::ApparentError;

    fn model(&self) -> Model {
        ANALYTICAL_APPARENT
    }

    fn position(
        &self,
        body: turquet::apparent::ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let state = AnalyticalEphemeris.position(body, epoch)?;
        Ok(State::new(
            epoch.offset_days(1.0).unwrap(),
            state.direction(),
            state.distance(),
        ))
    }
}
