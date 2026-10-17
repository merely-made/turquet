// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Signed apparent geocentric ecliptic-longitude motion on the true equinox
//! of date, per TT day (86,400 SI seconds).
//!
//! [`longitude_motion`] samples the existing position provider at **±0.01 TT
//! day** (a total interval of 0.02 day, or 28.8 minutes). It divides the shortest
//! signed longitude difference by that interval. The provider must describe a
//! continuous trajectory moving less than 180 degrees over the interval;
//! endpoint samples alone cannot detect extra revolutions. This is a numerical
//! derivative in the moving ecliptic-of-date frame, not an inertial velocity.
//!
//! Rates with absolute value **≤0.001 degree/day** are [`MotionClass::NearStation`].
//! This is an operational small-rate band, not an exact station, a root search,
//! or an uncertainty interval. The signed rate is retained even inside the band.
//! Position accuracy does not establish derivative accuracy: no rate-error bound
//! is inferred from the provider's angular accuracy. Independent fixture tests
//! qualify only their retained cohort. Both sample epochs must be supported.
//!
//! The inherited [`is_retrograde`](crate::apparent::is_retrograde) retains its
//! one-day sampling interval and sign-only behavior. Close to a station its
//! result can differ from the shorter interval and three-way policy here.

use apparent::{ApparentBody, ApparentError};
use foundation::{JulianDate, Model, TerrestrialTime, ValueError};
use provider::{ANALYTICAL_EPHEMERIS, GeocentricPositionProvider};
use std::error::Error;
use std::fmt;

/// Half of the central-difference interval, in TT days.
pub const SAMPLE_HALF_INTERVAL_DAYS: f64 = 0.01;
/// Inclusive absolute-rate ceiling for the near-station band, in degrees/day.
pub const NEAR_STATION_DEGREES_PER_DAY: f64 = 0.001;

/// A finite signed angular rate; positive means increasing longitude.
/// Unlike a longitude, this value is never normalized to [0, 360).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct LongitudeRate(f64);

impl LongitudeRate {
    pub fn from_degrees_per_day(value: f64) -> Result<Self, ValueError> {
        if !value.is_finite() {
            return Err(ValueError::NotFinite("longitude rate in degrees/day"));
        }
        Ok(Self(value))
    }

    pub fn degrees_per_day(self) -> f64 {
        self.0
    }

    /// Apply the inclusive, fixed small-rate policy documented by this module.
    pub fn classification(self) -> MotionClass {
        if self.0.abs() <= NEAR_STATION_DEGREES_PER_DAY {
            MotionClass::NearStation
        } else if self.0 < 0.0 {
            MotionClass::Retrograde
        } else {
            MotionClass::Direct
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionClass {
    Direct,
    Retrograde,
    /// A sampled small-rate band; makes no claim of an exact stationary instant.
    NearStation,
}

/// A sampled motion fact with its body, TT epoch, and position-source identity.
/// The model and snapshot identify the input positions; they do not assert a
/// measured rate accuracy. Numerical controls are the module's public constants.
#[derive(Clone, Debug, PartialEq)]
pub struct LongitudeMotion {
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    rate: LongitudeRate,
    model: Model,
    data_snapshot: Option<String>,
}

impl LongitudeMotion {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn epoch(&self) -> JulianDate<TerrestrialTime> {
        self.epoch
    }

    pub fn rate(&self) -> LongitudeRate {
        self.rate
    }

    pub fn classification(&self) -> MotionClass {
        self.rate.classification()
    }

    pub fn model(&self) -> Model {
        self.model
    }

    pub fn data_snapshot(&self) -> Option<&str> {
        self.data_snapshot.as_deref()
    }

    /// Full central-difference interval in TT days; samples are at half this
    /// duration before and after [`Self::epoch`].
    pub fn sample_interval_days(&self) -> f64 {
        2.0 * SAMPLE_HALF_INTERVAL_DAYS
    }

    /// Inclusive absolute-rate threshold used by [`Self::classification`].
    pub fn near_station_threshold_degrees_per_day(&self) -> f64 {
        NEAR_STATION_DEGREES_PER_DAY
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MotionError<E> {
    /// A position-source failure, retained without flattening its details.
    Provider(E),
    /// The epoch cannot resolve the fixed sample offsets, or is not finite
    /// when its two Julian-date parts are summed.
    UnresolvableInterval,
    /// A provider returned a state for a different TT instant.
    SampleEpochMismatch {
        requested: JulianDate<TerrestrialTime>,
        returned: JulianDate<TerrestrialTime>,
    },
    /// Exactly half a revolution has no unique shortest signed difference.
    AmbiguousHalfTurn,
}

impl<E: fmt::Display> fmt::Display for MotionError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MotionError::Provider(error) => write!(f, "motion position sample: {}", error),
            MotionError::UnresolvableInterval => {
                f.write_str("TT epoch cannot resolve motion samples")
            },
            MotionError::SampleEpochMismatch {
                requested,
                returned,
            } => write!(
                f,
                "motion sample requested TT JD {}, provider returned {}",
                requested.day(),
                returned.day()
            ),
            MotionError::AmbiguousHalfTurn => {
                f.write_str("motion samples differ by exactly 180 degrees")
            },
        }
    }
}

impl<E: Error + 'static> Error for MotionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MotionError::Provider(error) => Some(error),
            _ => None,
        }
    }
}

/// Sample any geocentric apparent position provider, including trait objects.
/// Provider failures at either endpoint are returned with their original error.
pub fn longitude_motion<P: GeocentricPositionProvider + ?Sized>(
    provider: &P,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<LongitudeMotion, MotionError<P::Error>> {
    let before_epoch = epoch
        .offset_days(-SAMPLE_HALF_INTERVAL_DAYS)
        .map_err(|_| MotionError::UnresolvableInterval)?;
    let after_epoch = epoch
        .offset_days(SAMPLE_HALF_INTERVAL_DAYS)
        .map_err(|_| MotionError::UnresolvableInterval)?;
    if !epoch.day().is_finite()
        || !(before_epoch.day() < epoch.day() && epoch.day() < after_epoch.day())
    {
        return Err(MotionError::UnresolvableInterval);
    }
    let sample = |at| {
        let state = provider.position(body, at).map_err(MotionError::Provider)?;
        // Compare instants, accepting equivalent two-part representations.
        let (requested1, requested2) = at.parts();
        let (returned1, returned2) = state.epoch().parts();
        if (returned1 - requested1) + (returned2 - requested2) != 0.0 {
            return Err(MotionError::SampleEpochMismatch {
                requested: at,
                returned: state.epoch(),
            });
        }
        Ok(state.direction().longitude().degrees())
    };
    let before = sample(before_epoch)?;
    let after = sample(after_epoch)?;
    let delta = (after - before + 180.0).rem_euclid(360.0) - 180.0;
    if delta.abs() == 180.0 {
        return Err(MotionError::AmbiguousHalfTurn);
    }
    Ok(LongitudeMotion {
        body,
        epoch,
        rate: LongitudeRate(delta / (2.0 * SAMPLE_HALF_INTERVAL_DAYS)),
        model: provider.model(),
        data_snapshot: provider.data_snapshot().map(str::to_owned),
    })
}

/// Convenience entry point using Turquet's existing analytical ephemeris.
pub fn analytical_longitude_motion(
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<LongitudeMotion, MotionError<ApparentError>> {
    longitude_motion(&ANALYTICAL_EPHEMERIS, body, epoch)
}
