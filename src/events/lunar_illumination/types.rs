// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use std::fmt;

use apparent::ApparentBody;
use foundation::{Angle, Distance, JulianDate, Model, TerrestrialTime};

/// Revisioned apparent Sun-Moon-Earth triangle used for the result.
///
/// It derives the fraction from same-epoch geocentric apparent states on the
/// true ecliptic and equinox of date. It does not model a topocentric view,
/// lunar libration or limb relief, atmospheric transmission, or visibility.
pub const GEOCENTRIC_LUNAR_ILLUMINATION: Model = Model::new(
    "geocentric apparent Sun-Moon-Earth illumination triangle",
    "1",
);

/// Geocentric apparent lunar illumination and its retained geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct LunarIllumination {
    pub(super) epoch: JulianDate<TerrestrialTime>,
    pub(super) illuminated_fraction: f64,
    pub(super) elongation: Angle,
    pub(super) phase_angle: Angle,
    pub(super) moon_distance: Distance,
    pub(super) sun_distance: Distance,
    pub(super) moon_sun_distance: Distance,
    pub(super) illumination_model: Model,
    pub(super) provider_model: Model,
    pub(super) provider_snapshot: Option<String>,
}

impl LunarIllumination {
    pub fn epoch(&self) -> JulianDate<TerrestrialTime> {
        self.epoch
    }

    /// Fraction of the lunar disk illuminated by the apparent Sun, in `[0, 1]`.
    pub fn illuminated_fraction(&self) -> f64 {
        self.illuminated_fraction
    }

    /// Apparent geocentric Sun-Moon center separation.
    pub fn elongation(&self) -> Angle {
        self.elongation
    }

    /// Apparent phase angle at the Moon, between Earth and the Sun.
    pub fn phase_angle(&self) -> Angle {
        self.phase_angle
    }

    pub fn moon_distance(&self) -> Distance {
        self.moon_distance
    }

    pub fn sun_distance(&self) -> Distance {
        self.sun_distance
    }

    pub fn moon_sun_distance(&self) -> Distance {
        self.moon_sun_distance
    }

    pub fn illumination_model(&self) -> Model {
        self.illumination_model
    }

    pub fn provider_model(&self) -> Model {
        self.provider_model
    }

    pub fn provider_snapshot(&self) -> Option<&str> {
        self.provider_snapshot.as_ref().map(String::as_str)
    }
}

/// A failure while deriving a lunar illumination fact.
#[derive(Debug)]
pub enum LunarIlluminationError<P> {
    Position {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        source: P,
    },
    StateEpochMismatch {
        body: ApparentBody,
        expected_epoch: JulianDate<TerrestrialTime>,
        actual_epoch: JulianDate<TerrestrialTime>,
    },
    BodyAtObserver {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        distance: Distance,
    },
    CoincidentSunAndMoon {
        epoch: JulianDate<TerrestrialTime>,
    },
    NonFiniteTriangle {
        epoch: JulianDate<TerrestrialTime>,
    },
}

impl<P: fmt::Display> fmt::Display for LunarIlluminationError<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LunarIlluminationError::Position {
                body,
                epoch,
                ref source,
            } => write!(
                formatter,
                "could not obtain {} apparent position at TT JD {}: {}",
                body.name(),
                epoch.day(),
                source
            ),
            LunarIlluminationError::StateEpochMismatch {
                body,
                expected_epoch,
                actual_epoch,
            } => write!(
                formatter,
                "{} provider state at TT JD {} was requested for TT JD {}",
                body.name(),
                actual_epoch.day(),
                expected_epoch.day()
            ),
            LunarIlluminationError::BodyAtObserver {
                body,
                epoch,
                distance,
            } => write!(
                formatter,
                "{} geocentric distance {} km is zero at TT JD {}",
                body.name(),
                distance.kilometers(),
                epoch.day()
            ),
            LunarIlluminationError::CoincidentSunAndMoon { epoch } => write!(
                formatter,
                "Sun and Moon occupy the same geocentric position at TT JD {}",
                epoch.day()
            ),
            LunarIlluminationError::NonFiniteTriangle { epoch } => write!(
                formatter,
                "Sun-Moon-Earth illumination triangle is not finite at TT JD {}",
                epoch.day()
            ),
        }
    }
}

impl<P> ::std::error::Error for LunarIlluminationError<P>
where
    P: ::std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            LunarIlluminationError::Position { ref source, .. } => Some(source),
            LunarIlluminationError::StateEpochMismatch { .. }
            | LunarIlluminationError::BodyAtObserver { .. }
            | LunarIlluminationError::CoincidentSunAndMoon { .. }
            | LunarIlluminationError::NonFiniteTriangle { .. } => None,
        }
    }
}
