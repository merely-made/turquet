// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Named airless solar twilight crossings.
//!
//! This module only names caller-selected Sun center-altitude crossings. It
//! does not select civil, nautical, or astronomical thresholds, or add
//! refraction, limb, horizon-dip, terrain, or visibility policy.

use apparent::ApparentBody;
use foundation::{Model, Observer};
use provider::{EarthOrientationProvider, GeocentricPositionProvider};

use super::{
    airless_altitude_crossings, AirlessAltitudeCrossing, AltitudeCrossingError,
    AltitudeCrossingKind, AltitudeCrossingSearch,
};

/// Naming model for a caller-selected airless solar twilight crossing.
///
/// The nested crossing retains the selected physical center-altitude threshold
/// and all observer, position, transform, and Earth-orientation provenance.
pub const AIRLESS_SOLAR_TWILIGHT_NAMING: Model =
    Model::new("caller-threshold airless solar twilight naming", "1");

/// Direction through a caller-selected airless solar twilight threshold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolarTwilightKind {
    Dawn,
    Dusk,
}

/// One named projection of an airless solar altitude crossing.
///
/// `Dawn` is an ascending Sun-center crossing and `Dusk` is a descending one.
/// This is not a civil-time or human-visibility result. The nested crossing
/// retains the caller-selected threshold and complete numerical provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct SolarTwilightEvent {
    kind: SolarTwilightKind,
    crossing: AirlessAltitudeCrossing,
    naming_model: Model,
}

impl SolarTwilightEvent {
    pub fn kind(&self) -> SolarTwilightKind {
        self.kind
    }

    pub fn crossing(&self) -> &AirlessAltitudeCrossing {
        &self.crossing
    }

    pub fn naming_model(&self) -> Model {
        self.naming_model
    }
}

/// Error boundary shared by airless solar twilight events.
pub type SolarTwilightError<P, E> = AltitudeCrossingError<P, E>;

/// Name sampled airless Sun-center crossings as dawn or dusk.
///
/// `search.threshold()` remains caller policy. The search makes no selection
/// among conventional twilight bands and does not add refraction, solar limb,
/// horizon dip, terrain, obstruction, weather, civil-day, or visibility policy.
/// An empty result has the same limited sampled meaning as
/// [`airless_altitude_crossings`].
pub fn airless_solar_twilight_events<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    search: AltitudeCrossingSearch,
) -> Result<Vec<SolarTwilightEvent>, SolarTwilightError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    airless_altitude_crossings(
        positions,
        earth_orientation,
        observer,
        ApparentBody::Sun,
        search,
    )
    .map(|crossings| {
        crossings
            .into_iter()
            .map(|crossing| SolarTwilightEvent {
                kind: match crossing.kind() {
                    AltitudeCrossingKind::Ascending => SolarTwilightKind::Dawn,
                    AltitudeCrossingKind::Descending => SolarTwilightKind::Dusk,
                },
                crossing,
                naming_model: AIRLESS_SOLAR_TWILIGHT_NAMING,
            })
            .collect()
    })
}
