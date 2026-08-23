// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Published IAU SOFA validation vectors from issue 2023-10-11. These values
//! are fixtures rather than calls back into a second implementation.

extern crate turquet;

use turquet::foundation::{AccuracyEvidence, Gcrs, JulianDate, TerrestrialTime, UnitVector};
use turquet::orientation::{gcrs_to_true_equator, nutation, IAU_2006_2000A};

#[test]
fn iau_2006_adjusted_2000a_nutation_matches_sofa_vector() {
    let epoch =
        JulianDate::<TerrestrialTime>::from_parts(2_400_000.5, 53_736.0).expect("finite TT epoch");
    let result = nutation(epoch);
    assert_close(
        result.value().longitude().radians(),
        -0.9630912025820308797e-5,
        1e-13,
    );
    assert_close(
        result.value().obliquity().radians(),
        0.4063238496887249798e-4,
        1e-13,
    );
    assert_eq!(result.model(), IAU_2006_2000A);
    assert_eq!(result.accuracy().evidence(), AccuracyEvidence::Conformance);
}

#[test]
fn iau_2006_2000a_rotation_matches_sofa_vector() {
    let epoch = JulianDate::<TerrestrialTime>::from_parts(2_400_000.5, 50_123.9999)
        .expect("finite TT epoch");
    let rotation = gcrs_to_true_equator(epoch);
    let matrix = rotation.value().matrix();
    let expected = [
        [
            0.9999995832794205484,
            0.8372382772630962111e-3,
            0.3639684771140623099e-3,
        ],
        [
            -0.8372533744743683605e-3,
            0.9999996486492861646,
            0.4132905944611019498e-4,
        ],
        [
            -0.3639337469629464969e-3,
            -0.4163377605910663999e-4,
            0.9999999329094260057,
        ],
    ];
    for row in 0..3 {
        for column in 0..3 {
            assert_close(matrix[row][column], expected[row][column], 1e-12);
        }
    }

    let x_axis = UnitVector::<Gcrs>::new([1.0, 0.0, 0.0]).expect("unit vector");
    let transformed = rotation.into_value().apply(x_axis).components();
    for row in 0..3 {
        assert_close(transformed[row], expected[row][0], 1e-12);
    }
}

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {:.17e}, got {:.17e}, residual {:.3e}",
        expected,
        actual,
        actual - expected
    );
}
