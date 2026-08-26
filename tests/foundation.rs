// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use std::convert::Infallible;

use turquet::apparent::{analytical_accuracy, ApparentBody};
use turquet::foundation::{
    AccuracyEvidence, Angle, Direction, Distance, EastLongitude, Gcrs, JulianDate, Latitude,
    Length, Longitude, Observer, ScaleAwareEpoch, State, TerrestrialTime, TimeOffset,
    TrueEclipticEquinoxOfDate, UnitVector, UniversalTime1, ValueError,
};
use turquet::provider::{AnalyticalEphemeris, GeocentricPositionProvider};

struct UndisclosedAccuracyProvider;

impl GeocentricPositionProvider for UndisclosedAccuracyProvider {
    type Error = Infallible;

    fn model(&self) -> turquet::foundation::Model {
        turquet::foundation::Model::new("undisclosed test provider", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        _epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        unreachable!("the default-accuracy test does not request a position")
    }
}

#[test]
fn physical_values_reject_invalid_scalar_states() {
    assert_eq!(
        Angle::from_radians(f64::NAN),
        Err(ValueError::NotFinite("angle"))
    );
    assert!(matches!(
        Latitude::from_degrees(91.0),
        Err(ValueError::OutOfRange {
            field: "latitude",
            ..
        })
    ));
    assert_eq!(
        Distance::from_meters(-1.0),
        Err(ValueError::Negative("distance"))
    );
    assert_eq!(
        UnitVector::<Gcrs>::new([0.0, 0.0, 0.0]),
        Err(ValueError::ZeroVector)
    );
    assert_eq!(
        TimeOffset::from_seconds(f64::INFINITY),
        Err(ValueError::NotFinite("time offset"))
    );
}

#[test]
fn observed_dut1_constructs_a_distinct_ut1_epoch() {
    let utc = ScaleAwareEpoch::from_gregorian_utc(2024, 4, 8, 18, 0, 0, 0);
    let dut1 = TimeOffset::from_seconds(-0.01669).expect("finite DUT1");
    let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(utc, dut1);
    assert!((ut1.day() - (utc.to_jde_utc_days() + dut1.days())).abs() < 1e-9);
    assert!((ut1.parts().1 - (utc.to_jde_utc_days() - 2_451_545.0 + dut1.days())).abs() < 1e-12);
}

#[test]
fn tt_epoch_round_trips_across_the_2016_leap_second() {
    // A TT Julian scalar preserves the physical instant, but hifitime documents
    // the inserted UTC `:60` spelling as ambiguous. Check both sides and the
    // four elapsed SI seconds instead of requiring one civil-label spelling.
    let cases = [
        ScaleAwareEpoch::from_gregorian_utc(2016, 12, 31, 23, 59, 58, 500_000_000),
        ScaleAwareEpoch::from_gregorian_utc(2017, 1, 1, 0, 0, 1, 500_000_000),
    ];

    let before_tt = JulianDate::<TerrestrialTime>::from_epoch(cases[0]);
    let after_tt = JulianDate::<TerrestrialTime>::from_epoch(cases[1]);
    let elapsed_tt_seconds = (after_tt.day() - before_tt.day()) * 86_400.0;
    assert!((elapsed_tt_seconds - 4.0).abs() < 100_000.0 / 1_000_000_000.0);

    let mut recovered = Vec::new();
    for original in cases.iter() {
        let recovered_epoch = JulianDate::<TerrestrialTime>::from_epoch(*original).to_epoch();
        let error_nanoseconds = (recovered_epoch - *original).total_nanoseconds().abs();
        assert!(
            error_nanoseconds <= 100_000,
            "TT round trip differed by {} ns for {}",
            error_nanoseconds,
            original
        );
        recovered.push(recovered_epoch);
    }

    let recovered_elapsed_nanoseconds = (recovered[1] - recovered[0]).total_nanoseconds();
    assert!((recovered_elapsed_nanoseconds - 4_000_000_000).abs() <= 200_000);
}

#[test]
fn tt_epoch_round_trip_recovers_an_ordinary_utc_label() {
    let original = ScaleAwareEpoch::from_gregorian_utc(2024, 4, 8, 18, 0, 0, 500_000_000);
    let recovered = JulianDate::<TerrestrialTime>::from_epoch(original).to_epoch();
    let original_parts = original.to_gregorian_utc();
    let recovered_parts = recovered.to_gregorian_utc();

    assert_eq!(recovered_parts.0, original_parts.0);
    assert_eq!(recovered_parts.1, original_parts.1);
    assert_eq!(recovered_parts.2, original_parts.2);
    assert_eq!(recovered_parts.3, original_parts.3);
    assert_eq!(recovered_parts.4, original_parts.4);
    assert_eq!(recovered_parts.5, original_parts.5);
    assert!((i64::from(recovered_parts.6) - i64::from(original_parts.6)).abs() <= 100_000);
}

#[test]
fn tt_epoch_conversion_retains_the_small_julian_date_part() {
    let base = JulianDate::<TerrestrialTime>::from_parts(2_451_545.0, 0.0)
        .expect("finite J2000 TT")
        .to_epoch();
    let offset = JulianDate::<TerrestrialTime>::from_parts(2_451_545.0, 1.0e-12)
        .expect("finite two-part TT")
        .to_epoch();

    let offset_nanoseconds = (offset - base).total_nanoseconds();
    assert!((offset_nanoseconds - 86).abs() <= 1);
}

#[test]
fn providers_disclose_accuracy_without_inventing_a_default() {
    assert_eq!(UndisclosedAccuracyProvider.accuracy(), None);

    let accuracy = AnalyticalEphemeris
        .accuracy()
        .expect("the measured analytical provider discloses its accuracy");
    assert_eq!(accuracy, analytical_accuracy());
    assert_eq!(accuracy.evidence(), AccuracyEvidence::ExternalComparison);
    assert_eq!(accuracy.authority(), "NASA/JPL Horizons");
    assert_eq!(accuracy.max_angular_error().degrees(), 0.010);
    assert_eq!(
        accuracy.scope(),
        "5,277 DE440s vectors across 1885-2099; angular only"
    );
}

#[test]
fn longitude_normalizes_and_frame_typed_direction_round_trips() {
    let direction = Direction::<Gcrs>::new(
        Longitude::from_degrees(-10.0).expect("finite longitude"),
        Latitude::from_degrees(30.0).expect("physical latitude"),
    );
    assert!((direction.longitude().degrees() - 350.0).abs() < 1e-12);
    let recovered = direction.to_unit_vector().to_direction();
    assert!((recovered.longitude().degrees() - 350.0).abs() < 1e-12);
    assert!((recovered.latitude().degrees() - 30.0).abs() < 1e-12);
}

#[test]
fn observer_states_the_geodetic_sign_and_height_contract() {
    let observer = Observer::new(
        EastLongitude::from_degrees(-71.0589).expect("Boston west longitude"),
        Latitude::from_degrees(42.3601).expect("Boston latitude"),
        Length::from_meters(43.0).expect("finite ellipsoid height"),
    );
    assert_eq!(observer.longitude().degrees(), -71.0589);
    assert_eq!(observer.latitude().degrees(), 42.3601);
    assert_eq!(observer.height().meters(), 43.0);
}
