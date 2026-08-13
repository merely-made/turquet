// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Apparent geocentric ecliptic positions for the ten classical chart bodies.
//!
//! This is the first Turquet-era module and the T3 down payment: one pipeline
//! that composes the inherited VSOP87D planetary theory, the partial
//! ELP-2000/82 lunar solution, the analytical Pluto series, the IAU 1980
//! nutation, and the inherited ecliptic precession into apparent positions on
//! the true equinox of date. No external crate and no data file is involved.
//!
//! Validation authority: NASA/JPL Horizons observer-table quantity 31
//! (`tests/apparent.rs`), at J2000, the 2024-04-08 total solar eclipse, and
//! 2026-08-13. Measured residuals: the Sun, Moon, and eight planets agree
//! with Horizons at millidegree rounding (worst 2 millidegrees); Pluto agrees
//! within 14 millidegrees, limited by its truncated series.
//!
//! Explicit stages, per the roadmap: heliocentric position of date
//! (ELP-2000/82 is directly geocentric; Pluto is precessed from its J2000
//! frame with the inherited ecliptic precession), light-time iteration,
//! annual aberration from a numerically differentiated Earth velocity, and
//! nutation in longitude. Solar gravitational deflection is not applied.
//!
//! Supported range: the UTC conversion is defined from 1972 (the leap-second
//! era) and the Pluto series is stated for 1885 to 2099. Requests outside a
//! defined range are errors, never silent degradation.

use angle;
use lunar;
use nutation;
use planet;
use pluto;
use precess;

/// Astronomical units travelled by light in one day.
const LIGHT_SPEED_AU_PER_DAY: f64 = 173.144_632_674_24;
/// Central-difference step for Earth's velocity, in days.
const VELOCITY_STEP_DAYS: f64 = 0.01;
/// Central-difference step for the retrograde test, in days.
const RETROGRADE_STEP_DAYS: f64 = 0.5;
const J2000_JD: f64 = 2_451_545.0;
/// TT - TAI, the fixed offset in seconds.
const TT_MINUS_TAI_SECONDS: f64 = 32.184;
/// The Pluto series' stated validity, in Julian years.
const PLUTO_RANGE_YEARS: (f64, f64) = (1885.0, 2099.0);

/// TAI - UTC in seconds, keyed by the Gregorian date the value took effect.
const LEAP_SECONDS: [(i32, u32, u32, f64); 28] = [
    (1972, 1, 1, 10.0),
    (1972, 7, 1, 11.0),
    (1973, 1, 1, 12.0),
    (1974, 1, 1, 13.0),
    (1975, 1, 1, 14.0),
    (1976, 1, 1, 15.0),
    (1977, 1, 1, 16.0),
    (1978, 1, 1, 17.0),
    (1979, 1, 1, 18.0),
    (1980, 1, 1, 19.0),
    (1981, 7, 1, 20.0),
    (1982, 7, 1, 21.0),
    (1983, 7, 1, 22.0),
    (1985, 7, 1, 23.0),
    (1988, 1, 1, 24.0),
    (1990, 1, 1, 25.0),
    (1991, 1, 1, 26.0),
    (1992, 7, 1, 27.0),
    (1993, 7, 1, 28.0),
    (1994, 7, 1, 29.0),
    (1996, 1, 1, 30.0),
    (1997, 7, 1, 31.0),
    (1999, 1, 1, 32.0),
    (2006, 1, 1, 33.0),
    (2009, 1, 1, 34.0),
    (2012, 7, 1, 35.0),
    (2015, 7, 1, 36.0),
    (2017, 1, 1, 37.0),
];

/// The ten bodies of the apparent pipeline, in conventional chart order.
pub const APPARENT_BODIES: [ApparentBody; 10] = [
    ApparentBody::Sun,
    ApparentBody::Moon,
    ApparentBody::Mercury,
    ApparentBody::Venus,
    ApparentBody::Mars,
    ApparentBody::Jupiter,
    ApparentBody::Saturn,
    ApparentBody::Uranus,
    ApparentBody::Neptune,
    ApparentBody::Pluto,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApparentBody {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Pluto,
}

impl ApparentBody {
    pub fn name(&self) -> &'static str {
        match *self {
            ApparentBody::Sun => "sun",
            ApparentBody::Moon => "moon",
            ApparentBody::Mercury => "mercury",
            ApparentBody::Venus => "venus",
            ApparentBody::Mars => "mars",
            ApparentBody::Jupiter => "jupiter",
            ApparentBody::Saturn => "saturn",
            ApparentBody::Uranus => "uranus",
            ApparentBody::Neptune => "neptune",
            ApparentBody::Pluto => "pluto",
        }
    }

    /// Heliocentric rectangular coordinates on the mean ecliptic of date, in
    /// AU. The Sun sits at the origin by construction; the Moon is geocentric
    /// and never enters this path.
    fn heliocent_rect(&self, jde_tt: f64) -> [f64; 3] {
        match *self {
            ApparentBody::Sun => [0.0, 0.0, 0.0],
            ApparentBody::Moon => {
                unreachable!("the Moon is geocentric and handled in apparent position")
            }
            ApparentBody::Mercury => planet_rect(&planet::Planet::Mercury, jde_tt),
            ApparentBody::Venus => planet_rect(&planet::Planet::Venus, jde_tt),
            ApparentBody::Mars => planet_rect(&planet::Planet::Mars, jde_tt),
            ApparentBody::Jupiter => planet_rect(&planet::Planet::Jupiter, jde_tt),
            ApparentBody::Saturn => planet_rect(&planet::Planet::Saturn, jde_tt),
            ApparentBody::Uranus => planet_rect(&planet::Planet::Uranus, jde_tt),
            ApparentBody::Neptune => planet_rect(&planet::Planet::Neptune, jde_tt),
            ApparentBody::Pluto => pluto_rect_of_date(jde_tt),
        }
    }
}

/// An epoch outside a model's stated validity.
#[derive(Clone, Debug, PartialEq)]
pub enum ApparentError {
    /// The UTC instant precedes the 1972 leap-second era.
    BeforeLeapSecondEra,
    /// The calendar fields do not form a valid UTC instant.
    InvalidCivilTime,
    /// The requested epoch is outside the named body's series validity.
    OutsideSeriesRange {
        body: &'static str,
        julian_year: f64,
    },
}

/// Julian day in Terrestrial Time for a proleptic-Gregorian UTC instant.
/// Defined from 1972; earlier instants have no tabulated UTC offset here.
pub fn jde_tt_frm_utc(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: f64,
) -> Result<f64, ApparentError> {
    let civil_ok = month >= 1
        && month <= 12
        && day >= 1
        && day <= 31
        && hour <= 23
        && minute <= 59
        && second >= 0.0
        && second < 61.0;
    if !civil_ok {
        return Err(ApparentError::InvalidCivilTime);
    }
    let mut leap = None;
    for &(leap_year, leap_month, leap_day, seconds) in LEAP_SECONDS.iter() {
        if (year, month, day) >= (leap_year, leap_month, leap_day) {
            leap = Some(seconds);
        }
    }
    let leap = match leap {
        Some(seconds) => seconds,
        None => return Err(ApparentError::BeforeLeapSecondEra),
    };
    let day_fraction = day as f64 + (hour as f64 + (minute as f64 + second / 60.0) / 60.0) / 24.0;
    Ok(gregorian_julian_day(year, month, day_fraction) + (TT_MINUS_TAI_SECONDS + leap) / 86_400.0)
}

/// Apparent geocentric ecliptic longitude and latitude on the true equinox of
/// date, in radians, with longitude normalized to `[0, 2pi)`.
pub fn geocent_apparent_ecl_pos(
    body: &ApparentBody,
    jde_tt: f64,
) -> Result<(f64, f64), ApparentError> {
    check_range(body, jde_tt)?;
    if let ApparentBody::Moon = *body {
        return Ok(moon_apparent(jde_tt));
    }

    let earth = planet_rect(&planet::Planet::Earth, jde_tt);

    // Light-time: the body is seen where it was when the light left it.
    let mut offset = [0.0_f64; 3];
    let mut light_time_days = 0.0;
    for _ in 0..3 {
        let target = body.heliocent_rect(jde_tt - light_time_days);
        offset = [
            target[0] - earth[0],
            target[1] - earth[1],
            target[2] - earth[2],
        ];
        light_time_days = norm(&offset) / LIGHT_SPEED_AU_PER_DAY;
    }

    // Annual aberration displaces the apparent direction toward the
    // observer's motion. Earth's velocity comes from a central difference
    // rather than a memorised orbit constant.
    let velocity = earth_velocity(jde_tt);
    let direction = normalize(&offset);
    let aberrated = normalize(&[
        direction[0] + velocity[0] / LIGHT_SPEED_AU_PER_DAY,
        direction[1] + velocity[1] / LIGHT_SPEED_AU_PER_DAY,
        direction[2] + velocity[2] / LIGHT_SPEED_AU_PER_DAY,
    ]);

    // Mean equinox of date to true equinox of date.
    let (nut_in_long, _) = nutation::nutation(jde_tt);
    let longitude = angle::limit_to_two_PI(aberrated[1].atan2(aberrated[0]) + nut_in_long);
    let latitude = aberrated[2].asin();
    Ok((longitude, latitude))
}

/// Whether the body's apparent longitude is decreasing at the epoch, from a
/// central difference one half day on either side. The range check therefore
/// covers the full sampled interval.
pub fn is_retrograde(body: &ApparentBody, jde_tt: f64) -> Result<bool, ApparentError> {
    let before = geocent_apparent_ecl_pos(body, jde_tt - RETROGRADE_STEP_DAYS)?.0;
    let after = geocent_apparent_ecl_pos(body, jde_tt + RETROGRADE_STEP_DAYS)?.0;
    let two_pi = 2.0 * ::std::f64::consts::PI;
    let delta = (after - before + ::std::f64::consts::PI).rem_euclid(two_pi) - ::std::f64::consts::PI;
    Ok(delta < 0.0)
}

/// The Moon from the inherited partial ELP-2000/82: already geocentric on the
/// mean equinox of date, so no light-time loop or aberration term is added;
/// apparent position is the series value plus nutation.
fn moon_apparent(jde_tt: f64) -> (f64, f64) {
    let (point, _distance_km) = lunar::geocent_ecl_pos(jde_tt);
    let (nut_in_long, _) = nutation::nutation(jde_tt);
    (angle::limit_to_two_PI(point.long + nut_in_long), point.lat)
}

fn check_range(body: &ApparentBody, jde_tt: f64) -> Result<(), ApparentError> {
    if let ApparentBody::Pluto = *body {
        let julian_year = 2000.0 + (jde_tt - J2000_JD) / 365.25;
        if julian_year < PLUTO_RANGE_YEARS.0 || julian_year > PLUTO_RANGE_YEARS.1 {
            return Err(ApparentError::OutsideSeriesRange {
                body: "pluto",
                julian_year: julian_year,
            });
        }
    }
    Ok(())
}

fn planet_rect(planet: &planet::Planet, jde: f64) -> [f64; 3] {
    let (longitude, latitude, radius) = planet::heliocent_coords(planet, jde);
    spherical_to_rect(longitude, latitude, radius)
}

/// The inherited Pluto series is referred to the standard equinox of J2000.0;
/// the rest of the pipeline is referred to the equinox of date. The inherited
/// ecliptic precession carries it across before the frames are mixed.
fn pluto_rect_of_date(jde: f64) -> [f64; 3] {
    let (longitude_j2000, latitude_j2000, radius) = pluto::heliocent_pos(jde);
    let (longitude, latitude) =
        precess::precess_ecl_coords(longitude_j2000, latitude_j2000, J2000_JD, jde);
    spherical_to_rect(longitude, latitude, radius)
}

fn spherical_to_rect(longitude: f64, latitude: f64, radius: f64) -> [f64; 3] {
    [
        radius * latitude.cos() * longitude.cos(),
        radius * latitude.cos() * longitude.sin(),
        radius * latitude.sin(),
    ]
}

fn earth_velocity(jde: f64) -> [f64; 3] {
    let before = planet_rect(&planet::Planet::Earth, jde - VELOCITY_STEP_DAYS);
    let after = planet_rect(&planet::Planet::Earth, jde + VELOCITY_STEP_DAYS);
    let scale = 2.0 * VELOCITY_STEP_DAYS;
    [
        (after[0] - before[0]) / scale,
        (after[1] - before[1]) / scale,
        (after[2] - before[2]) / scale,
    ]
}

fn norm(vector: &[f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn normalize(vector: &[f64; 3]) -> [f64; 3] {
    let length = norm(vector);
    if length == 0.0 {
        return *vector;
    }
    [
        vector[0] / length,
        vector[1] / length,
        vector[2] / length,
    ]
}

fn gregorian_julian_day(year: i32, month: u32, day_fraction: f64) -> f64 {
    let (year, month) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let century = (year as f64 / 100.0).floor();
    let gregorian = 2.0 - century + (century / 4.0).floor();
    (365.25 * (year as f64 + 4_716.0)).floor()
        + (30.6001 * (month as f64 + 1.0)).floor()
        + day_fraction
        + gregorian
        - 1_524.5
}
