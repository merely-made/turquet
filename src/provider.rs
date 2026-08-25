// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Position and Earth-orientation provider boundaries for shared calculations.

use apparent::{self, ApparentBody, ApparentError, ANALYTICAL_APPARENT};
use std::convert::Infallible;

use foundation::{JulianDate, Model, State, TerrestrialTime, TrueEclipticEquinoxOfDate};
use observer::EarthOrientation;

/// A provider of geocentric apparent states on the true ecliptic of date.
///
/// Event algorithms depend on this contract rather than on Turquet's
/// analytical implementation. Provider errors retain range and data-source
/// failures instead of being flattened into an absent event. Within one
/// calculation, repeated requests for the same body and TT epoch must return
/// the same state or error while `model()` and `data_snapshot()` are unchanged.
pub trait GeocentricPositionProvider {
    type Error;

    /// Stable identity of the position model used by this provider.
    fn model(&self) -> Model;

    /// Runtime data snapshot used by the provider, when one exists.
    fn data_snapshot(&self) -> Option<&str> {
        None
    }

    /// Calculate one geocentric apparent state at a typed TT epoch.
    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error>;
}

/// A source of Earth-orientation facts for each TT sample in an observer
/// calculation or event search.
///
/// A provider instance has one immutable authority and data snapshot. Its
/// returned [`EarthOrientation`] must carry the same identity, while UT1 and
/// polar motion correspond to the requested TT epoch. Within one calculation,
/// repeated requests for the same TT epoch must return the same facts or error.
pub trait EarthOrientationProvider {
    type Error;

    fn authority(&self) -> &str;

    fn data_snapshot(&self) -> &str;

    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error>;
}

/// A disclosed approximation that holds UT1-minus-TT and polar motion fixed.
///
/// The absolute UT1 value advances at the same SI-day rate as each requested
/// TT epoch. This is suitable for bounded searches whose caller accepts the
/// named reference snapshot; it does not turn observed Earth orientation into
/// a timeless constant.
#[derive(Clone, Debug, PartialEq)]
pub struct ConstantOffsetEarthOrientation {
    reference_epoch: JulianDate<TerrestrialTime>,
    reference: EarthOrientation,
}

impl ConstantOffsetEarthOrientation {
    pub fn new(reference_epoch: JulianDate<TerrestrialTime>, reference: EarthOrientation) -> Self {
        Self {
            reference_epoch,
            reference,
        }
    }

    pub fn reference_epoch(&self) -> JulianDate<TerrestrialTime> {
        self.reference_epoch
    }

    pub fn reference(&self) -> &EarthOrientation {
        &self.reference
    }
}

impl EarthOrientationProvider for ConstantOffsetEarthOrientation {
    type Error = Infallible;

    fn authority(&self) -> &str {
        self.reference.authority()
    }

    fn data_snapshot(&self) -> &str {
        self.reference.snapshot()
    }

    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        let elapsed_days = epoch.day() - self.reference_epoch.day();
        let ut1 = self
            .reference
            .ut1()
            .offset_days(elapsed_days)
            .expect("finite typed epochs have a finite elapsed interval");
        Ok(EarthOrientation::new(
            ut1,
            self.reference.polar_motion_x(),
            self.reference.polar_motion_y(),
            self.reference.authority(),
            self.reference.snapshot(),
        ))
    }
}

/// The kernel-free Turquet analytical position provider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnalyticalEphemeris;

impl GeocentricPositionProvider for AnalyticalEphemeris {
    type Error = ApparentError;

    fn model(&self) -> Model {
        ANALYTICAL_APPARENT
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        apparent::position(body, epoch).map(|position| position.into_value())
    }
}

/// Shared value for callers that do not need provider state.
pub const ANALYTICAL_EPHEMERIS: AnalyticalEphemeris = AnalyticalEphemeris;

#[cfg(feature = "verify")]
impl GeocentricPositionProvider for ::verify::JplVerifier {
    type Error = ::verify::VerifyError;

    fn model(&self) -> Model {
        ::verify::JPL_SPK_VERIFIER
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some(self.kernel_snapshot())
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        self.geocent_apparent_state(&body, epoch)
    }
}
