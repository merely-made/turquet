// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Measures the apparent pipeline against NASA/JPL Horizons observer-table
//! quantity 31: geocentric apparent ecliptic-of-date longitude and latitude,
//! including light time and stellar aberration, at three instants. Values are
//! compared in millidegrees because that is the precision the first consumer
//! stores; the assertion ceiling is a measured bound, not an accuracy claim.

extern crate turquet;

use turquet::apparent::{
    geocent_apparent_ecl_pos, is_retrograde, jde_tt_frm_epoch, jde_tt_frm_utc, ApparentBody,
    ApparentError, Epoch, APPARENT_BODIES,
};

/// Worst allowed residual. Measured: the Sun, Moon, and eight planets land on
/// the Horizons millidegree exactly or within 2; Pluto's truncated series is
/// the limiting term at 14.
const MAX_ERROR_MILLIDEGREES: i64 = 20;

struct Golden {
    label: &'static str,
    utc: (i32, u32, u32, u32, u32, f64),
    positions: [(&'static str, i64, i64); 10],
}

const CHARTS: [Golden; 3] = [
    Golden {
        label: "J2000",
        utc: (2000, 1, 1, 12, 0, 0.0),
        positions: [
            ("sun", 280_369, 0),
            ("moon", 223_324, 5_171),
            ("mercury", 271_889, -995),
            ("venus", 241_566, 2_066),
            ("mars", 327_963, -1_068),
            ("jupiter", 25_253, -1_262),
            ("saturn", 40_396, -2_445),
            ("uranus", 314_809, -658),
            ("neptune", 303_193, 235),
            ("pluto", 251_455, 10_855),
        ],
    },
    Golden {
        label: "2024 total solar eclipse",
        utc: (2024, 4, 8, 18, 0, 0.0),
        positions: [
            ("sun", 19_386, 0),
            ("moon", 19_183, 329),
            ("mercury", 24_807, 2_836),
            ("venus", 4_427, -1_497),
            ("mars", 343_040, -1_245),
            ("jupiter", 49_043, -802),
            ("saturn", 344_454, -1_684),
            ("uranus", 51_170, -271),
            ("neptune", 358_190, -1_222),
            ("pluto", 301_967, -2_964),
        ],
    },
    Golden {
        label: "2026-08-13",
        utc: (2026, 8, 13, 12, 0, 0.0),
        positions: [
            ("sun", 140_769, 0),
            ("moon", 151_005, -112),
            ("mercury", 126_483, 741),
            ("venus", 186_638, -1_216),
            ("mars", 91_419, 265),
            ("jupiter", 129_719, 496),
            ("saturn", 14_486, -2_581),
            ("uranus", 65_361, -155),
            ("neptune", 4_065, -1_402),
            ("pluto", 303_889, -4_291),
        ],
    },
];

#[test]
fn horizons_observer_longitudes_and_latitudes() {
    let mut worst = 0_i64;
    let mut worst_label = String::new();
    for golden in CHARTS.iter() {
        let (year, month, day, hour, minute, second) = golden.utc;
        let jde_tt =
            jde_tt_frm_utc(year, month, day, hour, minute, second).expect("epoch after 1972");
        for body in APPARENT_BODIES.iter() {
            let (longitude, latitude) =
                geocent_apparent_ecl_pos(body, jde_tt).expect("epoch inside series range");
            let got_longitude = (longitude.to_degrees() * 1_000.0).round() as i64;
            let got_latitude = (latitude.to_degrees() * 1_000.0).round() as i64;
            let &(name, expected_longitude, expected_latitude) = golden
                .positions
                .iter()
                .find(|&&(name, _, _)| name == body.name())
                .expect("golden row for body");
            let longitude_error = circular_error(got_longitude, expected_longitude);
            let latitude_error = got_latitude - expected_latitude;
            assert!(
                longitude_error.abs() <= MAX_ERROR_MILLIDEGREES,
                "{} longitude at {}: expected {}, got {}, error {}",
                name,
                golden.label,
                expected_longitude,
                got_longitude,
                longitude_error
            );
            assert!(
                latitude_error.abs() <= MAX_ERROR_MILLIDEGREES,
                "{} latitude at {}: expected {}, got {}, error {}",
                name,
                golden.label,
                expected_latitude,
                got_latitude,
                latitude_error
            );
            for &error in [longitude_error.abs(), latitude_error.abs()].iter() {
                if error > worst {
                    worst = error;
                    worst_label = format!("{} at {}", name, golden.label);
                }
            }
        }
    }
    println!("worst residual: {} millidegrees ({})", worst, worst_label);
}

#[test]
fn known_retrograde_states_at_the_2024_eclipse() {
    let jde_tt = jde_tt_frm_utc(2024, 4, 8, 18, 0, 0.0).expect("epoch after 1972");
    // Mercury was retrograde during the 2024 eclipse; Mars and the Sun were
    // direct, and the Moon never retrogrades.
    assert_eq!(
        is_retrograde(&ApparentBody::Mercury, jde_tt),
        Ok(true),
        "Mercury was retrograde on 2024-04-08"
    );
    assert_eq!(is_retrograde(&ApparentBody::Mars, jde_tt), Ok(false));
    assert_eq!(is_retrograde(&ApparentBody::Sun, jde_tt), Ok(false));
    assert_eq!(is_retrograde(&ApparentBody::Moon, jde_tt), Ok(false));
}

#[test]
fn range_violations_are_errors_rather_than_degradation() {
    // 1971 precedes the leap-second era.
    assert_eq!(
        jde_tt_frm_utc(1971, 12, 31, 0, 0, 0.0),
        Err(ApparentError::BeforeLeapSecondEra)
    );
    // The Pluto series is stated for 1885 to 2099; the year 2150 is outside.
    let far_future = 2_451_545.0 + 150.0 * 365.25;
    match geocent_apparent_ecl_pos(&ApparentBody::Pluto, far_future) {
        Err(ApparentError::OutsideSeriesRange { body, .. }) => assert_eq!(body, "pluto"),
        other => panic!("expected an out-of-range error, got {:?}", other),
    }
    // The same epoch is fine for a VSOP87D body.
    assert!(geocent_apparent_ecl_pos(&ApparentBody::Jupiter, far_future).is_ok());
}

#[test]
fn the_typed_epoch_and_civil_field_paths_agree() {
    let civil = jde_tt_frm_utc(2026, 8, 13, 12, 0, 0.0).expect("epoch after 1972");
    let typed = jde_tt_frm_epoch(
        Epoch::maybe_from_gregorian_utc(2026, 8, 13, 12, 0, 0, 0).expect("valid UTC epoch"),
    );
    assert_eq!(civil, typed);
    // A TT epoch converts without the UTC leap-second offset.
    let tt = jde_tt_frm_epoch(Epoch::from_gregorian_tai(2026, 8, 13, 12, 0, 0, 0));
    assert!((typed - tt).abs() > 1e-6, "UTC and TAI epochs must differ");
}

#[test]
fn j2000_noon_utc_is_the_standard_epoch_in_terrestrial_time() {
    let jde = jde_tt_frm_utc(2000, 1, 1, 12, 0, 0.0).expect("epoch after 1972");
    // TT ran 64.184 s ahead of UTC in 2000.
    let expected = 2_451_545.0 + 64.184 / 86_400.0;
    assert!((jde - expected).abs() < 1e-9, "got {}", jde);
}

fn circular_error(actual: i64, expected: i64) -> i64 {
    (actual - expected + 180_000).rem_euclid(360_000) - 180_000
}
