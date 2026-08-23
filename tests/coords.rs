/*
Copyright (c) 2015, 2016 Saurav Sachidanand

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
*/

extern crate turquet;

use std::f64::consts::PI;
use turquet::coords;

fn assert_close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {:.16}, got {:.16}",
        expected,
        actual
    );
}

#[test]
fn east_positive_longitude_advances_local_sidereal_time() {
    let greenwich_sidereal = 1.0;
    let east_longitude = 0.25;
    let right_ascension = 0.75;

    assert_close(
        coords::hr_angl_frm_observer_long(
            greenwich_sidereal,
            east_longitude,
            right_ascension,
        ),
        0.5,
        1e-15,
    );
}

#[test]
fn equatorial_to_horizontal_matches_sofa_validation_vector() {
    // IAU SOFA release 2023-10-11, t_sofa_c iauHd2ae. Turquet's inherited
    // azimuth convention is south zero, east positive, hence the PI shift
    // from SOFA's north-zero result.
    let hour_angle = 1.1;
    let declination = 1.2;
    let latitude = 0.3;

    assert_close(
        coords::az_frm_eq(hour_angle, declination, latitude),
        5.916889243730066194 - PI,
        1e-13,
    );
    assert_close(
        coords::alt_frm_eq(hour_angle, declination, latitude),
        0.4472186304990486228,
        1e-14,
    );
}

#[test]
fn horizontal_to_equatorial_matches_sofa_validation_vector() {
    // IAU SOFA release 2023-10-11, t_sofa_c iauAe2hd. Convert SOFA's
    // north-zero azimuth to Turquet's inherited south-zero convention.
    let azimuth = 5.5 - PI;
    let altitude = 1.1;
    let latitude = 0.7;

    assert_close(
        coords::hr_angl_frm_hz(azimuth, altitude, latitude),
        0.5933291115507309663,
        1e-14,
    );
    assert_close(
        coords::dec_frm_hz(azimuth, altitude, latitude),
        0.9613934761647817620,
        1e-14,
    );
}
