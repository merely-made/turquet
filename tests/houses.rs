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

#[path = "houses/tropical.rs"]
mod tropical;
