// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use turquet::eop::{self, EopError, EopFormalErrors, EopSample, EopSeries};
use turquet::foundation::{JulianDate, TerrestrialTime};
use turquet::provider::EarthOrientationProvider;

const SOURCE: &str = include_str!("vectors/eop-finals2000a-observed.txt");

fn series() -> EopSeries {
    EopSeries::parse_finals2000a(SOURCE, "IERS finals2000A", "retained fixture sha256").unwrap()
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{} != {}",
        actual,
        expected
    );
}

fn alter(start: usize, end: usize, replacement: &str) -> String {
    assert_eq!(end - start, replacement.len());
    let mut row = SOURCE.lines().next().unwrap().to_owned();
    row.replace_range(start..end, replacement);
    row
}

#[test]
fn retains_source_final_b_values_units_and_identity() {
    let series = series();
    assert_eq!(series.source_text(), SOURCE);
    assert_eq!(series.samples().len(), 9);
    assert_eq!(series.authority(), "IERS finals2000A");
    assert_eq!(series.snapshot(), "retained fixture sha256");
    let sample = &series.samples()[0];
    assert_eq!(sample.mjd_utc(), 57751.0);
    assert_eq!(sample.xp_arcseconds(), 0.084589);
    assert_eq!(sample.yp_arcseconds(), 0.264175);
    assert_eq!(sample.dut1_seconds(), -0.4060935);
    // Compare B fields directly against the retained primary-source row.
    let row = SOURCE.lines().next().unwrap();
    assert_eq!(
        sample.xp_arcseconds(),
        row[134..144].trim().parse::<f64>().unwrap()
    );
    assert_eq!(
        sample.yp_arcseconds(),
        row[144..154].trim().parse::<f64>().unwrap()
    );
    assert_eq!(
        sample.dut1_seconds(),
        row[154..165].trim().parse::<f64>().unwrap()
    );
    assert_ne!(
        sample.xp_arcseconds(),
        row[18..27].trim().parse::<f64>().unwrap()
    );
    assert_eq!(sample.formal_errors(), None);
    for (index, sample) in series.samples().iter().enumerate() {
        let result = series.evaluate(sample.epoch_tt()).unwrap();
        assert_eq!(result.requested_tt.parts(), sample.epoch_tt().parts());
        assert_eq!(
            (result.lower_index, result.upper_index, result.fraction),
            (index, index, 0.0)
        );
        assert_eq!(result.dut1_seconds, sample.dut1_seconds());
        assert_eq!(result.earth_orientation.authority(), series.authority());
        assert_eq!(result.earth_orientation.snapshot(), series.snapshot());
        close(
            result.earth_orientation.polar_motion_x().radians(),
            sample.xp_arcseconds().to_radians() / 3600.0,
            1e-18,
        );
        close(
            result.earth_orientation.polar_motion_y().radians(),
            sample.yp_arcseconds().to_radians() / 3600.0,
            1e-18,
        );
        let (a, b) = result.earth_orientation.ut1().parts();
        close(
            a - 2_400_000.5 + b,
            sample.mjd_utc() + sample.dut1_seconds() / 86_400.0,
            1e-11,
        );
    }
}

#[test]
fn interpolates_daily_observations_and_implements_existing_provider() {
    let series = series();
    let left = &series.samples()[6];
    let right = &series.samples()[7];
    let tt = left.epoch_tt().offset_days(0.25).unwrap();
    let result = series.evaluate(tt).unwrap();
    assert_eq!(result.requested_tt.parts(), tt.parts());
    assert_eq!((result.lower_index, result.upper_index), (6, 7));
    close(result.fraction, 0.25, 1e-15);
    close(
        result.dut1_seconds,
        left.dut1_seconds() * 0.75 + right.dut1_seconds() * 0.25,
        1e-15,
    );
    close(
        result.xp_arcseconds,
        left.xp_arcseconds() * 0.75 + right.xp_arcseconds() * 0.25,
        1e-15,
    );
    close(
        result.yp_arcseconds,
        left.yp_arcseconds() * 0.75 + right.yp_arcseconds() * 0.25,
        1e-15,
    );
    assert_eq!(series.at(tt).unwrap(), result.earth_orientation);
    assert_eq!(series.at(tt).unwrap(), series.clone().at(tt).unwrap());
    assert_eq!(
        EarthOrientationProvider::authority(&series),
        series.authority()
    );
    assert_eq!(series.data_snapshot(), series.snapshot());
    // UT1 advances at the requested TT, including the observed DUT1 slope.
    let (ta, tb) = tt.parts();
    let (ua, ub) = result.earth_orientation.ut1().parts();
    close(
        ((ta - ua) + (tb - ub)) * 86400.0,
        69.184 - result.dut1_seconds,
        1e-10,
    );
}

#[test]
fn no_extrapolation_even_just_outside_endpoints() {
    let series = series();
    let (first, last) = series.coverage_tt();
    for tt in [
        first.offset_days(-1e-9).unwrap(),
        last.offset_days(1e-9).unwrap(),
        first.offset_days(-1000.0).unwrap(),
    ] {
        assert_eq!(series.evaluate(tt), Err(EopError::OutOfRange));
    }
    assert!(series.at(first).is_ok());
    assert!(series.at(last).is_ok());
}

#[test]
fn leap_interval_is_refused_but_exact_observations_on_both_sides_work() {
    let series = series();
    let dec31 = &series.samples()[2];
    let jan1 = &series.samples()[3];
    assert_eq!((dec31.mjd_utc(), jan1.mjd_utc()), (57753.0, 57754.0));
    assert!((jan1.dut1_seconds() - dec31.dut1_seconds()).abs() > 0.9);
    for days in [0.25, 0.5, 1.0, 1.0 + 0.5 / 86400.0] {
        assert_eq!(
            series.evaluate(dec31.epoch_tt().offset_days(days).unwrap()),
            Err(EopError::LeapDiscontinuity {
                lower_index: 2,
                upper_index: 3
            })
        );
    }
    assert!(series.at(dec31.epoch_tt()).is_ok());
    assert!(series.at(jan1.epoch_tt()).is_ok());
    assert!(series.at(jan1.epoch_tt().offset_days(0.5).unwrap()).is_ok());
}

#[test]
fn refuses_gaps_and_suspect_dut1_jumps_without_losing_knots() {
    let series = series();
    assert_eq!(
        series.evaluate(series.samples()[5].epoch_tt().offset_days(1.0).unwrap()),
        Err(EopError::Gap {
            lower_index: 5,
            upper_index: 6
        })
    );
    let a = EopSample::new(58849.0, -0.4, 0.1, 0.2, None).unwrap();
    let b = EopSample::new(58850.0, 0.4, 0.1, 0.2, None).unwrap();
    let tt = a.epoch_tt().offset_days(0.5).unwrap();
    let jump = EopSeries::new("supplied", "IERS", "jump", vec![a, b]).unwrap();
    assert_eq!(
        jump.evaluate(tt),
        Err(EopError::LeapDiscontinuity {
            lower_index: 0,
            upper_index: 1
        })
    );
}

#[test]
fn rejects_predictions_missing_b_and_bad_fixed_columns() {
    for row in [
        alter(16, 17, "P"),
        alter(57, 58, "P"),
        alter(16, 17, " "),
        alter(134, 144, "          "),
        alter(144, 154, "          "),
        alter(154, 165, "           "),
    ] {
        assert_eq!(
            EopSeries::parse_finals2000a(&row, "IERS", "x"),
            Err(EopError::NotFinalObserved { line: 1 })
        );
    }
    for row in [
        String::from("short"),
        alter(134, 144, "       NaN"),
        alter(134, 144, "       inf"),
        alter(7, 15, "57751.50"),
        alter(0, 2, "15"),
        alter(2, 4, "99"),
        alter(4, 6, "30"),
        SOURCE.lines().next().unwrap().replace('I', "é"),
    ] {
        assert!(matches!(
            EopSeries::parse_finals2000a(&row, "IERS", "x"),
            Err(EopError::Parse { line: 1, .. })
        ));
    }
    // A's independent nutation classification must not reclassify B's EOP.
    assert!(EopSeries::parse_finals2000a(&alter(95, 96, "P"), "IERS", "x").is_ok());
}

#[test]
fn rejects_empty_identity_empty_series_duplicates_and_reversed_samples() {
    assert_eq!(
        EopSeries::parse_finals2000a("\n \r\n", "IERS", "x"),
        Err(EopError::EmptySeries)
    );
    for (authority, snapshot) in [(" ", "x"), ("IERS", "\n")] {
        assert_eq!(
            EopSeries::parse_finals2000a(SOURCE, authority, snapshot),
            Err(EopError::EmptyIdentity)
        );
    }
    let a = series().samples()[0].clone();
    let b = series().samples()[1].clone();
    for samples in [vec![a.clone(), a], vec![b, series().samples()[0].clone()]] {
        assert_eq!(
            EopSeries::new("", "IERS", "x", samples),
            Err(EopError::NotStrictlyIncreasing { index: 1 })
        );
    }
    let row = SOURCE.lines().next().unwrap();
    assert_eq!(
        EopSeries::parse_finals2000a(&format!("{}\n{}", row, row), "IERS", "x"),
        Err(EopError::NotStrictlyIncreasing { index: 1 })
    );
}

#[test]
fn single_knot_and_caller_formal_errors_are_retained() {
    let errors = EopFormalErrors {
        dut1_seconds: 0.00001,
        xp_arcseconds: 0.00002,
        yp_arcseconds: 0.00003,
    };
    let sample = EopSample::new(58849.0, -0.1, 0.2, 0.3, Some(errors)).unwrap();
    let tt = sample.epoch_tt();
    let series =
        EopSeries::new("caller source", " authority ", " snapshot ", vec![sample]).unwrap();
    assert_eq!(series.samples()[0].formal_errors(), Some(errors));
    assert_eq!(series.at(tt).unwrap().authority(), " authority ");
    assert_eq!(series.at(tt).unwrap().snapshot(), " snapshot ");
    assert_eq!(
        series.evaluate(tt.offset_days(1e-6).unwrap()),
        Err(EopError::OutOfRange)
    );
    for bad in [f64::NAN, f64::INFINITY, -0.1] {
        let mut errors = errors;
        errors.dut1_seconds = bad;
        assert!(EopSample::new(58849.0, -0.1, 0.2, 0.3, Some(errors)).is_err());
    }
}

#[test]
fn validates_bounds_and_rejects_overflowing_tt() {
    for mjd in [f64::NAN, f64::INFINITY, 41316.0, 58849.5, 1e100] {
        assert!(EopSample::new(mjd, 0.1, 0.2, 0.3, None).is_err());
    }
    for (dut1, xp, yp) in [
        (f64::NAN, 0.0, 0.0),
        (0.0, f64::INFINITY, 0.0),
        (0.0, 0.0, 11.0),
        (1.1, 0.0, 0.0),
    ] {
        assert!(EopSample::new(58849.0, dut1, xp, yp, None).is_err());
    }
    let tt = JulianDate::<TerrestrialTime>::from_parts(f64::MAX, f64::MAX).unwrap();
    assert_eq!(
        series().evaluate(tt),
        Err(EopError::InvalidValue("TT epoch"))
    );
    let cancelling = JulianDate::<TerrestrialTime>::from_parts(f64::MAX, -f64::MAX).unwrap();
    assert_eq!(
        series().evaluate(cancelling),
        Err(EopError::InvalidValue("TT epoch"))
    );
    let oversized = " ".repeat(eop::MAX_EOP_SOURCE_BYTES + 1);
    assert_eq!(
        EopSeries::parse_finals2000a(&oversized, "IERS", "x"),
        Err(EopError::ResourceLimit)
    );
}

#[test]
fn century_rule_crlf_and_atomic_parse_errors() {
    let text = include_str!("vectors/eop-finals2000a-century.txt");
    let retained = format!("\r\n{}\r\n", text.replace('\n', "\r\n"));
    let series = EopSeries::parse_finals2000a(&retained, "IERS", "century").unwrap();
    assert_eq!(series.source_text(), retained);
    assert_eq!(
        series
            .samples()
            .iter()
            .map(|s| s.mjd_utc())
            .collect::<Vec<_>>(),
        vec![51543.0, 51544.0, 51545.0]
    );
    let invalid = format!("{}\n{}", SOURCE.lines().next().unwrap(), alter(16, 17, "P"));
    assert_eq!(
        EopSeries::parse_finals2000a(&invalid, "IERS", "x"),
        Err(EopError::NotFinalObserved { line: 2 })
    );
}

#[test]
fn preserves_submicrosecond_request_parts_and_checks_sample_limit() {
    let series = series();
    let tt = series.samples()[6]
        .epoch_tt()
        .offset_days(1e-9 / 86400.0)
        .unwrap();
    let result = series.evaluate(tt).unwrap();
    assert_eq!(result.requested_tt.parts(), tt.parts());
    assert!(result.fraction > 0.0);
    assert_eq!((result.lower_index, result.upper_index), (6, 7));
    assert_eq!(
        EopSeries::new(
            "",
            "IERS",
            "x",
            vec![series.samples()[0].clone(); eop::MAX_EOP_SAMPLES + 1]
        ),
        Err(EopError::ResourceLimit)
    );
}
