// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use std::fmt;

use apparent::ApparentBody;
use foundation::{Angle, Distance, JulianDate, Model, Observer, TerrestrialTime};

use super::super::{AltitudeCrossingError, EventError, EventInterval, SearchWindow};

/// Maximum full TT span accepted around a new-moon phase root.
pub const MAX_LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS: f64 = 1.0;

/// Revisioned model for local, airless, spherical fixed-limb circumstances.
///
/// The solver uses the IAU 2015 nominal solar radius and Turquet's mean lunar
/// radius. It deliberately excludes lunar limb relief, refraction, terrain,
/// obstruction, weather, and an optical or human visibility convention.
pub const LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCES: Model = Model::new(
    "topocentric airless spherical fixed-limb solar eclipse circumstances",
    "1",
);

/// Numerical controls for a local solar-eclipse circumstance search.
///
/// The full `circumstance_span_days` is centered on each new-moon phase root.
/// Its endpoints must lie outside a local partial eclipse, otherwise the
/// result reports a bounded-span error instead of silently omitting a contact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalSolarEclipseSearch {
    phase_window: SearchWindow,
    circumstance_span_days: f64,
}

impl LocalSolarEclipseSearch {
    pub fn new(
        phase_window: SearchWindow,
        circumstance_span_days: f64,
    ) -> Result<Self, LocalSolarEclipseSearchError> {
        if !circumstance_span_days.is_finite() {
            return Err(LocalSolarEclipseSearchError::SpanNotFinite);
        }
        if circumstance_span_days <= 0.0 {
            return Err(LocalSolarEclipseSearchError::SpanNotPositive);
        }
        if circumstance_span_days > MAX_LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS {
            return Err(LocalSolarEclipseSearchError::SpanTooLarge);
        }
        if circumstance_span_days <= 2.0 * phase_window.tolerance_days() {
            return Err(LocalSolarEclipseSearchError::ToleranceExceedsHalfSpan);
        }
        Ok(Self {
            phase_window,
            circumstance_span_days,
        })
    }

    pub fn phase_window(self) -> SearchWindow {
        self.phase_window
    }

    pub fn circumstance_span_days(self) -> f64 {
        self.circumstance_span_days
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalSolarEclipseSearchError {
    SpanNotFinite,
    SpanNotPositive,
    SpanTooLarge,
    ToleranceExceedsHalfSpan,
}

impl fmt::Display for LocalSolarEclipseSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            LocalSolarEclipseSearchError::SpanNotFinite => {
                "local solar eclipse circumstance span must be finite"
            }
            LocalSolarEclipseSearchError::SpanNotPositive => {
                "local solar eclipse circumstance span must be positive"
            }
            LocalSolarEclipseSearchError::SpanTooLarge => {
                "local solar eclipse circumstance span exceeds one TT day"
            }
            LocalSolarEclipseSearchError::ToleranceExceedsHalfSpan => {
                "event tolerance must be smaller than half the circumstance span"
            }
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for LocalSolarEclipseSearchError {}

/// Local class at greatest eclipse from the observer's topocentric disk geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalSolarEclipseKind {
    Partial,
    Annular,
    Total,
}

/// The ordered local contact convention.
///
/// `First` and `Fourth` are the external partial contacts C1 and C4. `Second`
/// and `Third` are the internal central contacts C2 and C3, present only for
/// annular and total local classes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalSolarEclipseContactKind {
    First,
    Second,
    Third,
    Fourth,
}

/// One bounded local solar-eclipse contact interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalSolarEclipseContact {
    pub(super) kind: LocalSolarEclipseContactKind,
    pub(super) interval: EventInterval,
}

impl LocalSolarEclipseContact {
    pub fn kind(self) -> LocalSolarEclipseContactKind {
        self.kind
    }

    pub fn interval(self) -> EventInterval {
        self.interval
    }
}

/// Topocentric disk and horizon facts evaluated at greatest local eclipse.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalSolarEclipseGeometry {
    pub(super) center_separation: Angle,
    pub(super) sun_angular_radius: Angle,
    pub(super) moon_angular_radius: Angle,
    pub(super) sun_center_altitude: Angle,
    pub(super) sun_upper_limb_altitude: Angle,
}

impl LocalSolarEclipseGeometry {
    pub fn center_separation(self) -> Angle {
        self.center_separation
    }

    pub fn sun_angular_radius(self) -> Angle {
        self.sun_angular_radius
    }

    pub fn moon_angular_radius(self) -> Angle {
        self.moon_angular_radius
    }

    pub fn sun_center_altitude(self) -> Angle {
        self.sun_center_altitude
    }

    /// Airless physical upper solar-limb altitude at greatest eclipse.
    pub fn sun_upper_limb_altitude(self) -> Angle {
        self.sun_upper_limb_altitude
    }
}

/// Geometric horizon state at greatest local eclipse.
///
/// This is deliberately not a general visibility window. It includes the
/// physical solar upper limb, but excludes refraction, terrain, obstruction,
/// weather, eye safety, and every social or civil convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalSolarEclipseVisibility {
    SunUpperLimbAboveAirlessHorizonAtGreatest,
    SunUpperLimbAtOrBelowAirlessHorizonAtGreatest,
}

/// Local solar-eclipse contacts, class, and provenance for one observer.
#[derive(Clone, Debug, PartialEq)]
pub struct LocalSolarEclipseCircumstances {
    pub(super) kind: LocalSolarEclipseKind,
    pub(super) phase_interval: EventInterval,
    pub(super) greatest_interval: EventInterval,
    pub(super) greatest_geometry: LocalSolarEclipseGeometry,
    pub(super) contacts: Vec<LocalSolarEclipseContact>,
    pub(super) search: LocalSolarEclipseSearch,
    pub(super) visibility: LocalSolarEclipseVisibility,
    pub(super) observer: Observer,
    pub(super) geometry_model: Model,
    pub(super) provider_model: Model,
    pub(super) provider_snapshot: Option<String>,
    pub(super) transform_model: Model,
    pub(super) earth_orientation_authority: String,
    pub(super) earth_orientation_snapshot: String,
}

impl LocalSolarEclipseCircumstances {
    pub fn kind(&self) -> LocalSolarEclipseKind {
        self.kind
    }

    pub fn phase_interval(&self) -> EventInterval {
        self.phase_interval
    }

    pub fn greatest_interval(&self) -> EventInterval {
        self.greatest_interval
    }

    pub fn greatest_geometry(&self) -> LocalSolarEclipseGeometry {
        self.greatest_geometry
    }

    pub fn contacts(&self) -> &[LocalSolarEclipseContact] {
        &self.contacts
    }

    pub fn search(&self) -> LocalSolarEclipseSearch {
        self.search
    }

    pub fn visibility(&self) -> LocalSolarEclipseVisibility {
        self.visibility
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn geometry_model(&self) -> Model {
        self.geometry_model
    }

    pub fn provider_model(&self) -> Model {
        self.provider_model
    }

    pub fn provider_snapshot(&self) -> Option<&str> {
        self.provider_snapshot.as_ref().map(String::as_str)
    }

    pub fn transform_model(&self) -> Model {
        self.transform_model
    }

    pub fn earth_orientation_authority(&self) -> &str {
        &self.earth_orientation_authority
    }

    pub fn earth_orientation_snapshot(&self) -> &str {
        &self.earth_orientation_snapshot
    }
}

/// A failure while composing a local solar-eclipse circumstance.
#[derive(Debug)]
pub enum LocalSolarEclipseError<P, E> {
    Phase(EventError<P>),
    Observation(AltitudeCrossingError<P, E>),
    CircumstanceSpanTooShort {
        phase_epoch: JulianDate<TerrestrialTime>,
        span_days: f64,
    },
    BodyContainsObserver {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        physical_radius: Distance,
        topocentric_distance: Distance,
    },
}

impl<P: fmt::Display, E: fmt::Display> fmt::Display for LocalSolarEclipseError<P, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LocalSolarEclipseError::Phase(ref source) => source.fmt(formatter),
            LocalSolarEclipseError::Observation(ref source) => source.fmt(formatter),
            LocalSolarEclipseError::CircumstanceSpanTooShort {
                phase_epoch,
                span_days,
            } => write!(
                formatter,
                "local solar eclipse span {} TT days around phase JD {} does not reach both exterior disk states",
                span_days,
                phase_epoch.day()
            ),
            LocalSolarEclipseError::BodyContainsObserver {
                body,
                epoch,
                physical_radius,
                topocentric_distance,
            } => write!(
                formatter,
                "{} physical radius {} km is not smaller than topocentric distance {} km at TT JD {}",
                body.name(),
                physical_radius.kilometers(),
                topocentric_distance.kilometers(),
                epoch.day()
            ),
        }
    }
}

impl<P, E> ::std::error::Error for LocalSolarEclipseError<P, E>
where
    P: ::std::error::Error + 'static,
    E: ::std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            LocalSolarEclipseError::Phase(ref source) => Some(source),
            LocalSolarEclipseError::Observation(ref source) => Some(source),
            LocalSolarEclipseError::CircumstanceSpanTooShort { .. }
            | LocalSolarEclipseError::BodyContainsObserver { .. } => None,
        }
    }
}
