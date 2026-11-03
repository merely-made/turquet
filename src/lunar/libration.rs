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

use super::*;

#[inline]
pub(super) fn A(
    mn_geocent_moon_long: f64,
    app_geocent_moon_lat: f64,
    long_of_mn_ascent_node: f64,
) -> f64 {
    let I = inc_of_mn_lunar_eq();
    let W = mn_geocent_moon_long - long_of_mn_ascent_node;

    (W.sin() * app_geocent_moon_lat.cos() * I.cos() - app_geocent_moon_lat.sin() * I.sin())
        .atan2(W.cos() * app_geocent_moon_lat.cos())
}

#[inline]
pub(super) fn F(JC: f64) -> f64 {
    angle::limit_to_360(Horner_eval!(
        JC,
        93.272095,
        483202.0175233,
        -0.0036539,
        -1.0 / 3526000.0,
        1.0 / 863310000.0
    ))
    .to_radians()
}

#[inline]
pub(super) fn E(JC: f64) -> f64 {
    1.0 - JC * (0.002_516 + JC * 0.000_0074)
}

#[inline]
pub(super) fn DMM1(JC: f64) -> (f64, f64, f64) {
    let D = angle::limit_to_360(Horner_eval!(
        JC,
        297.8501921,
        445267.1114034,
        -0.0018819,
        1.0 / 545868.0,
        -1.0 / 113065000.0
    ))
    .to_radians();

    let M = angle::limit_to_360(Horner_eval!(
        JC,
        357.5291092,
        35999.0502909,
        -0.0001536,
        1.0 / 24490000.0
    ))
    .to_radians();

    let M1 = angle::limit_to_360(Horner_eval!(
        JC,
        134.9633964,
        477198.8675055,
        0.0087414,
        1.0 / 69699.0,
        -1.0 / 14712000.0
    ))
    .to_radians();

    (D, M, M1)
}

#[inline]
pub(super) fn rho_sig(D: f64, M1: f64, F: f64) -> (f64, f64) {
    let x2F = 2.0 * F;
    let x2F2D = x2F - 2.0 * D;

    (
        (-0.02752 * M1.cos() - 0.02245 * F.sin() + 0.00684 * (M1 - x2F).cos()
            - 0.00293 * (x2F).cos()
            - 0.00085 * (x2F2D).cos()
            - 0.00054 * (M1 - 2.0 * D).cos()
            - 0.00020 * ((M1 + F).sin() + (M1 + x2F).cos() + (M1 - F).cos())
            + 0.00014 * (M1 + x2F2D).cos())
        .to_radians(),
        (-0.02816 * M1.sin() + 0.02244 * F.cos()
            - 0.00682 * (M1 - x2F).sin()
            - 0.00279 * (x2F).sin()
            - 0.00083 * (x2F2D).sin()
            + 0.00069 * (M1 - 2.0 * D).sin()
            + 0.0004 * (M1 + F).cos()
            - 0.00025 * (2.0 * M1).sin()
            - 0.00023 * (M1 + x2F).sin()
            + 0.0002 * (M1 - F).cos()
            + 0.00019 * (M1 - F).sin()
            + 0.00013 * (M1 + x2F2D).sin()
            - 0.0001 * (M1 - 3.0 * F).cos())
        .to_radians(),
    )
}

/**
Computes the optical librations of the Moon in longitude and
latitude

# Returns

`(optical_libr_in_long, optical_libr_in_lat)`

* `optical_libr_in_long`: The optical libration in
                          longitude *| in radians*
* `optical_libr_in_lat` : The optical libration in
                          latitude *| in radians*

# Arguments

* `JD`                 : Julian (Ephemeris) day
* `mn_ecl_long_moon`   : The mean ecliptic longitude of the
                         Moon *| in radians*, i.e, *without* the correction for
                         nutation
* `apprnt_ecl_lat_moon`: The apparent ecliptic latitude of the
                         Moon *| in radians*, i.e, *with* the correction for
                         nutation
**/
pub fn optical_libr(JD: f64, mn_ecl_long_moon: f64, apprnt_ecl_lat_moon: f64) -> (f64, f64) {
    let JC = time::julian_cent(JD);
    let F = F(JC);
    let I = inc_of_mn_lunar_eq();

    let long_of_mn_ascend_node = mn_ascend_node(JC);
    let W = mn_ecl_long_moon - long_of_mn_ascend_node;

    let A = A(
        mn_ecl_long_moon,
        apprnt_ecl_lat_moon,
        long_of_mn_ascend_node,
    );

    let b1 = (-W.sin() * apprnt_ecl_lat_moon.cos() * I.sin() - apprnt_ecl_lat_moon.sin() * I.cos())
        .asin();

    (angle::limit_to_two_PI(A - F), b1)
}

/**
Computes the physical librations of the Moon in longitude and
latitude

# Returns

`(physical_libr_in_long, physical_libr_in_lat)`

* `physical_libr_in_long`: The physical libration in
                           longitude *| in radians*
* `physical_libr_in_lat`: The physical libration in
                          latitude *| in radians*

# Arguments

* `JD`                 : Julian (Ephemeris) day
* `mn_ecl_long_moon`   : The mean ecliptic longitude of the
                         Moon *| in radians*, i.e, *without* the correction
                         for nutation
* `apprnt_ecl_lat_moon`: The apparent ecliptic latitude of the
                         Moon *| in radians*, i.e, *with* the correction for
                         nutation
* `optical_lib_lat`    : The optical libration in latitude *| in radians*
**/
pub fn physical_libr(
    JD: f64,
    mn_ecl_long_moon: f64,
    apprnt_ecl_lat_moon: f64,
    optical_lib_lat: f64,
) -> (f64, f64) {
    let JC = time::julian_cent(JD);
    let K1 = (119.75 + 131.849 * JC).to_radians();
    let K2 = (72.560 + 20.186 * JC).to_radians();

    let long_of_mn_ascend_node = mn_ascend_node(JC);
    let (D, M, M1) = DMM1(JC);
    let F = F(JC);
    let E = E(JC);

    let x2F = 2.0 * F;
    let x2D = 2.0 * D;
    let x2F2D = x2F - x2D;

    let (rho, sig) = rho_sig(D, M1, F);
    let tau = (0.02520 * E * M.sin() + 0.00473 * (2.0 * M1 - x2F).sin() - 0.00467 * M1.sin()
        + 0.00396 * K1.sin()
        + 0.00276 * (2.0 * M1 - x2D).sin()
        + 0.00196 * long_of_mn_ascend_node.sin()
        - 0.00183 * (M1 - F).cos()
        + 0.00115 * (M1 - x2D).sin()
        - 0.00096 * (M1 - D).sin()
        + 0.00046 * (x2F2D).sin()
        - 0.00039 * (M1 - F).sin()
        - 0.00032 * (M1 - M - D).sin()
        + 0.00027 * (2.0 * M1 - M - x2D).sin()
        + 0.00023 * K2.sin()
        - 0.00014 * (x2D.sin() - (2.0 * M1 - x2F).cos())
        - 0.00012 * ((M1 - x2F).sin() + (2.0 * M1).sin())
        + 0.00011 * (2.0 * (M1 - M - D)).sin())
    .to_radians();

    let A = A(
        mn_ecl_long_moon,
        apprnt_ecl_lat_moon,
        long_of_mn_ascend_node,
    );

    (
        optical_lib_lat.tan() * (rho * A.cos() + sig * A.sin()) - tau,
        sig * A.cos() - rho * A.sin(),
    )
}

/**
Computes the total librations of the Moon in longitude and
latitude

# Returns

`(total_libr_in_long, total_libr_in_lat)`

* `total_libr_in_long`: The total libration in longitude *| in radians*
* `total_libr_in_lat` : The total libration in latitude *| in radians*

# Arguments

* `JD`                 : Julian (Ephemeris) day
* `mn_ecl_long_moon`   : The mean ecliptic longitude of the
                         Moon *| in radians*, i.e, *without* the correction
                         for nutation
* `apprnt_ecl_lat_moon`: The apparent ecliptic latitude of the
                         Moon *| in radians*, i.e, *with* the correction for
                         nutation
**/
pub fn total_libr(JD: f64, mn_ecl_long_moon: f64, apprnt_ecl_lat_moon: f64) -> (f64, f64) {
    let (opt_long, opt_lat) = optical_libr(mn_ecl_long_moon, apprnt_ecl_lat_moon, JD);
    let (phys_long, phys_lat) = physical_libr(mn_ecl_long_moon, apprnt_ecl_lat_moon, JD, opt_lat);

    (opt_long + phys_long, opt_lat + phys_lat)
}

/**
Computes the position angle of the axis of rotation of the Moon

# Returns

* `pos_angl_of_axis_of_rot`: The position angle of the axis
                             of rotation of the Moon *| in radians*

# Arguments

* `JD`                 : Julian (Ephemeris) day
* `mn_ascent_node_long`: Longitude of the mean ascending node of the
                         Moon *| in radians*
* `total_lib_lat`      : Total libration of the Moon in
                         latitude *| in radians*
* `nut_in_long`        : Nutation correction for longitude *| in radians*
* `true_oblq`          : True obliquity of the ecliptic *| in radians*
* `apprnt_moon_asc`    : Apparent geocentric right ascension of the
                         Moon *| in radians*
**/
#[allow(unused_variables)]
pub fn pos_angl_of_axis_of_rot(
    JD: f64,
    mn_ascent_node_long: f64,
    total_lib_lat: f64,
    nut_in_long: f64,
    true_oblq: f64,
    apprnt_moon_asc: f64,
) -> f64 {
    let JC = time::julian_cent(JD);
    let (D, M, M1) = DMM1(JC);
    let F = F(JC);
    let (rho, sig) = rho_sig(D, M1, F);

    let I = inc_of_mn_lunar_eq();
    let V = mn_ascent_node_long + nut_in_long + sig / I.sin();
    let X = (I + rho).sin() * V.sin();
    let Y = (I + rho).sin() * V.cos() * true_oblq.cos() - (I + rho).cos() * true_oblq.sin();
    let w = X.atan2(Y);

    ((X * X + Y * Y).sqrt() * (apprnt_moon_asc - w).cos() / total_lib_lat.cos()).asin()
}

/**
Computes the topocentric librations of the Moon

# Returns

`(topocent_libr_in_long, topocent_libr_in_lat, topocent_libr_in_P)`

* `topocent_libr_in_long`: Topocentric libration in longitude *| in radians*
* `topocent_libr_in_lat` : Topocentric libration in latitude *| in radians*
* `topocent_libr_in_P`   : Topocentric libration in position angle of the axis
                           of rotation *| in radians*

# Arguments

* `observer_lat`          : Latitude of the observer *| in radians*
* `geocent_dec_moon`      : Geocentric decation of the Moon *| in radians*
* `local_hour_angl`       : Local hour angle of the Moon *| in radians*
* `geocent_hz_parllx_moon`: Geocentric equatorial horizontal parallax of the
                            Moon *| in radians*
* `pos_angl_axis_of_rot`  : Position angle of the axis of rotation of the
                            Moon *| in radians*
* `total_lib_lat`         : Total libration of the Moon in
                            latitude *| in radians*
**/
pub fn topocent_libr_by_diff_corrections(
    observer_lat: f64,
    geocent_dec_moon: f64,
    local_hour_angl: f64,
    geocent_hz_parllx_moon: f64,
    pos_angl_axis_of_rot: f64,
    total_lib_lat: f64,
) -> (f64, f64, f64) {
    let Q = (observer_lat.cos() * local_hour_angl.sin()).atan2(
        geocent_dec_moon.cos() * observer_lat.sin()
            - geocent_dec_moon.sin() * observer_lat.cos() * local_hour_angl.cos(),
    );
    let z = (geocent_dec_moon.sin() * observer_lat.sin()
        + geocent_dec_moon.cos() * observer_lat.cos() * local_hour_angl.cos())
    .acos();
    let pi1 = geocent_hz_parllx_moon * (z.sin() + 0.0084 * (2.0 * z).sin());

    let delta_l = -pi1 * (Q - pos_angl_axis_of_rot).sin() / total_lib_lat.cos();
    let delta_b = -pi1 * (Q - pos_angl_axis_of_rot).cos();
    let delta_P =
        delta_l * (total_lib_lat + delta_b).sin() - pi1 * Q.sin() * geocent_dec_moon.tan();

    (delta_l, delta_b, delta_P)
}
