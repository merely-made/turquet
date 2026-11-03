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

//! The Earth's moon

use angle;
use coords;
use time;

/**
Computes the equatorial horizontal parallax of the Moon

# Returns

* `eq_hz_parllx`: Equatorial horizontal parallax of the
                  Moon *| in radians*

# Arguments

* `earth_moon_dist`: Earth-Moon distance *| in kilometers*
**/
#[inline]
pub fn eq_hz_parllx(earth_moon_dist: f64) -> f64 {
    (6378.14 / earth_moon_dist).asin()
}

/**
Computes the equatorial semidiameter of the Moon

# Returns

* `eq_semidiameter`: Geocentric equatorial semidiameter
                     *| in radians per kilometers*

# Arguments

* `earth_moon_dist`: Earth-Moon distance *| in kilometers*
**/
#[inline]
pub fn semidiameter(earth_moon_dist: f64) -> f64 {
    0.272481 * eq_hz_parllx(earth_moon_dist).sin()
}

/**
Computes the inclination of the mean lunar equator with the
ecliptic

# Returns

* `inc`: Inclination of the mean lunar equator with the
         ecliptic *| in radians*
**/
#[inline]
pub fn inc_of_mn_lunar_eq() -> f64 {
    angle::deg_frm_dms(1, 32, 32.7).to_radians()
}

/**
Computes the geocentric ecliptic position of the Moon,
referred to the mean equinox of the date

# Returns

`(moon_ecl_point, rad_vec)`

* `moon_ecl_point`: Ecliptic point of the Moon *| in radians*
* `rad_vec`: Moon-Earth distance *| in kilometers*

This function uses the partial Chapront ELP-2000/82
lunar theory, so the accruacy of:

* `moon_ecl_point.long`: is `10` arcseconds
* `moon_ecl_point.lat`: is `4` arcseconds

# Arguments

* `JD`: Julian (Ephemeris) day
**/
pub fn geocent_ecl_pos(JD: f64) -> (coords::EclPoint, f64) {
    let JC = time::julian_cent(JD);
    let (D, M, M1) = DMM1(JC);
    let F = F(JC);
    let E = E(JC);
    let L1 = angle::limit_to_360(Horner_eval!(
        JC,
        218.3164477,
        481267.88123421,
        -0.0015786,
        1.0 / 538841.0,
        -1.0 / 65194000.0
    ))
    .to_radians();

    let A1 = angle::limit_to_360(119.75 + 131.849 * JC).to_radians();
    let A2 = angle::limit_to_360(53.090 + 479264.29 * JC).to_radians();
    let A3 = angle::limit_to_360(313.45 + 481266.484 * JC).to_radians();

    struct lrterms(i8, i8, i8, i8, i32, i32);
    let terms_for_lr = [
        lrterms(0, 0, 1, 0, 6288774, -20905355),
        lrterms(2, 0, -1, 0, 1274027, -3699111),
        lrterms(2, 0, 0, 0, 658314, -2955968),
        lrterms(0, 0, 2, 0, 213618, -569925),
        lrterms(0, 1, 0, 0, -185116, 48888),
        lrterms(0, 0, 0, 2, -114332, -3149),
        lrterms(2, 0, -2, 0, 58793, 246158),
        lrterms(2, -1, -1, 0, 57066, -152138),
        lrterms(2, 0, 1, 0, 53322, -170733),
        lrterms(2, -1, 0, 0, 45758, -204586),
        lrterms(0, 1, -1, 0, -40923, -129620),
        lrterms(1, 0, 0, 0, -34720, 108743),
        lrterms(0, 1, 1, 0, -30383, 104755),
        lrterms(2, 0, 0, -2, 15327, 10321),
        lrterms(0, 0, 1, 2, -12528, 0),
        lrterms(0, 0, 1, -2, 10980, 79661),
        lrterms(4, 0, -1, 0, 10675, -34782),
        lrterms(0, 0, 3, 0, 10034, -23210),
        lrterms(4, 0, -2, 0, 8548, -21636),
        lrterms(2, 1, -1, 0, -7888, 24208),
        lrterms(2, 1, 0, 0, -6766, 30824),
        lrterms(1, 0, -1, 0, -5163, -8379),
        lrterms(1, 1, 0, 0, 4987, -16675),
        lrterms(2, -1, 1, 0, 4036, -12831),
        lrterms(2, 0, 2, 0, 3994, -10445),
        lrterms(4, 0, 0, 0, 3861, -11650),
        lrterms(2, 0, -3, 0, 3665, 14403),
        lrterms(0, 1, -2, 0, -2689, -7003),
        lrterms(2, 0, -1, 2, -2602, 0),
        lrterms(2, -1, -2, 0, 2390, 10056),
        lrterms(1, 0, 1, 0, -2348, 6322),
        lrterms(2, -2, 0, 0, 2236, -9884),
        lrterms(0, 1, 2, 0, -2120, 5751),
        lrterms(0, 2, 0, 0, -2069, 0),
        lrterms(2, -2, -1, 0, 2048, -4950),
        lrterms(2, 0, 1, -2, -1773, 4130),
        lrterms(2, 0, 0, 2, -1595, 0),
        lrterms(4, -1, -1, 0, 1215, -3958),
        lrterms(0, 0, 2, 2, -1110, 0),
        lrterms(3, 0, -1, 0, -892, 3258),
        lrterms(2, 1, 1, 0, -810, 2616),
        lrterms(4, -1, -2, 0, 759, -1897),
        lrterms(0, 2, -1, 0, -713, -2117),
        lrterms(2, 2, -1, 0, -700, 2354),
        lrterms(2, 1, -2, 0, 691, 0),
        lrterms(2, -1, 0, -2, 596, 0),
        lrterms(4, 0, 1, 0, 549, -1423),
        lrterms(0, 0, 4, 0, 537, -1117),
        lrterms(4, -1, 0, 0, 520, -1571),
        lrterms(1, 0, -2, 0, -487, -1739),
        lrterms(2, 1, 0, -2, -399, 0),
        lrterms(0, 0, 2, -2, -381, -4421),
        lrterms(1, 1, 1, 0, 351, 0),
        lrterms(3, 0, -2, 0, -340, 0),
        lrterms(4, 0, -3, 0, 330, 0),
        lrterms(2, -1, 2, 0, 327, 0),
        lrterms(0, 2, 1, 0, -323, 1165),
        lrterms(1, 1, -1, 0, 299, 0),
        lrterms(2, 0, 3, 0, 294, 0),
        lrterms(2, 0, -1, -2, 0, 8752),
    ];
    struct bterms(i8, i8, i8, i8, i32);
    let terms_for_b = [
        bterms(0, 0, 0, 1, 5128122),
        bterms(0, 0, 1, 1, 280602),
        bterms(0, 0, 1, -1, 277693),
        bterms(2, 0, 0, -1, 173237),
        bterms(2, 0, -1, 1, 55413),
        bterms(2, 0, -1, -1, 46271),
        bterms(2, 0, 0, 1, 32573),
        bterms(0, 0, 2, 1, 17198),
        bterms(2, 0, 1, -1, 9266),
        bterms(0, 0, 2, -1, 8822),
        bterms(2, -1, 0, -1, 8216),
        bterms(2, 0, -2, -1, 4324),
        bterms(2, 0, 1, 1, 4200),
        bterms(2, 1, 0, -1, -3359),
        bterms(2, -1, -1, 1, 2463),
        bterms(2, -1, 0, 1, 2211),
        bterms(2, -1, -1, -1, 2065),
        bterms(0, 1, -1, -1, -1870),
        bterms(4, 0, -1, -1, 1828),
        bterms(0, 1, 0, 1, -1794),
        bterms(0, 0, 0, 3, -1749),
        bterms(0, 1, -1, 1, -1565),
        bterms(1, 0, 0, 1, -1491),
        bterms(0, 1, 1, 1, -1475),
        bterms(0, 1, 1, -1, -1410),
        bterms(0, 1, 0, -1, -1344),
        bterms(1, 0, 0, -1, -1335),
        bterms(0, 0, 3, 1, 1107),
        bterms(4, 0, 0, -1, 1021),
        bterms(4, 0, -1, 1, 833),
        bterms(0, 0, 1, -3, 777),
        bterms(4, 0, -2, 1, 671),
        bterms(2, 0, 0, -3, 607),
        bterms(2, 0, 2, -1, 596),
        bterms(2, -1, 1, -1, 491),
        bterms(2, 0, -2, 1, -451),
        bterms(0, 0, 3, -1, 439),
        bterms(2, 0, 2, 1, 422),
        bterms(2, 0, -3, -1, 421),
        bterms(2, 1, -1, 1, -366),
        bterms(2, 1, 0, 1, -351),
        bterms(4, 0, 0, 1, 331),
        bterms(2, -1, 1, 1, 315),
        bterms(2, -2, 0, -1, 302),
        bterms(0, 0, 1, 3, -283),
        bterms(2, 1, 1, -1, -229),
        bterms(1, 1, 0, -1, 223),
        bterms(1, 1, 0, 1, 223),
        bterms(0, 1, -2, -1, -220),
        bterms(2, 1, -1, -1, -220),
        bterms(1, 0, 1, 1, -185),
        bterms(2, -1, -2, -1, 181),
        bterms(0, 1, 2, 1, -177),
        bterms(4, 0, -2, -1, 176),
        bterms(4, -1, -1, -1, 166),
        bterms(1, 0, 1, -1, -164),
        bterms(4, 0, 1, -1, 132),
        bterms(1, 0, -1, -1, -119),
        bterms(4, -1, 0, -1, 115),
        bterms(2, -2, 0, 1, 107),
    ];

    let mut l = 0.0;
    let mut r = 0.0;
    let mut b = 0.0;

    for x in terms_for_lr.iter() {
        let arg = (x.0 as f64) * D + (x.1 as f64) * M + (x.2 as f64) * M1 + (x.3 as f64) * F;

        let t = if (x.1).abs() == 1 {
            E
        } else if (x.1).abs() == 2 {
            E * E
        } else {
            1.0
        };

        l += (x.4 as f64) * t * arg.sin();
        r += (x.5 as f64) * t * arg.cos();
    }

    for x in terms_for_b.iter() {
        let arg = (x.0 as f64) * D + (x.1 as f64) * M + (x.2 as f64) * M1 + (x.3 as f64) * F;
        let mut t = (x.4 as f64) * arg.sin();

        t *= if (x.1).abs() == 1 {
            E
        } else if (x.1).abs() == 2 {
            E * E
        } else {
            1.0
        };

        b += t;
    }

    l += 3958.0 * A1.sin() + 1962.0 * (L1 - F).sin() + 318.0 * A2.sin();

    b += -2235.0 * L1.sin()
        + 382.0 * A3.sin()
        + 175.0 * ((A1 - F).sin() + (A1 + F).sin())
        + 127.0 * (L1 - M1).sin()
        - 115.0 * (L1 + M1).sin();

    l = l.to_radians();
    b = b.to_radians();

    let ecl_point = coords::EclPoint {
        long: L1 + l / 1000000.0,
        lat: b / 1000000.0,
    };

    let rad_vec = 385000.56 + r / 1000.0;

    (ecl_point, rad_vec)
}

/**
Computes the longitude of the mean ascending node of the Moon

# Returns

* `long_mn_ascend_node`: Longitude of the mean ascending
                         node *| in radians*

# Arguments

* `JC`: Julian century
**/
pub fn mn_ascend_node(JC: f64) -> f64 {
    angle::limit_to_360(Horner_eval!(
        JC,
        125.0445479,
        -1934.1362891,
        0.0020754,
        1.0 / 467441.0,
        -1.0 / 60616000.0
    ))
    .to_radians()
}

/**
Computes the longitude of the true ascending node of the Moon

# Returns

* `long_true_ascend_node`: Longitude of the true ascending
                           node *| in radians*

# Arguments

* `JC`: Julian century
**/
pub fn true_ascend_node(JC: f64) -> f64 {
    let (D, M, M1) = DMM1(JC);
    let F = F(JC);

    mn_ascend_node(JC)
        + (-1.4979 * (2.0 * (D - F)).sin() - 0.1500 * M.sin() - 0.1226 * (2.0 * D).sin()
            + 0.1176 * (2.0 * F).sin()
            - 0.0801 * (2.0 * (M1 - F)).sin())
        .to_radians()
}

/**
Computes the longitude of the mean perigee of the Moon

# Returns

* `long_mn_perigee`: Longitude of mean perigee
                     *| in radians*

# Arguments

* `JC`: Julian century
**/
pub fn mn_perigee(JC: f64) -> f64 {
    angle::limit_to_360(Horner_eval!(
        JC,
        83.3532465,
        4069.0137287,
        -0.01032,
        -1.0 / 80053.0,
        1.0 / 18999000.0
    ))
    .to_radians()
}

/**
Computes the position angle of the Moon's bright limb

# Returns

* `pos_angl_of_bright_limb` The position angle of the midpoint
                            of the illuminated limb of the Moon
                            *| in radians*

# Arguments

* `sun_eq_point`: Equatorial coordinate of the Sun *| in radians*
* `moon_eq_point`: Equatorial coordinate of the Moon *| in radians*
**/
pub fn bright_limb(sun_eq_point: coords::EqPoint, moon_eq_point: coords::EqPoint) -> f64 {
    let a = sun_eq_point.dec.cos();
    let n = a * (sun_eq_point.asc - moon_eq_point.asc).sin();
    let d = sun_eq_point.dec.sin() * moon_eq_point.dec.cos()
        - moon_eq_point.dec.sin() * (sun_eq_point.asc - moon_eq_point.asc).cos() * a;

    n.atan2(d)
}

/**
Computes the illuminated fraction of the lunar disk, using equatorial
coordinates

# Arguments

* `sun_eq_point`   : Equatorial coordinate of the Sun *| in radians*
* `moon_eq_point`  : Equatorial coordinate of the Moon *| in radians*
* `earth_moon_dist`: Earth-Moon distance (in any unit, but same as
                     that of `earth_sun_dist`)
* `earth_sun_dist` : Earth-Sun distance (in any unit, but same as
                     that of `earth_moon_dist`)
**/
pub fn illum_frac_frm_eq_coords(
    sun_eq_point: &coords::EqPoint,
    moon_eq_point: &coords::EqPoint,
    earth_moon_dist: f64,
    earth_sun_dist: f64,
) -> f64 {
    illuminated_frac(
        sun_eq_point.anglr_sepr(&moon_eq_point).acos(),
        earth_moon_dist,
        earth_sun_dist,
    )
}

/**
Computes the illuminated fraction of the lunar disk, using eclipctic
coordinates

# Arguments

* `moon_long`      : Eclipctic longitude of the Moon *| in radians*
* `moon_lat`       : Eclipctic latitude of the Moon *| in radians*
* `sun_long`       : Eclipctic longitude of the Sun *| in radians*
* `earth_moon_dist`: Earth-Moon distance (in any unit, but same as
                     that of `earth_sun_dist`)
* `earth_sun_dist` : Earth-Sun distance (in any unit, but same as
                     that of `earth_moon_dist`)
**/
pub fn illum_frac_frm_ecl_coords(
    moon_long: f64,
    moon_lat: f64,
    sun_long: f64,
    earth_moon_dist: f64,
    earth_sun_dist: f64,
) -> f64 {
    illuminated_frac(
        (moon_lat.cos() * (moon_long - sun_long).cos()).acos(),
        earth_moon_dist,
        earth_sun_dist,
    )
}

#[inline]
fn illuminated_frac(moon_geocent_elong: f64, earth_moon_dist: f64, earth_sun_dist: f64) -> f64 {
    let i = (earth_sun_dist * moon_geocent_elong.sin())
        .atan2(earth_moon_dist - earth_sun_dist * moon_geocent_elong.cos());

    (1.0 + i.cos()) / 2.0
}

mod libration;

pub use self::libration::{
    optical_libr, physical_libr, pos_angl_of_axis_of_rot, topocent_libr_by_diff_corrections,
    total_libr,
};

use self::libration::{DMM1, E, F};

mod phases;

pub use self::phases::{Phase, time_of_passage_through_nodes, time_of_phase};
