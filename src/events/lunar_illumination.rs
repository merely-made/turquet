// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Geocentric apparent lunar illumination facts.
//!
//! This module derives the illuminated lunar fraction from a same-epoch
//! Sun-Moon-Earth triangle in the provider's true ecliptic of date. It does
//! not make a topocentric, lunar-libration, limb-profile, or visibility claim.

use apparent::ApparentBody;
use foundation::{Angle, Distance, JulianDate, TerrestrialTime};
use provider::GeocentricPositionProvider;

mod types;
pub use self::types::{LunarIllumination, LunarIlluminationError, GEOCENTRIC_LUNAR_ILLUMINATION};

/// Derive the geocentric apparent illuminated lunar fraction at one TT epoch.
///
/// The source states must share the provider's true-ecliptic-of-date frame.
/// The result is a geometric Sun-Moon-Earth triangle: it does not select an
/// observer, atmosphere, lunar limb model, or a human visibility convention.
pub fn lunar_illumination_at<P>(
    positions: &P,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<LunarIllumination, LunarIlluminationError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let moon = positions
        .position(ApparentBody::Moon, epoch)
        .map_err(|source| LunarIlluminationError::Position {
            body: ApparentBody::Moon,
            epoch,
            source,
        })?;
    require_state_epoch(ApparentBody::Moon, epoch, moon.epoch())?;
    let sun = positions
        .position(ApparentBody::Sun, epoch)
        .map_err(|source| LunarIlluminationError::Position {
            body: ApparentBody::Sun,
            epoch,
            source,
        })?;
    require_state_epoch(ApparentBody::Sun, epoch, sun.epoch())?;
    let moon_distance = moon.distance();
    let sun_distance = sun.distance();
    require_nonzero_distance(ApparentBody::Moon, epoch, moon_distance)?;
    require_nonzero_distance(ApparentBody::Sun, epoch, sun_distance)?;

    let moon_direction = moon.direction().to_unit_vector().components();
    let sun_direction = sun.direction().to_unit_vector().components();
    let moon_vector = scale(moon_direction, moon_distance.meters());
    let sun_vector = scale(sun_direction, sun_distance.meters());
    let moon_to_sun = subtract(sun_vector, moon_vector);
    let moon_sun_distance_meters = norm(moon_to_sun);
    if !moon_sun_distance_meters.is_finite() {
        return Err(LunarIlluminationError::NonFiniteTriangle { epoch });
    }
    if moon_sun_distance_meters == 0.0 {
        return Err(LunarIlluminationError::CoincidentSunAndMoon { epoch });
    }

    let moon_sun_distance = Distance::from_meters(moon_sun_distance_meters)
        .expect("a finite positive Sun-Moon leg is a valid distance");
    let elongation = angle_between(moon_direction, sun_direction);
    let phase_angle = angle_between(negate(moon_direction), normalize(moon_to_sun));
    let illuminated_fraction = (1.0 + phase_angle.radians().cos()) / 2.0;
    if !illuminated_fraction.is_finite() || illuminated_fraction < 0.0 || illuminated_fraction > 1.0
    {
        return Err(LunarIlluminationError::NonFiniteTriangle { epoch });
    }

    Ok(LunarIllumination {
        epoch,
        illuminated_fraction,
        elongation,
        phase_angle,
        moon_distance,
        sun_distance,
        moon_sun_distance,
        illumination_model: GEOCENTRIC_LUNAR_ILLUMINATION,
        provider_model: positions.model(),
        provider_snapshot: positions.data_snapshot().map(str::to_owned),
    })
}

fn require_state_epoch<P>(
    body: ApparentBody,
    expected_epoch: JulianDate<TerrestrialTime>,
    actual_epoch: JulianDate<TerrestrialTime>,
) -> Result<(), LunarIlluminationError<P>> {
    if actual_epoch != expected_epoch {
        return Err(LunarIlluminationError::StateEpochMismatch {
            body,
            expected_epoch,
            actual_epoch,
        });
    }
    Ok(())
}

fn require_nonzero_distance<P>(
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    distance: Distance,
) -> Result<(), LunarIlluminationError<P>> {
    if distance.meters() == 0.0 {
        return Err(LunarIlluminationError::BodyAtObserver {
            body,
            epoch,
            distance,
        });
    }
    Ok(())
}

fn scale(vector: [f64; 3], scalar: f64) -> [f64; 3] {
    [vector[0] * scalar, vector[1] * scalar, vector[2] * scalar]
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn negate(vector: [f64; 3]) -> [f64; 3] {
    [-vector[0], -vector[1], -vector[2]]
}

fn norm(vector: [f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn normalize(vector: [f64; 3]) -> [f64; 3] {
    let length = norm(vector);
    [vector[0] / length, vector[1] / length, vector[2] / length]
}

fn angle_between(first: [f64; 3], second: [f64; 3]) -> Angle {
    let dot = first[0] * second[0] + first[1] * second[1] + first[2] * second[2];
    let cross = [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ];
    Angle::from_radians(norm(cross).atan2(dot))
        .expect("finite normalized directions have a finite angular separation")
}
