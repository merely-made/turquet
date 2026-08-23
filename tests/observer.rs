// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Airless topocentric comparison with NASA/JPL Horizons. The fixture is
//! quantity 2 (apparent RA/Dec), quantity 4 (azimuth/elevation), quantity 20
//! (range), and quantity 49 (DUT1) for a user-defined WGS84 site.

extern crate turquet;

use std::collections::BTreeMap;

use turquet::apparent::ApparentBody;
use turquet::foundation::{
    EastLongitude, JulianDate, Latitude, Length, Observer, ScaleAwareEpoch, TerrestrialTime,
    TimeOffset, UniversalTime1,
};
use turquet::observer::{position, EarthOrientation, ObserverSky, ANALYTICAL_TOPOCENTRIC};

const VECTORS: &str = include_str!("vectors/observer_horizons.tsv");

#[test]
fn boston_eclipse_moon_matches_horizons() {
    let utc = ScaleAwareEpoch::from_gregorian_utc(2024, 4, 8, 18, 0, 0, 0);
    let tt = JulianDate::<TerrestrialTime>::from_epoch(utc);
    let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
        utc,
        TimeOffset::from_seconds(-0.01669).expect("finite DUT1"),
    );
    let earth_orientation = EarthOrientation::zero_polar_motion(
        ut1,
        "NASA/JPL Horizons quantity 49",
        "eop.260821.p261117; polar motion approximated as zero",
    );
    let observer = Observer::new(
        EastLongitude::from_degrees(-71.0589).expect("Boston longitude"),
        Latitude::from_degrees(42.3601).expect("Boston latitude"),
        Length::from_meters(43.0).expect("Boston height"),
    );

    let result = position(ApparentBody::Moon, tt, earth_orientation, observer)
        .expect("Moon is supported at the eclipse");
    let observation = result.value();
    let equatorial = observation.equatorial().direction();
    let horizon = observation.horizon();

    assert_angle_close(equatorial.longitude().degrees(), 17.329049221, 0.010);
    assert_angle_close(equatorial.latitude().degrees(), 7.228875705, 0.010);
    assert_angle_close(horizon.longitude().degrees(), 211.069165665, 0.010);
    assert_angle_close(horizon.latitude().degrees(), 51.032060943, 0.010);
    assert!(
        (observation.equatorial().distance().astronomical_units() - 0.00237166053612).abs() < 1e-7
    );
    assert_eq!(result.model(), ANALYTICAL_TOPOCENTRIC);
    assert_eq!(
        observation.earth_orientation().snapshot(),
        "eop.260821.p261117; polar motion approximated as zero"
    );
}

#[test]
fn observer_cohort_matches_horizons() {
    for expected in &[
        "oracle: NASA/JPL Horizons API, DE441",
        "EOP eop.260821.p261117",
        "polar motion: Horizons applies its EOP pole",
    ] {
        assert!(
            VECTORS.lines().any(|line| line.contains(expected)),
            "observer vector header must record {}",
            expected
        );
    }

    let mut skies = BTreeMap::new();
    let mut compared = 0_usize;
    let mut worst_angle = (0.0_f64, String::new());
    let mut worst_range = (0.0_f64, String::new());

    for line in VECTORS.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 11, "observer vector column count");
        let site = fields[0];
        let coordinates: Vec<f64> = fields[1]
            .split(',')
            .map(|field| field.parse().expect("site coordinate"))
            .collect();
        let utc_label = fields[2];
        let julian_utc: f64 = fields[3].parse().expect("UTC Julian day");
        let body = body_named(fields[4]).expect("known apparent body");
        let expected_ra: f64 = fields[5].parse().expect("right ascension");
        let expected_dec: f64 = fields[6].parse().expect("declination");
        let expected_azimuth: f64 = fields[7].parse().expect("azimuth");
        let expected_altitude: f64 = fields[8].parse().expect("altitude");
        let expected_range: f64 = fields[9].parse().expect("range");
        let dut1_seconds: f64 = fields[10].parse().expect("DUT1");

        let key = format!("{}:{}", site, fields[3]);
        if !skies.contains_key(&key) {
            let utc = ScaleAwareEpoch::from_jde_utc(julian_utc);
            let tt = JulianDate::<TerrestrialTime>::from_epoch(utc);
            let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
                utc,
                TimeOffset::from_seconds(dut1_seconds).expect("finite DUT1"),
            );
            let earth_orientation = EarthOrientation::zero_polar_motion(
                ut1,
                "NASA/JPL Horizons quantity 49",
                "eop.260821.p261117; polar motion approximated as zero",
            );
            let observer = Observer::new(
                EastLongitude::from_degrees(coordinates[0]).expect("site longitude"),
                Latitude::from_degrees(coordinates[1]).expect("site latitude"),
                Length::from_meters(coordinates[2] * 1_000.0).expect("site height"),
            );
            skies.insert(
                key.clone(),
                ObserverSky::at(tt, earth_orientation, observer),
            );
        }

        let result = skies[&key]
            .position(body)
            .unwrap_or_else(|_| panic!("{} {} is supported", site, utc_label));
        let observation = result.value();
        let equatorial = observation.equatorial().direction();
        let horizon = observation.horizon();
        let residuals = [
            circular_degrees(equatorial.longitude().degrees(), expected_ra),
            equatorial.latitude().degrees() - expected_dec,
            circular_degrees(horizon.longitude().degrees(), expected_azimuth),
            horizon.latitude().degrees() - expected_altitude,
        ];
        for residual in residuals.iter() {
            if residual.abs() > worst_angle.0 {
                worst_angle = (
                    residual.abs(),
                    format!("{} {} {}", site, utc_label, fields[4]),
                );
            }
        }
        let range_residual =
            (observation.equatorial().distance().astronomical_units() - expected_range).abs();
        if range_residual > worst_range.0 {
            worst_range = (
                range_residual,
                format!("{} {} {}", site, utc_label, fields[4]),
            );
        }
        compared += 1;
    }

    eprintln!(
        "observer cohort: {} vectors; worst angle {:.6} deg at {}; worst range {:.9} AU at {}",
        compared, worst_angle.0, worst_angle.1, worst_range.0, worst_range.1
    );
    assert_eq!(compared, 90);
    assert!(
        worst_angle.0 <= 0.010,
        "worst observer angular residual {:.6} deg at {}",
        worst_angle.0,
        worst_angle.1
    );
    assert!(
        worst_range.0 <= 0.001,
        "worst observer range residual {:.9} AU at {}",
        worst_range.0,
        worst_range.1
    );
}

fn body_named(name: &str) -> Option<ApparentBody> {
    use turquet::apparent::APPARENT_BODIES;
    APPARENT_BODIES
        .iter()
        .find(|body| body.name() == name)
        .copied()
}

fn circular_degrees(actual: f64, expected: f64) -> f64 {
    (actual - expected + 180.0).rem_euclid(360.0) - 180.0
}

fn assert_angle_close(actual: f64, expected: f64, tolerance_degrees: f64) {
    let residual = (actual - expected + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        residual.abs() <= tolerance_degrees,
        "expected {:.9} deg, got {:.9} deg, residual {:.6} deg",
        expected,
        actual,
        residual
    );
}
