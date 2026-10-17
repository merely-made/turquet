// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT
//! Tropical house geometry. TT supplies true obliquity; UT1 supplies Earth
//! rotation. Geographic longitude is east-positive. No civil-time resolution,
//! Earth-orientation observations, refraction or symbolic interpretation is
//! inferred here. Callers must disclose any UT1 approximation they supply.
use foundation::{
    EastLongitude, JulianDate, Latitude, Longitude, Model, TerrestrialTime, UniversalTime1,
};
use observer::EarthOrientation;
use orientation;
use std::{f64::consts::PI, fmt};

pub const HOUSE_GEOMETRY: Model = Model::new("Turquet tropical horizon / IAU 2006/2000A", "1");
/// Semi-arc Placidus geometry using the legacy uncorrected horizon.
pub const PLACIDUS_HOUSE_GEOMETRY: Model =
    Model::new("Turquet tropical Placidus semi-arcs / IAU 2006/2000A", "2");
/// Caller-EOP geometry using the inverse-pom00 geodetic zenith horizon.
/// The selected house division remains explicit in [`Houses::system`].
pub const OBSERVED_HOUSE_GEOMETRY: Model = Model::new(
    "Turquet observed tropical geodetic horizon / IAU 2006/2000A",
    "2",
);
/// Whole Sign starts at the rising sign's zero degree. Equal starts at the
/// ascendant itself. Both progress in twelve equal 30-degree longitude spans.
/// Placidus divides diurnal/nocturnal semi-arcs and has unequal spans; it is
/// unavailable at or beyond the polar circle and never falls back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseSystem {
    WholeSign,
    Equal,
    Placidus,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseError {
    UndefinedAscendant,
    /// Circumpolar geometry, a degenerate horizon, or an unsolved cusp prevents
    /// a complete, ordered Placidus result. No substitute cusps are returned.
    PlacidusUnavailable,
}
impl fmt::Display for HouseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match self {
            HouseError::UndefinedAscendant => {
                "No unique rising ecliptic intersection at this latitude and sidereal time"
            },
            HouseError::PlacidusUnavailable => {
                "Placidus houses unavailable: circumpolar geometry or unsolved cusps"
            },
        })
    }
}
impl std::error::Error for HouseError {}
#[derive(Clone, Debug, PartialEq)]
pub struct Houses {
    system: HouseSystem,
    model: Model,
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
        self.model
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
/// Calculate airless houses with an immutable, caller-supplied EOP snapshot.
/// UT1 and polar motion come only from `earth_orientation`; no observations
/// are fetched. Inverse SOFA `pom00` rotates the ITRS geodetic zenith into
/// TIRS, defining both the corrected horizon latitude and meridian longitude.
/// This uses the geodetic surface normal, not the geocentric site position;
/// height and refraction do not enter this directional horizon geometry.
/// The legacy [`calculate`] path retains its exact uncorrected behavior.
pub fn calculate_observed(
    tt: JulianDate<TerrestrialTime>,
    earth_orientation: &EarthOrientation,
    latitude: Latitude,
    longitude: EastLongitude,
    system: HouseSystem,
) -> Result<Houses, HouseError> {
    let (ta, tb) = tt.parts();
    let (ua, ub) = earth_orientation.ut1().parts();
    let pom = sofars::pnp::pom00(
        earth_orientation.polar_motion_x().radians(),
        earth_orientation.polar_motion_y().radians(),
        sofars::pnp::sp00(ta, tb),
    );
    let (sp, cp) = latitude.radians().sin_cos();
    let (sl, cl) = longitude.radians().sin_cos();
    let itrs = [cp * cl, cp * sl, sp];
    // pom maps TIRS to ITRS, so its transpose maps the zenith back to TIRS.
    let zenith: [f64; 3] =
        std::array::from_fn(|i| pom[0][i] * itrs[0] + pom[1][i] * itrs[1] + pom[2][i] * itrs[2]);
    let mut houses = geometry(
        sofars::erst::gst06a(ua, ub, ta, tb) + zenith[1].atan2(zenith[0]),
        orientation::true_obliquity(tt).into_value().radians(),
        zenith[2].atan2(zenith[0].hypot(zenith[1])),
        system,
    )?;
    houses.model = OBSERVED_HOUSE_GEOMETRY;
    Ok(houses)
}
fn geometry(theta: f64, epsilon: f64, phi: f64, system: HouseSystem) -> Result<Houses, HouseError> {
    if system == HouseSystem::Placidus
        && (!theta.is_finite()
            || !epsilon.is_finite()
            || !phi.is_finite()
            || phi.abs() + epsilon.abs() >= PI / 2.0)
    {
        return Err(HouseError::PlacidusUnavailable);
    }
    let undefined = if system == HouseSystem::Placidus {
        HouseError::PlacidusUnavailable
    } else {
        HouseError::UndefinedAscendant
    };
    let (st, ct) = theta.sin_cos();
    let (se, ce) = epsilon.sin_cos();
    let (sp, cp) = phi.sin_cos();
    // For an ecliptic direction (cos λ, sin λ cos ε, sin λ sin ε),
    // the zenith dot product is A cos λ + B sin λ. Its roots are opposite.
    let a = cp * ct;
    let b = cp * st * ce + sp * se;
    if cp.abs() < 1e-12 || a.hypot(b) < 1e-12 {
        return Err(undefined);
    }
    let mut asc = a.atan2(-b);
    // Choose the root whose altitude increases as Earth rotates: rising,
    // rather than the opposite (setting) intersection, including south/polar latitudes.
    let rate = cp * (-st * asc.cos() + ct * asc.sin() * ce);
    if rate.abs() < 1e-12 {
        return Err(undefined);
    }
    if rate < 0.0 {
        asc += PI;
    }
    let ascendant = Longitude::from_radians(asc).unwrap();
    let midheaven = Longitude::from_radians(st.atan2(ct * ce)).unwrap();
    if system == HouseSystem::Placidus {
        return Ok(Houses {
            system,
            model: PLACIDUS_HOUSE_GEOMETRY,
            ascendant,
            midheaven,
            cusps: placidus_cusps(theta, epsilon, phi, ascendant, midheaven)?,
        });
    }
    let first = match system {
        HouseSystem::WholeSign => (ascendant.degrees() / 30.0).floor() * 30.0,
        HouseSystem::Equal => ascendant.degrees(),
        HouseSystem::Placidus => unreachable!("handled above"),
    };
    let mut cusps =
        std::array::from_fn(|i| Longitude::from_degrees(first + i as f64 * 30.0).unwrap());
    // Retain the exact angular value at the defining Equal cusp. A degrees
    // round trip can otherwise put the ascendant infinitesimally before cusp 1.
    if system == HouseSystem::Equal {
        cusps[0] = ascendant;
    }
    Ok(Houses {
        system,
        model: HOUSE_GEOMETRY,
        ascendant,
        midheaven,
        cusps,
    })
}

/// Solve the semi-arc definition directly, without Swiss source or runtime.
/// See https://www.astro.com/swisseph/swisseph.htm, section 6.2.1.
fn placidus_cusps(
    theta: f64,
    epsilon: f64,
    phi: f64,
    ascendant: Longitude,
    midheaven: Longitude,
) -> Result<[Longitude; 12], HouseError> {
    let theta = theta.rem_euclid(2.0 * PI);
    let (se, ce) = epsilon.sin_cos();
    let tp = phi.tan();
    // x = right ascension east of the upper meridian, in [0, pi].
    // For declination delta the rising hour angle has magnitude
    // H0 = acos(-tan(phi) tan(delta)); the nocturnal semi-arc is pi-H0.
    let solve = |fraction: f64, nocturnal: bool| -> Result<Longitude, HouseError> {
        let residual = |x: f64| -> Result<f64, HouseError> {
            let alpha = theta + x;
            let lambda = alpha.sin().atan2(ce * alpha.cos());
            let sd = se * lambda.sin();
            let horizon = -tp * sd / (1.0 - sd * sd).sqrt();
            if !horizon.is_finite() || horizon.abs() >= 1.0 {
                return Err(HouseError::PlacidusUnavailable);
            }
            let h0 = horizon.acos();
            Ok(x - if nocturnal {
                PI - fraction * (PI - h0)
            } else {
                fraction * h0
            })
        };
        let (mut lo, mut hi) = (0.0, PI);
        if residual(lo)? >= 0.0 || residual(hi)? <= 0.0 {
            return Err(HouseError::PlacidusUnavailable);
        }
        // Bisection remains bounded even near the polar circle, where a
        // fixed-point iteration can converge very slowly or fail altogether.
        for _ in 0..64 {
            let x = (lo + hi) / 2.0;
            let r = residual(x)?;
            if r.abs() < 1e-13 {
                let alpha = theta + x;
                return Longitude::from_radians(alpha.sin().atan2(ce * alpha.cos()))
                    .map_err(|_| HouseError::PlacidusUnavailable);
            }
            if r < 0.0 {
                lo = x;
            } else {
                hi = x;
            }
        }
        Err(HouseError::PlacidusUnavailable)
    };
    let opposite = |l: Longitude| Longitude::from_radians(l.radians() + PI).unwrap();
    let mut cusps = [ascendant; 12];
    cusps[9] = midheaven;
    cusps[10] = solve(1.0 / 3.0, false)?;
    cusps[11] = solve(2.0 / 3.0, false)?;
    cusps[1] = solve(2.0 / 3.0, true)?;
    cusps[2] = solve(1.0 / 3.0, true)?;
    for i in [0, 1, 2, 9, 10, 11] {
        cusps[(i + 6) % 12] = opposite(cusps[i]);
    }
    let offsets = cusps.map(|l| (l.radians() - ascendant.radians()).rem_euclid(2.0 * PI));
    if offsets.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(HouseError::PlacidusUnavailable);
    }
    Ok(cusps)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placidus_polar_boundary_and_nonfinite_solver_inputs_refuse() {
        let eps = 23.4_f64.to_radians();
        for sign in [-1.0, 1.0] {
            for excess in [0.0, 1e-12, 0.01] {
                for theta in [0.0, 1.0, 3.0, 5.0] {
                    assert_eq!(
                        geometry(
                            theta,
                            eps,
                            sign * (PI / 2.0 - eps + excess),
                            HouseSystem::Placidus
                        ),
                        Err(HouseError::PlacidusUnavailable)
                    );
                }
            }
            for theta in [0.0, 1.0, 3.0, 5.0] {
                assert!(
                    geometry(
                        theta,
                        eps,
                        sign * (PI / 2.0 - eps - 1e-8),
                        HouseSystem::Placidus
                    )
                    .is_ok()
                );
            }
        }
        for (theta, eps, phi) in [
            (f64::NAN, eps, 0.0),
            (0.0, f64::NAN, 0.0),
            (0.0, eps, f64::INFINITY),
        ] {
            assert_eq!(
                geometry(theta, eps, phi, HouseSystem::Placidus),
                Err(HouseError::PlacidusUnavailable)
            );
        }
        // Exercise the solver's own rejection path, independent of the public
        // polar-circle guard (a circumpolar trial direction has no semi-arc).
        assert_eq!(
            placidus_cusps(
                0.0,
                eps,
                80_f64.to_radians(),
                Longitude::from_degrees(90.0).unwrap(),
                Longitude::from_degrees(0.0).unwrap()
            ),
            Err(HouseError::PlacidusUnavailable)
        );
    }
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
                assert_eq!(h.house_of(h.ascendant()), 1);
                assert_eq!(h.house_of(Longitude::from_radians(l - 1e-10).unwrap()), 12);
                assert_eq!(h.house_of(Longitude::from_radians(l + 1e-10).unwrap()), 1);
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
