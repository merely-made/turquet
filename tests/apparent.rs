// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Measures the apparent pipeline against NASA/JPL Horizons observer-table
//! quantity 31: geocentric apparent ecliptic-of-date longitude and latitude,
//! including light time and stellar aberration, at three instants. Values are
//! compared in millidegrees because that is the precision the first consumer
//! stores; the assertion ceiling is a measured bound, not an accuracy claim.

extern crate turquet;

use turquet::apparent::{
    is_retrograde, position, ApparentBody, ApparentError, ApparentSky, ApparentStage,
    ANALYTICAL_APPARENT, APPARENT_BODIES, APPARENT_STAGES,
};
use turquet::compat::apparent as legacy_apparent;
use turquet::foundation::{AccuracyEvidence, JulianDate, ScaleAwareEpoch, TerrestrialTime};

/// Worst allowed residual. Measured: all ten bodies land on the Horizons
/// millidegree exactly or within 2 across these three charts.
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
        let epoch = tt_from_utc(year, month, day, hour, minute, second);
        let sky = ApparentSky::at(epoch);
        for body in APPARENT_BODIES.iter() {
            let modelled = sky.position(*body).expect("epoch inside series range");
            let state = modelled.value();
            let got_longitude = (state.direction().longitude().degrees() * 1_000.0).round() as i64;
            let got_latitude = (state.direction().latitude().degrees() * 1_000.0).round() as i64;
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
    let epoch = tt_from_utc(2024, 4, 8, 18, 0, 0.0);
    // Mercury was retrograde during the 2024 eclipse; Mars and the Sun were
    // direct, and the Moon never retrogrades.
    assert_eq!(
        is_retrograde(ApparentBody::Mercury, epoch),
        Ok(true),
        "Mercury was retrograde on 2024-04-08"
    );
    assert_eq!(is_retrograde(ApparentBody::Mars, epoch), Ok(false));
    assert_eq!(is_retrograde(ApparentBody::Sun, epoch), Ok(false));
    assert_eq!(is_retrograde(ApparentBody::Moon, epoch), Ok(false));
}

#[test]
fn range_violations_are_errors_rather_than_degradation() {
    // 1971 precedes the leap-second era.
    assert_eq!(
        legacy_apparent::jde_tt_frm_utc(1971, 12, 31, 0, 0, 0.0),
        Err(ApparentError::BeforeLeapSecondEra)
    );
    // The Pluto series is stated for 1885 to 2099; the year 2150 is outside.
    let far_future = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0 + 150.0 * 365.25)
        .expect("finite TT epoch");
    match position(ApparentBody::Pluto, far_future) {
        Err(ApparentError::OutsideSeriesRange { body, .. }) => assert_eq!(body, "pluto"),
        other => panic!("expected an out-of-range error, got {:?}", other),
    }
    // The same epoch is fine for a VSOP87D body.
    assert!(position(ApparentBody::Jupiter, far_future).is_ok());
}

#[test]
fn the_typed_epoch_and_civil_field_paths_agree() {
    let civil = legacy_apparent::jde_tt_frm_utc(2026, 8, 13, 12, 0, 0.0).expect("epoch after 1972");
    let typed = JulianDate::<TerrestrialTime>::from_epoch(
        ScaleAwareEpoch::maybe_from_gregorian_utc(2026, 8, 13, 12, 0, 0, 0)
            .expect("valid UTC epoch"),
    )
    .day();
    assert_eq!(civil, typed);
    // A TT epoch converts without the UTC leap-second offset.
    let tt = JulianDate::<TerrestrialTime>::from_epoch(ScaleAwareEpoch::from_gregorian_tai(
        2026, 8, 13, 12, 0, 0, 0,
    ))
    .day();
    assert!((typed - tt).abs() > 1e-6, "UTC and TAI epochs must differ");
}

#[test]
fn j2000_noon_utc_is_the_standard_epoch_in_terrestrial_time() {
    let jde = tt_from_utc(2000, 1, 1, 12, 0, 0.0).day();
    // TT ran 64.184 s ahead of UTC in 2000.
    let expected = 2_451_545.0 + 64.184 / 86_400.0;
    assert!((jde - expected).abs() < 1e-9, "got {}", jde);
}

#[test]
fn primary_states_disclose_units_model_and_accuracy() {
    let state = position(ApparentBody::Moon, tt_from_utc(2026, 8, 13, 12, 0, 0.0))
        .expect("supported epoch");
    assert_eq!(state.model(), ANALYTICAL_APPARENT);
    assert_eq!(
        state.accuracy().evidence(),
        AccuracyEvidence::ExternalComparison
    );
    assert_eq!(state.accuracy().max_angular_error().degrees(), 0.010);
    let lunar_distance = state.value().distance().kilometers();
    assert!(lunar_distance > 350_000.0 && lunar_distance < 410_000.0);
}

#[test]
fn analytical_stage_order_is_explicit() {
    assert_eq!(
        APPARENT_STAGES,
        [
            ApparentStage::Precession,
            ApparentStage::LightTime,
            ApparentStage::SolarDeflection,
            ApparentStage::AnnualAberration,
            ApparentStage::Nutation,
        ]
    );
}

#[test]
fn mercury_station_is_bracketed_against_horizons() {
    let retrograde_epoch = tt_from_utc(2024, 4, 25, 0, 0, 0.0);
    let direct_epoch = tt_from_utc(2024, 4, 26, 0, 0, 0.0);
    assert_eq!(
        is_retrograde(ApparentBody::Mercury, retrograde_epoch),
        Ok(true)
    );
    assert_eq!(
        is_retrograde(ApparentBody::Mercury, direct_epoch),
        Ok(false)
    );

    let retrograde = position(ApparentBody::Mercury, retrograde_epoch)
        .expect("Mercury station epoch is supported")
        .into_value()
        .direction();
    let direct = position(ApparentBody::Mercury, direct_epoch)
        .expect("Mercury station epoch is supported")
        .into_value()
        .direction();
    assert_degrees_close(retrograde.longitude().degrees(), 15.9932191, 0.010);
    assert_degrees_close(retrograde.latitude().degrees(), -1.1967370, 0.010);
    assert_degrees_close(direct.longitude().degrees(), 15.9900610, 0.010);
    assert_degrees_close(direct.latitude().degrees(), -1.4210524, 0.010);
}

#[test]
fn lunar_perigee_and_apogee_samples_match_horizons() {
    let samples = [
        (2024, 4, 7, 18, 0, 4.1365027, -1.0573360, 0.00239870764692),
        (2024, 4, 20, 2, 0, 167.6320605, 2.4842272, 0.00271160426587),
    ];
    for &(year, month, day, hour, minute, longitude, latitude, distance) in samples.iter() {
        let state = position(
            ApparentBody::Moon,
            tt_from_utc(year, month, day, hour, minute, 0.0),
        )
        .expect("lunar extreme epoch is supported")
        .into_value();
        assert_degrees_close(state.direction().longitude().degrees(), longitude, 0.010);
        assert_degrees_close(state.direction().latitude().degrees(), latitude, 0.010);
        assert!(
            (state.distance().astronomical_units() - distance).abs() < 0.000_002,
            "lunar range residual at {:04}-{:02}-{:02} was {:.9} AU",
            year,
            month,
            day,
            state.distance().astronomical_units() - distance
        );
    }
}

fn tt_from_utc(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: f64,
) -> JulianDate<TerrestrialTime> {
    let whole_seconds = second.trunc() as u8;
    let nanoseconds = ((second.fract()) * 1e9).round() as u32;
    JulianDate::from_epoch(
        ScaleAwareEpoch::maybe_from_gregorian_utc(
            year,
            month as u8,
            day as u8,
            hour as u8,
            minute as u8,
            whole_seconds,
            nanoseconds,
        )
        .expect("valid UTC epoch"),
    )
}

fn circular_error(actual: i64, expected: i64) -> i64 {
    (actual - expected + 180_000).rem_euclid(360_000) - 180_000
}

fn assert_degrees_close(actual: f64, expected: f64, tolerance_degrees: f64) {
    let residual = (actual - expected + 180.0).rem_euclid(360.0) - 180.0;
    assert!(
        residual.abs() <= tolerance_degrees,
        "expected {:.9} deg, got {:.9} deg, residual {:.6} deg",
        expected,
        actual,
        residual
    );
}
