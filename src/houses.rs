// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT
//! Tropical house geometry. TT supplies true obliquity; UT1 supplies Earth
//! rotation. Geographic longitude is east-positive. No civil-time resolution,
//! Earth-orientation observations, refraction or symbolic interpretation is
//! inferred here. Callers must disclose any UT1 approximation they supply.
use foundation::{
    EastLongitude, JulianDate, Latitude, Longitude, Model, TerrestrialTime, UniversalTime1,
};
use orientation;
use std::{f64::consts::PI, fmt};

pub const HOUSE_GEOMETRY: Model = Model::new("Turquet tropical horizon / IAU 2006/2000A", "1");
/// Whole Sign starts at the rising sign's zero degree. Equal starts at the
/// ascendant itself. Both progress in twelve equal 30-degree longitude spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseSystem {
    WholeSign,
    Equal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseError {
    UndefinedAscendant,
}
impl fmt::Display for HouseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "No unique rising ecliptic intersection at this latitude and sidereal time"
        )
    }
}
impl std::error::Error for HouseError {}
#[derive(Clone, Debug, PartialEq)]
pub struct Houses {
    system: HouseSystem,
    ascendant: Longitude,
    midheaven: Longitude,
    cusps: [Longitude; 12],
}
impl Houses {
    pub fn system(&self) -> HouseSystem {
        self.system
    }
    pub fn ascendant(&self) -> Longitude {
        self.ascendant
    }
    pub fn midheaven(&self) -> Longitude {
        self.midheaven
    }
    pub fn cusps(&self) -> &[Longitude; 12] {
        &self.cusps
    }
    pub fn model(&self) -> Model {
        HOUSE_GEOMETRY
    }
    /// House number 1..=12, including its leading cusp and excluding the next.
    /// This longitude-only assignment deliberately ignores ecliptic latitude.
    pub fn house_of(&self, longitude: Longitude) -> u8 {
        let delta = (longitude.radians() - self.cusps[0].radians()).rem_euclid(2.0 * PI);
        self.cusps
            .iter()
            .rposition(|cusp| {
                delta >= (cusp.radians() - self.cusps[0].radians()).rem_euclid(2.0 * PI)
            })
            .unwrap_or(0) as u8
            + 1
    }
}
/// Calculate airless tropical geometry using geodetic latitude and the local
/// apparent sidereal meridian, without polar-motion corrections. Epochs are
/// typed separately so TT can never silently stand in for UT1.
///
/// ```compile_fail
/// use turquet::{houses, foundation::*};
/// let tt = JulianDate::<TerrestrialTime>::from_julian_day(2451545.0).unwrap();
/// houses::calculate(tt, tt, Latitude::from_degrees(40.0).unwrap(),
///     EastLongitude::from_degrees(-74.0).unwrap(), houses::HouseSystem::Equal);
/// ```
pub fn calculate(
    tt: JulianDate<TerrestrialTime>,
    ut1: JulianDate<UniversalTime1>,
    latitude: Latitude,
    longitude: EastLongitude,
    system: HouseSystem,
) -> Result<Houses, HouseError> {
    let (ua, ub) = ut1.parts();
    let (ta, tb) = tt.parts();
    let theta = sofars::erst::gst06a(ua, ub, ta, tb) + longitude.radians();
    geometry(
        theta,
        orientation::true_obliquity(tt).into_value().radians(),
        latitude.radians(),
        system,
    )
}
fn geometry(theta: f64, epsilon: f64, phi: f64, system: HouseSystem) -> Result<Houses, HouseError> {
    let (st, ct) = theta.sin_cos();
    let (se, ce) = epsilon.sin_cos();
    let (sp, cp) = phi.sin_cos();
    // For an ecliptic direction (cos λ, sin λ cos ε, sin λ sin ε),
    // the zenith dot product is A cos λ + B sin λ. Its roots are opposite.
    let a = cp * ct;
    let b = cp * st * ce + sp * se;
    if cp.abs() < 1e-12 || a.hypot(b) < 1e-12 {
        return Err(HouseError::UndefinedAscendant);
    }
    let mut asc = a.atan2(-b);
    // Choose the root whose altitude increases as Earth rotates: rising,
    // rather than the opposite (setting) intersection, including south/polar latitudes.
    let rate = cp * (-st * asc.cos() + ct * asc.sin() * ce);
    if rate.abs() < 1e-12 {
        return Err(HouseError::UndefinedAscendant);
    }
    if rate < 0.0 {
        asc += PI;
    }
    let ascendant = Longitude::from_radians(asc).unwrap();
    let midheaven = Longitude::from_radians(st.atan2(ct * ce)).unwrap();
    let first = match system {
        HouseSystem::WholeSign => (ascendant.degrees() / 30.0).floor() * 30.0,
        HouseSystem::Equal => ascendant.degrees(),
    };
    let cusps = std::array::from_fn(|i| Longitude::from_degrees(first + i as f64 * 30.0).unwrap());
    Ok(Houses {
        system,
        ascendant,
        midheaven,
        cusps,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn degenerate_horizon_and_poles_refuse() {
        for phi in [PI / 2.0, -PI / 2.0] {
            assert_eq!(
                geometry(0.0, 0.4, phi, HouseSystem::Equal),
                Err(HouseError::UndefinedAscendant)
            );
        }
        assert_eq!(
            geometry(3.0 * PI / 2.0, 0.4, PI / 2.0 - 0.4, HouseSystem::Equal),
            Err(HouseError::UndefinedAscendant)
        );
    }
    #[test]
    fn horizon_root_is_rising_in_both_hemispheres_and_wraps() {
        for latitude in [-80.0_f64, -40.0, 0.0, 40.0, 80.0] {
            for degrees in [0.0_f64, 40.0, 90.0, 180.0, 270.0, 359.9] {
                let theta = degrees.to_radians();
                let phi = latitude.to_radians();
                let eps = 23.4_f64.to_radians();
                let h = geometry(theta, eps, phi, HouseSystem::Equal).unwrap();
                let l = h.ascendant().radians();
                let altitude = |t: f64| {
                    phi.cos() * t.cos() * l.cos()
                        + (phi.cos() * t.sin() * eps.cos() + phi.sin() * eps.sin()) * l.sin()
                };
                assert!(altitude(theta).abs() < 1e-12);
                assert!(altitude(theta + 1e-5) > altitude(theta - 1e-5));
                for i in 0..12 {
                    let cusp = h.cusps()[i];
                    assert_eq!(
                        h.house_of(Longitude::from_degrees(cusp.degrees() + 0.001).unwrap()),
                        i as u8 + 1
                    );
                }
            }
        }
    }
}
