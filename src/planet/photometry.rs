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

/**
Computes the illuminated fraction of a planet's disk from it's phase
angle

# Returns

* `illum_frac`: Illuminated fraction of the planet's disk

# Arguments

* `i`: Phase angle of the planet *| in radians*
**/
#[inline]
pub fn illum_frac_frm_phase_angl(i: f64) -> f64 {
    (1.0 + i.cos()) / 2.0
}

/**
Computes the illuminated fraction of a planet's disk from it's distance
to the Sun and Earth

# Returns

* `illum_frac`: Illuminated fraction of the planet's disk

# Arguments

* `r`    : Planet-Sun distance *| in AU*
* `delta`: Planet-Earth distance *| in AU*
* `R`    : Sun-Earth distance *| in AU*
**/
#[inline]
pub fn illum_frac_frm_dist(r: f64, delta: f64, R: f64) -> f64 {
    let x = r + delta;

    (x * x - R * R) / (4.0 * r * delta)
}

/**
Computes a planet's phase angle

# Returns

* `phase_angl`: Phase angle of the planet *| in radians*

# Arguments

* `r`    : Planet-Sun distance *| in AU*
* `delta`: Planet-Earth distance *| in AU*
* `R`    : Sun-Earth distance *| in AU*
**/
#[inline]
pub fn phase_angl(r: f64, delta: f64, R: f64) -> f64 {
    (r * r + delta * delta - R * R) / (2.0 * r * delta)
}

/**
Computes the position angle of the bright limb of a planet

# Returns

* `pos_angl_of_bright_limb`: The position angle of the midpoint of
                             the illuminated limb of a planet
                             *| in radians*

# Arguments

* `sun_eq_point`   : Equatorial point of the Sun *| in radians*
* `planet_eq_point`: Equatorial point of the planet *| in radians*
**/
pub fn pos_angle_of_bright_limb(
    sun_eq_point: coords::EqPoint,
    planet_eq_point: coords::EqPoint,
) -> f64 {
    let a = sun_eq_point.dec.cos();
    let n = a * (sun_eq_point.asc - planet_eq_point.asc).sin();
    let d = sun_eq_point.dec.sin() * planet_eq_point.dec.cos()
        - planet_eq_point.dec.sin() * (sun_eq_point.asc - planet_eq_point.asc).cos() * a;

    n.atan2(d)
}

/**
Computes a planet's equatorial semidiameter

# Returns

* `semidiameter`: Equatorial semidiameter *| in radians*

# Arguments

* `planet`           : The [Planet](./enum.Planet.html)
* `planet_earth_dist`: Planet-Earth distance *| in AU*
**/
pub fn semidiameter<'a>(planet: &Planet, planet_earth_dist: f64) -> Result<f64, &'a str> {
    let s = match *planet {
        Planet::Mercury => angle::deg_frm_dms(0, 0, 3.360).to_radians(),
        Planet::Venus => angle::deg_frm_dms(0, 0, 8.410).to_radians(),
        Planet::Mars => angle::deg_frm_dms(0, 0, 4.680).to_radians(),
        Planet::Uranus => angle::deg_frm_dms(0, 0, 35.02).to_radians(),
        Planet::Neptune => angle::deg_frm_dms(0, 0, 33.50).to_radians(),

        Planet::Jupiter => jupiter::eq_semidiameter(1.0),
        Planet::Saturn => saturn::eq_semidiameter(1.0),

        Planet::Earth => {
            return Err("Planet::Earth was passed to the function planet::semidiameter()");
        },
    };

    Ok(s / planet_earth_dist)
}

/**
Computes a planet's apparent magnitude using G. Muller's formulae

# Returns

* `app_mag`: Apparent magnitude of the planet

# Arguments

* `planet`: Any variant of [Planet](./enum.Planet.html)
* `i`     : Phase angle of the planet *| in radians*
* `delta` : Planet-Earth distance *| in AU*
* `r`     : Planet-Sun distance *| in AU*
**/
pub fn apprnt_mag_muller<'a>(planet: &Planet, i: f64, delta: f64, r: f64) -> Result<f64, &'a str> {
    let x = 5.0 * (r * delta).log10();

    match *planet {
        Planet::Mercury => Ok(x + 1.16 + (i - 50.0) * (0.02838 + (i - 50.0) * 0.000102)),
        Planet::Venus => Ok(x - 4.0 + i * (0.01322 + i * i * 0.0000004247)),
        Planet::Earth => {
            return Err("Planet::Earth was passed to the function planet::apprnt_mag_muller()");
        },
        Planet::Mars => Ok(x - 1.3 + i * 0.01486),
        Planet::Jupiter => Ok(x - 8.93),
        Planet::Saturn => {
            return Err(
                "Planet::Saturn was passed to the function planet::apprnt_mag_muller(). Use the function planet::saturn::apprnt_mag_muller() instead.",
            );
        },
        Planet::Uranus => Ok(x - 6.85),
        Planet::Neptune => Ok(x - 7.05),
    }
}

/**
Computes a planet's apparent magnitude using the Astronomical
Almanac's method adopted in 1984

# Returns

* `app_mag`: Apparent magnitude of the planet

# Arguments

* `planet`: Any variant of [Planet](./enum.Planet.html)
* `i`     : Phase angle of the planet *| in radians*
* `delta` : Planet-Earth distance *| in AU*
* `r`     : Planet-Sun distance *| in AU*
**/
pub fn apprnt_mag_84<'a>(planet: &Planet, i: f64, delta: f64, r: f64) -> Result<f64, &'a str> {
    let x = 5.0 * (r * delta).log10();

    match *planet {
        Planet::Mercury => Ok(x - 0.42 + i * (0.0380 - i * (0.000273 - i * 0.00000200))),
        Planet::Venus => Ok(x - 4.40 + i * (0.0009 + i * (0.000239 - i * 0.00000065))),
        Planet::Earth => {
            return Err("Planet::Earth was passed to the function planet::apprnt_mag_84()");
        },
        Planet::Mars => Ok(x - 1.52 + i * 0.016),
        Planet::Jupiter => Ok(x - 9.4 + i * 0.005),
        Planet::Saturn => {
            return Err(
                "Planet::Saturn was passed to the function planet::apprnt_mag_84(). Use the function planet::saturn::apprnt_mag_84() instead.",
            );
        },
        Planet::Uranus => Ok(x - 7.19),
        Planet::Neptune => Ok(x - 6.87),
    }
}
