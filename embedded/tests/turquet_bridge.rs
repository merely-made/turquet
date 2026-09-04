// SPDX-License-Identifier: MIT

//! Host-side proof that the public observer result feeds the no-std profile.

use turquet::apparent::ApparentBody;
use turquet::foundation::{
    EastLongitude, JulianDate, Latitude, Length, Observer, ScaleAwareEpoch, TerrestrialTime,
    TimeOffset, UniversalTime1,
};
use turquet::observer::{EarthOrientation, ObserverSky};
use turquet_embedded::{solve, UnitVector};

#[test]
fn analytical_observer_sun_feeds_embedded_tracker_geometry() {
    let utc = ScaleAwareEpoch::from_gregorian_utc(2024, 4, 8, 18, 0, 0, 0);
    let tt = JulianDate::<TerrestrialTime>::from_epoch(utc);
    let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
        utc,
        TimeOffset::from_seconds(-0.01669).expect("finite DUT1"),
    );
    let earth_orientation = EarthOrientation::zero_polar_motion(
        ut1,
        "T5c bridge fixture",
        "explicit zero polar motion",
    );
    let observer = Observer::new(
        EastLongitude::from_degrees(-71.0589).expect("Boston longitude"),
        Latitude::from_degrees(42.3601).expect("Boston latitude"),
        Length::from_meters(43.0).expect("Boston height"),
    );

    let observation = ObserverSky::at(tt, earth_orientation, observer)
        .position(ApparentBody::Sun)
        .expect("analytical Sun is supported")
        .into_value();
    let horizon = observation.horizon();
    let sun_components = horizon.to_unit_vector().components();
    let sun = UnitVector::new(sun_components).expect("canonical horizon direction is finite");
    let panel_normal = UnitVector::new([1.0, -2.0, 3.0]).expect("finite panel normal");
    let output = solve(sun, panel_normal);

    // Direction's public conversion is the north/east/up horizon frame.
    let altitude = horizon.latitude().radians();
    let azimuth = horizon.longitude().radians();
    let expected_components = [
        altitude.cos() * azimuth.cos(),
        altitude.cos() * azimuth.sin(),
        altitude.sin(),
    ];
    for (actual, expected) in sun.components().iter().zip(expected_components.iter()) {
        assert!((actual - expected).abs() < 1.0e-14);
    }

    let components = sun.components();
    let panel_components = panel_normal.components();
    let independent_dot = components[0] * panel_components[0]
        + components[1] * panel_components[1]
        + components[2] * panel_components[2];
    assert_eq!(output.desired_sun_direction, sun);
    assert!((output.signed_incidence_cosine - independent_dot).abs() < 1.0e-14);
    assert!((-1.0..=1.0).contains(&output.signed_incidence_cosine));
}
