extern crate turquet;
use turquet::observer::EarthOrientation;
use turquet::{foundation::*, houses::*};

include!("vectors/placidus_swiss.inc");

fn placidus_at(tt: f64, ut1: f64, lat: f64, lon: f64) -> Result<Houses, HouseError> {
    calculate(
        JulianDate::<TerrestrialTime>::from_julian_day(tt).unwrap(),
        JulianDate::<UniversalTime1>::from_julian_day(ut1).unwrap(),
        Latitude::from_degrees(lat).unwrap(),
        EastLongitude::from_degrees(lon).unwrap(),
        HouseSystem::Placidus,
    )
}

#[test]
fn independent_swiss_placidus_vectors_and_assignment() {
    let mut worst = 0.0_f64;
    for &(tt, ut1, lat, lon, asc, mc, cusps) in PLACIDUS_SWISS {
        let h = placidus_at(tt, ut1, lat, lon).unwrap();
        assert_eq!(h.system(), HouseSystem::Placidus);
        assert_eq!(h.model(), PLACIDUS_HOUSE_GEOMETRY);
        assert_eq!(h.model().revision(), "2");
        assert_ne!(h.model(), HOUSE_GEOMETRY);
        let close = |actual: f64, expected: f64| {
            let error = ((actual - expected + 180.0).rem_euclid(360.0) - 180.0).abs();
            assert!(
                error < 0.002,
                "JD {}, lat {}, lon {}: {} vs {} ({} deg)",
                ut1,
                lat,
                lon,
                actual,
                expected,
                error
            );
            error
        };
        worst = worst.max(close(h.ascendant().degrees(), asc));
        worst = worst.max(close(h.midheaven().degrees(), mc));
        assert_eq!(h.cusps()[0], h.ascendant());
        assert_eq!(h.cusps()[9], h.midheaven());
        let mut total_width = 0.0;
        for i in 0..12 {
            let cusp = h.cusps()[i];
            worst = worst.max(close(cusp.degrees(), cusps[i]));
            let width = (h.cusps()[(i + 1) % 12].degrees() - cusp.degrees()).rem_euclid(360.0);
            assert!(width > 0.0 && width < 180.0);
            total_width += width;
            assert_eq!(h.house_of(cusp), i as u8 + 1);
            for fraction in [1e-5, 0.5, 1.0 - 1e-5] {
                assert_eq!(
                    h.house_of(Longitude::from_degrees(cusp.degrees() + fraction * width).unwrap()),
                    i as u8 + 1
                );
            }
            assert_eq!(
                h.house_of(Longitude::from_degrees(cusp.degrees() - 1e-7).unwrap()),
                ((i + 11) % 12) as u8 + 1
            );
            let opposite = (h.cusps()[(i + 6) % 12].degrees() - cusp.degrees()).rem_euclid(360.0);
            assert!((opposite - 180.0).abs() < 1e-10);
        }
        assert!((total_width - 360.0).abs() < 1e-10);
    }
    eprintln!(
        "{} independent Placidus cases; worst angular error {worst:.12} deg",
        PLACIDUS_SWISS.len()
    );
}

#[test]
fn swiss_placidus_errors_are_explicit_without_fallback() {
    for &(tt, ut1, lat, lon) in PLACIDUS_UNAVAILABLE {
        assert_eq!(
            placidus_at(tt, ut1, lat, lon),
            Err(HouseError::PlacidusUnavailable)
        );
        if lat.abs() < 90.0 {
            for system in [HouseSystem::Equal, HouseSystem::WholeSign] {
                assert!(
                    calculate(
                        JulianDate::<TerrestrialTime>::from_julian_day(tt).unwrap(),
                        JulianDate::<UniversalTime1>::from_julian_day(ut1).unwrap(),
                        Latitude::from_degrees(lat).unwrap(),
                        EastLongitude::from_degrees(lon).unwrap(),
                        system,
                    )
                    .is_ok()
                );
            }
        }
    }
    assert!(
        HouseError::PlacidusUnavailable
            .to_string()
            .contains("Placidus")
    );
}

#[test]
fn observed_zero_polar_motion_agrees_with_legacy_geometry() {
    for &(tt, ut1, lat, lon, _, _, _) in PLACIDUS_SWISS
        .iter()
        .take(7)
        .chain(PLACIDUS_SWISS.iter().skip(7).step_by(21))
    {
        let tt = JulianDate::<TerrestrialTime>::from_julian_day(tt).unwrap();
        let ut1 = JulianDate::<UniversalTime1>::from_julian_day(ut1).unwrap();
        let eop = EarthOrientation::zero_polar_motion(ut1, "explicit approximation", "test");
        for system in [
            HouseSystem::Equal,
            HouseSystem::WholeSign,
            HouseSystem::Placidus,
        ] {
            let lat = Latitude::from_degrees(lat).unwrap();
            let lon = EastLongitude::from_degrees(lon).unwrap();
            let legacy = calculate(tt, ut1, lat, lon, system).unwrap();
            let observed = calculate_observed(tt, &eop, lat, lon, system).unwrap();
            assert_eq!(observed.model(), OBSERVED_HOUSE_GEOMETRY);
            assert_eq!(observed.model().revision(), "2");
            assert_ne!(observed.model(), legacy.model());
            assert_ne!(observed.model(), PLACIDUS_HOUSE_GEOMETRY);
            // pom00 includes the tiny TIO locator s' even with zero xp/yp;
            // its effect is amplified for narrow near-polar houses.
            for i in 0..12 {
                let error = (observed.cusps()[i].radians() - legacy.cusps()[i].radians()
                    + std::f64::consts::PI)
                    .rem_euclid(2.0 * std::f64::consts::PI)
                    - std::f64::consts::PI;
                assert!(error.abs() < 1e-6, "{:?}: {}", system, error);
            }
        }
    }
}

#[test]
fn observed_zero_at_j2000_and_corrected_polar_boundary() {
    let tt = JulianDate::<TerrestrialTime>::from_julian_day(2451545.0).unwrap();
    let ut1 = JulianDate::<UniversalTime1>::from_julian_day(2451545.0).unwrap();
    let zero = EarthOrientation::zero_polar_motion(ut1, "test", "zero");
    let lon = EastLongitude::from_degrees(0.0).unwrap();
    for lat in [-60.0, 0.0, 60.0] {
        let lat = Latitude::from_degrees(lat).unwrap();
        for system in [
            HouseSystem::Equal,
            HouseSystem::WholeSign,
            HouseSystem::Placidus,
        ] {
            let a = calculate(tt, ut1, lat, lon, system).unwrap();
            let b = calculate_observed(tt, &zero, lat, lon, system).unwrap();
            for (a, b) in a.cusps().iter().zip(b.cusps()) {
                assert!((a.radians() - b.radians()).abs() < 1e-12);
            }
        }
    }
    // Magnified motion deliberately moves the zenith across the polar circle
    // in both directions. Availability follows the rotated horizon latitude.
    for (lat, xp, available) in [(66.55, 0.001, false), (66.57, -0.001, true)] {
        let lat = Latitude::from_degrees(lat).unwrap();
        let eop = EarthOrientation::new(
            ut1,
            Angle::from_radians(xp).unwrap(),
            Angle::from_radians(0.0).unwrap(),
            "synthetic test",
            "boundary",
        );
        let result = calculate_observed(tt, &eop, lat, lon, HouseSystem::Placidus);
        assert_eq!(result.is_ok(), available);
        if !available {
            assert_eq!(result, Err(HouseError::PlacidusUnavailable));
        }
        assert_eq!(
            calculate(tt, ut1, lat, lon, HouseSystem::Placidus).is_ok(),
            !available
        );
    }
}

#[test]
fn observed_nonzero_polar_motion_rotates_zenith_and_preserves_eop() {
    for &(tt, ut1, lat, lon, xp, yp, corrected_lat, corrected_lon, asc, mc, cusps) in
        PLACIDUS_OBSERVED
    {
        let tt = JulianDate::<TerrestrialTime>::from_julian_day(tt).unwrap();
        let ut1 = JulianDate::<UniversalTime1>::from_julian_day(ut1).unwrap();
        let latitude = Latitude::from_degrees(lat).unwrap();
        let longitude = EastLongitude::from_degrees(lon).unwrap();
        let eop = EarthOrientation::new(
            ut1,
            Angle::from_radians(xp).unwrap(),
            Angle::from_radians(yp).unwrap(),
            "caller",
            "immutable-test",
        );
        let snapshot = eop.clone();
        // Frozen numerical coordinates come from independent Python scalar
        // rotations, without calling sofars or the Rust implementation.
        assert!((corrected_lat - lat).abs() > 1e-6);
        assert!((corrected_lon - lon).abs() > 1e-6);
        let corrected_latitude = Latitude::from_degrees(corrected_lat).unwrap();
        let corrected_longitude = EastLongitude::from_degrees(corrected_lon).unwrap();
        for system in [
            HouseSystem::Equal,
            HouseSystem::WholeSign,
            HouseSystem::Placidus,
        ] {
            let expected =
                calculate(tt, ut1, corrected_latitude, corrected_longitude, system).unwrap();
            let actual = calculate_observed(tt, &eop, latitude, longitude, system).unwrap();
            for (a, b) in actual.cusps().iter().zip(expected.cusps()) {
                assert!((a.radians() - b.radians()).abs() < 1e-11);
            }
            assert!((actual.ascendant().radians() - expected.ascendant().radians()).abs() < 1e-11);
            assert!((actual.midheaven().radians() - expected.midheaven().radians()).abs() < 1e-11);
            if xp.abs().max(yp.abs()) > 0.01 && system != HouseSystem::WholeSign {
                let uncorrected = calculate(tt, ut1, latitude, longitude, system).unwrap();
                assert!(
                    (actual.ascendant().radians() - uncorrected.ascendant().radians()).abs() > 1e-6
                );
            }
            if system == HouseSystem::Placidus {
                let close = |a: f64, b: f64| {
                    assert!(
                        ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs() < 0.002,
                        "{} vs {}",
                        a,
                        b
                    )
                };
                close(actual.ascendant().degrees(), asc);
                close(actual.midheaven().degrees(), mc);
                for i in 0..12 {
                    close(actual.cusps()[i].degrees(), cusps[i]);
                }
                // Omitting either angular correction must fail the reference.
                for (test_lat, test_lon) in [
                    (latitude, corrected_longitude),
                    (corrected_latitude, longitude),
                ] {
                    let incomplete = calculate(tt, ut1, test_lat, test_lon, system).unwrap();
                    assert!(
                        (incomplete.ascendant().radians() - actual.ascendant().radians()).abs()
                            > 1e-9
                    );
                }
            }
        }
        assert_eq!(eop, snapshot);
    }
}
#[test]
fn independent_swiss_tropical_vectors() {
    let rows: &[(f64, f64, f64, f64, HouseSystem, f64, f64, [f64; 12])] = &[
        (
            2451545.0007387605,
            2451545.0,
            40.7128,
            -74.006,
            HouseSystem::Equal,
            274.24198357024164,
            208.46900977849552,
            [
                274.24198357024164,
                304.24198357024164,
                334.24198357024164,
                4.2419835702417,
                34.2419835702417,
                64.2419835702417,
                94.24198357024164,
                124.24198357024164,
                154.2419835702417,
                184.2419835702417,
                214.2419835702417,
                244.2419835702417,
            ],
        ),
        (
            2451545.0007387605,
            2451545.0,
            40.7128,
            -74.006,
            HouseSystem::WholeSign,
            274.24198357024164,
            208.46900977849552,
            [
                270.0, 300.0, 330.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0,
            ],
        ),
        (
            2460482.500799242,
            2460482.5,
            -33.8688,
            151.2093,
            HouseSystem::Equal,
            137.70220517059204,
            62.940790351420326,
            [
                137.70220517059204,
                167.70220517059204,
                197.70220517059204,
                227.70220517059204,
                257.70220517059204,
                287.70220517059204,
                317.70220517059204,
                347.70220517059204,
                17.702205170592038,
                47.70220517059204,
                77.70220517059204,
                107.70220517059204,
            ],
        ),
        (
            2460482.500799242,
            2460482.5,
            -33.8688,
            151.2093,
            HouseSystem::WholeSign,
            137.70220517059204,
            62.940790351420326,
            [
                120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0, 0.0, 30.0, 60.0, 90.0,
            ],
        ),
        (
            2460666.2507986426,
            2460666.25,
            51.4779,
            0.0,
            HouseSystem::Equal,
            117.1335915700547,
            0.8681816896941698,
            [
                117.1335915700547,
                147.1335915700547,
                177.1335915700547,
                207.1335915700547,
                237.1335915700547,
                267.1335915700547,
                297.1335915700547,
                327.1335915700547,
                357.1335915700547,
                27.133591570054705,
                57.133591570054705,
                87.1335915700547,
            ],
        ),
        (
            2460666.2507986426,
            2460666.25,
            51.4779,
            0.0,
            HouseSystem::WholeSign,
            117.1335915700547,
            0.8681816896941698,
            [
                90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0, 0.0, 30.0, 60.0,
            ],
        ),
        (
            2446167.7506307173,
            2446167.75,
            -34.6,
            -58.38,
            HouseSystem::Equal,
            328.31878155380815,
            234.37200409402257,
            [
                328.31878155380815,
                358.31878155380815,
                28.31878155380815,
                58.318781553808094,
                88.3187815538081,
                118.3187815538081,
                148.31878155380815,
                178.3187815538081,
                208.31878155380815,
                238.3187815538081,
                268.3187815538081,
                298.3187815538081,
            ],
        ),
        (
            2446167.7506307173,
            2446167.75,
            -34.6,
            -58.38,
            HouseSystem::WholeSign,
            328.31878155380815,
            234.37200409402257,
            [
                300.0, 330.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0,
            ],
        ),
        (
            2451544.500738756,
            2451544.5,
            0.0,
            0.0,
            HouseSystem::Equal,
            190.83991267628437,
            99.15666573909988,
            [
                190.83991267628437,
                220.83991267628437,
                250.83991267628437,
                280.83991267628437,
                310.83991267628437,
                340.83991267628437,
                10.83991267628437,
                40.83991267628437,
                70.83991267628437,
                100.83991267628437,
                130.83991267628437,
                160.83991267628437,
            ],
        ),
        (
            2451544.500738756,
            2451544.5,
            0.0,
            0.0,
            HouseSystem::WholeSign,
            190.83991267628437,
            99.15666573909988,
            [
                180.0, 210.0, 240.0, 270.0, 300.0, 330.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0,
            ],
        ),
        (
            2451544.750738758,
            2451544.75,
            0.0,
            0.0,
            HouseSystem::Equal,
            279.38384942556655,
            191.10694329846874,
            [
                279.38384942556655,
                309.38384942556655,
                339.38384942556655,
                9.383849425566495,
                39.383849425566495,
                69.3838494255665,
                99.38384942556655,
                129.38384942556655,
                159.3838494255665,
                189.3838494255665,
                219.3838494255665,
                249.3838494255665,
            ],
        ),
        (
            2451544.750738758,
            2451544.75,
            0.0,
            0.0,
            HouseSystem::WholeSign,
            279.38384942556655,
            191.10694329846874,
            [
                270.0, 300.0, 330.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0,
            ],
        ),
        (
            2451545.250738763,
            2451545.25,
            0.0,
            0.0,
            HouseSystem::Equal,
            99.83838214933792,
            11.640779924935458,
            [
                99.83838214933792,
                129.83838214933792,
                159.83838214933792,
                189.83838214933792,
                219.83838214933792,
                249.83838214933792,
                279.8383821493379,
                309.8383821493379,
                339.8383821493379,
                9.838382149337917,
                39.83838214933792,
                69.83838214933792,
            ],
        ),
        (
            2451545.250738763,
            2451545.25,
            0.0,
            0.0,
            HouseSystem::WholeSign,
            99.83838214933792,
            11.640779924935458,
            [
                90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0, 0.0, 30.0, 60.0,
            ],
        ),
        (
            2460389.500799532,
            2460389.5,
            60.0,
            179.99,
            HouseSystem::Equal,
            123.32307337711076,
            357.8286249441161,
            [
                123.32307337711076,
                153.32307337711075,
                183.32307337711075,
                213.32307337711075,
                243.32307337711075,
                273.32307337711075,
                303.32307337711075,
                333.32307337711075,
                3.3230733771107452,
                33.323073377110745,
                63.323073377110745,
                93.32307337711075,
            ],
        ),
        (
            2460389.500799532,
            2460389.5,
            60.0,
            179.99,
            HouseSystem::WholeSign,
            123.32307337711076,
            357.8286249441161,
            [
                120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0, 0.0, 30.0, 60.0, 90.0,
            ],
        ),
        (
            2460390.00079953,
            2460390.0,
            -60.0,
            -179.99,
            HouseSystem::Equal,
            303.6432312236459,
            178.38747133132472,
            [
                303.6432312236459,
                333.6432312236459,
                3.6432312236458984,
                33.6432312236459,
                63.6432312236459,
                93.6432312236459,
                123.6432312236459,
                153.6432312236459,
                183.6432312236459,
                213.6432312236459,
                243.6432312236459,
                273.6432312236459,
            ],
        ),
        (
            2460390.00079953,
            2460390.0,
            -60.0,
            -179.99,
            HouseSystem::WholeSign,
            303.6432312236459,
            178.38747133132472,
            [
                300.0, 330.0, 0.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0,
            ],
        ),
    ];
    let close = |a: f64, b: f64| {
        assert!(
            ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs() < 0.002,
            "{} vs {}",
            a,
            b
        )
    };
    for &(tt, ut1, lat, lon, system, asc, mc, cusps) in rows {
        let h = calculate(
            JulianDate::<TerrestrialTime>::from_julian_day(tt).unwrap(),
            JulianDate::<UniversalTime1>::from_julian_day(ut1).unwrap(),
            Latitude::from_degrees(lat).unwrap(),
            EastLongitude::from_degrees(lon).unwrap(),
            system,
        )
        .unwrap();
        assert_eq!(h.model(), HOUSE_GEOMETRY);
        assert_eq!(h.model().revision(), "1");
        close(h.ascendant().degrees(), asc);
        close(h.midheaven().degrees(), mc);
        for i in 0..12 {
            close(h.cusps()[i].degrees(), cusps[i]);
            assert_eq!(h.house_of(h.cusps()[i]), i as u8 + 1);
            close(
                h.cusps()[(i + 1) % 12].degrees(),
                h.cusps()[i].degrees() + 30.0,
            );
        }
    }
}
