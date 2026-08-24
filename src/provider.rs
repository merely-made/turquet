// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Position-provider boundary shared by analytical and verification engines.

use apparent::{self, ApparentBody, ApparentError, ANALYTICAL_APPARENT};
use foundation::{JulianDate, Model, State, TerrestrialTime, TrueEclipticEquinoxOfDate};

/// A provider of geocentric apparent states on the true ecliptic of date.
///
/// Event algorithms depend on this contract rather than on Turquet's
/// analytical implementation. Provider errors retain range and data-source
/// failures instead of being flattened into an absent event.
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
