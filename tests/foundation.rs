// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use turquet::foundation::{
    Angle, Direction, Distance, EastLongitude, Gcrs, JulianDate, Latitude, Length, Longitude,
    Observer, ScaleAwareEpoch, TimeOffset, UnitVector, UniversalTime1, ValueError,
};

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
