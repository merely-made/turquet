// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Provider-neutral celestial event searches.
//!
//! T4 event searches currently cover apparent ecliptic-longitude conjunctions,
//! stationary points, lunar quarter phases, eclipse candidates, lunar eclipse
//! circumstances, and airless observer-altitude crossings. Every result is a
//! bounded TT interval, not an isolated floating-point instant.

use std::f64::consts::PI;
use std::fmt;

use apparent::ApparentBody;
use foundation::{
    Angle, Distance, JulianDate, Longitude, Model, Observer, State, TerrestrialTime,
    TrueEclipticEquinoxOfDate,
};
use observer::{ObserverTransform, ObserverTransformError, AIRLESS_TOPOCENTRIC_TRANSFORM};
use provider::{EarthOrientationProvider, GeocentricPositionProvider};

const TWO_PI: f64 = 2.0 * PI;

/// Maximum sampling step accepted by the current event searches.
///
/// One TT day keeps even the Moon's relative motion safely below a quarter
/// turn, so an opposition wrap cannot masquerade as a conjunction and one
/// phase step cannot hide a second quarter angle. Station callers can select a
/// smaller step when resolving faster changes in longitude speed.
pub const MAX_EVENT_STEP_DAYS: f64 = 1.0;

/// Maximum sampling step accepted by an altitude-crossing search.
///
/// The one-hour ceiling limits how much diurnal motion can occur between
/// samples. It does not prove the absence of tangent or multiple roots.
pub const MAX_ALTITUDE_CROSSING_STEP_DAYS: f64 = 1.0 / 24.0;

/// Backward-compatible name for the shared event sampling ceiling.
pub const MAX_CONJUNCTION_STEP_DAYS: f64 = MAX_EVENT_STEP_DAYS;

/// Maximum full interval accepted for a station's central-difference speed.
pub const MAX_STATION_VELOCITY_SPAN_DAYS: f64 = 1.0;

/// Maximum full interval refined around a lunar eclipse's phase root.
pub const MAX_LUNAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS: f64 = 1.0;

/// IAU 2015 nominal solar radius used by the eclipse geometry model.
pub const NOMINAL_SOLAR_RADIUS_KM: f64 = 695_700.0;

/// WGS84 equatorial Earth radius used by the eclipse geometry model.
pub const WGS84_EARTH_EQUATORIAL_RADIUS_KM: f64 = 6_378.137;

/// Mean lunar radius used by the eclipse geometry model.
pub const MEAN_LUNAR_RADIUS_KM: f64 = 1_737.4;

/// Atmosphere-free spherical eclipse candidate geometry.
///
/// Revision 1 uses the radii exposed above. It deliberately omits atmospheric
/// enlargement of the terrestrial shadow, Earth oblateness, local contacts,
/// and terrain.
pub const SPHERICAL_ECLIPSE_GEOMETRY: Model =
    Model::new("atmosphere-free spherical eclipse candidate geometry", "1");

/// Validated numerical controls for an event search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SearchWindow {
    start: JulianDate<TerrestrialTime>,
    end: JulianDate<TerrestrialTime>,
    step_days: f64,
    tolerance_days: f64,
}

impl SearchWindow {
    pub fn new(
        start: JulianDate<TerrestrialTime>,
        end: JulianDate<TerrestrialTime>,
        step_days: f64,
        tolerance_days: f64,
    ) -> Result<Self, SearchWindowError> {
        if end.day() <= start.day() {
            return Err(SearchWindowError::NonIncreasingInterval);
        }
        if !step_days.is_finite() || !tolerance_days.is_finite() {
            return Err(SearchWindowError::NotFinite);
        }
        if step_days <= 0.0 || tolerance_days <= 0.0 {
            return Err(SearchWindowError::NotPositive);
        }
        if step_days > MAX_EVENT_STEP_DAYS {
            return Err(SearchWindowError::StepTooLarge);
        }
        if tolerance_days > step_days {
            return Err(SearchWindowError::ToleranceExceedsStep);
        }
        Ok(Self {
            start,
            end,
            step_days,
            tolerance_days,
        })
    }

    pub fn start(self) -> JulianDate<TerrestrialTime> {
        self.start
    }

    pub fn end(self) -> JulianDate<TerrestrialTime> {
        self.end
    }

    pub fn step_days(self) -> f64 {
        self.step_days
    }

    pub fn tolerance_days(self) -> f64 {
        self.tolerance_days
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchWindowError {
    NonIncreasingInterval,
    NotFinite,
    NotPositive,
    StepTooLarge,
    ToleranceExceedsStep,
}

impl fmt::Display for SearchWindowError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            SearchWindowError::NonIncreasingInterval => "search end must follow its start",
            SearchWindowError::NotFinite => "search controls must be finite",
            SearchWindowError::NotPositive => "search step and tolerance must be positive",
            SearchWindowError::StepTooLarge => "event search step exceeds one TT day",
            SearchWindowError::ToleranceExceedsStep => {
                "search tolerance cannot exceed its sampling step"
            }
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for SearchWindowError {}

/// Validated numerical controls for an airless altitude-crossing search.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AltitudeCrossingSearch {
    window: SearchWindow,
    threshold: Angle,
}

impl AltitudeCrossingSearch {
    pub fn new(
        window: SearchWindow,
        threshold: Angle,
    ) -> Result<Self, AltitudeCrossingSearchError> {
        if window.step_days() > MAX_ALTITUDE_CROSSING_STEP_DAYS {
            return Err(AltitudeCrossingSearchError::StepTooLarge);
        }
        if threshold.radians() < -PI / 2.0 || threshold.radians() > PI / 2.0 {
            return Err(AltitudeCrossingSearchError::ThresholdOutOfRange);
        }
        Ok(Self { window, threshold })
    }

    pub fn window(self) -> SearchWindow {
        self.window
    }

    pub fn threshold(self) -> Angle {
        self.threshold
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeCrossingSearchError {
    StepTooLarge,
    ThresholdOutOfRange,
}

impl fmt::Display for AltitudeCrossingSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AltitudeCrossingSearchError::StepTooLarge => {
                formatter.write_str("altitude-crossing search step exceeds one TT hour")
            }
            AltitudeCrossingSearchError::ThresholdOutOfRange => {
                formatter.write_str("altitude threshold must be between minus and plus 90 degrees")
            }
        }
    }
}

impl ::std::error::Error for AltitudeCrossingSearchError {}

/// Numerical controls specific to a stationary-point search.
///
/// `velocity_span_days` is the full width of the central difference used to
/// classify apparent ecliptic-longitude motion. It is retained in each event
/// because changing the span can move the reported root slightly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StationSearch {
    window: SearchWindow,
    velocity_span_days: f64,
}

impl StationSearch {
    pub fn new(window: SearchWindow, velocity_span_days: f64) -> Result<Self, StationSearchError> {
        if !velocity_span_days.is_finite() {
            return Err(StationSearchError::VelocitySpanNotFinite);
        }
        if velocity_span_days <= 0.0 {
            return Err(StationSearchError::VelocitySpanNotPositive);
        }
        if velocity_span_days > MAX_STATION_VELOCITY_SPAN_DAYS {
            return Err(StationSearchError::VelocitySpanTooLarge);
        }
        Ok(Self {
            window,
            velocity_span_days,
        })
    }

    pub fn window(self) -> SearchWindow {
        self.window
    }

    pub fn velocity_span_days(self) -> f64 {
        self.velocity_span_days
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationSearchError {
    VelocitySpanNotFinite,
    VelocitySpanNotPositive,
    VelocitySpanTooLarge,
}

impl fmt::Display for StationSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            StationSearchError::VelocitySpanNotFinite => "station velocity span must be finite",
            StationSearchError::VelocitySpanNotPositive => "station velocity span must be positive",
            StationSearchError::VelocitySpanTooLarge => "station velocity span exceeds one TT day",
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for StationSearchError {}

/// Numerical controls for lunar greatest-event and contact solving.
///
/// The phase window locates full-moon roots. `circumstance_span_days` is the
/// full TT interval, centered on each phase root, within which Turquet refines
/// greatest eclipse and brackets every contact. Provider requests therefore
/// extend half this span beyond the phase midpoint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarEclipseSearch {
    phase_window: SearchWindow,
    circumstance_span_days: f64,
}

impl LunarEclipseSearch {
    pub fn new(
        phase_window: SearchWindow,
        circumstance_span_days: f64,
    ) -> Result<Self, LunarEclipseSearchError> {
        if !circumstance_span_days.is_finite() {
            return Err(LunarEclipseSearchError::SpanNotFinite);
        }
        if circumstance_span_days <= 0.0 {
            return Err(LunarEclipseSearchError::SpanNotPositive);
        }
        if circumstance_span_days > MAX_LUNAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS {
            return Err(LunarEclipseSearchError::SpanTooLarge);
        }
        if circumstance_span_days <= 2.0 * phase_window.tolerance_days {
            return Err(LunarEclipseSearchError::ToleranceExceedsHalfSpan);
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
pub enum LunarEclipseSearchError {
    SpanNotFinite,
    SpanNotPositive,
    SpanTooLarge,
    ToleranceExceedsHalfSpan,
}

impl fmt::Display for LunarEclipseSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            LunarEclipseSearchError::SpanNotFinite => {
                "lunar eclipse circumstance span must be finite"
            }
            LunarEclipseSearchError::SpanNotPositive => {
                "lunar eclipse circumstance span must be positive"
            }
            LunarEclipseSearchError::SpanTooLarge => {
                "lunar eclipse circumstance span exceeds one TT day"
            }
            LunarEclipseSearchError::ToleranceExceedsHalfSpan => {
                "event tolerance must be smaller than half the circumstance span"
            }
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for LunarEclipseSearchError {}

/// The TT interval known to contain an event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventInterval {
    start: JulianDate<TerrestrialTime>,
    end: JulianDate<TerrestrialTime>,
}

impl EventInterval {
    pub fn start(self) -> JulianDate<TerrestrialTime> {
        self.start
    }

    pub fn end(self) -> JulianDate<TerrestrialTime> {
        self.end
    }

    pub fn width_days(self) -> f64 {
        self.end.day() - self.start.day()
    }

    pub fn midpoint(self) -> JulianDate<TerrestrialTime> {
        JulianDate::from_julian_day((self.start.day() + self.end.day()) / 2.0)
            .expect("an interval between finite epochs has a finite midpoint")
    }
}

/// Direction through a requested airless altitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeCrossingKind {
    Ascending,
    Descending,
}

/// One sampled, sign-changing crossing of an airless topocentric altitude.
///
/// The interval is bounded in TT. An empty search result means only that no
/// sampled sign crossing was found; it is not an always-above, always-below,
/// circumpolar, or grazing classification.
#[derive(Clone, Debug, PartialEq)]
pub struct AirlessAltitudeCrossing {
    body: ApparentBody,
    kind: AltitudeCrossingKind,
    interval: EventInterval,
    observer: Observer,
    threshold: Angle,
    provider_model: Model,
    provider_snapshot: Option<String>,
    transform_model: Model,
    earth_orientation_authority: String,
    earth_orientation_snapshot: String,
}

impl AirlessAltitudeCrossing {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn kind(&self) -> AltitudeCrossingKind {
        self.kind
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn threshold(&self) -> Angle {
        self.threshold
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

/// An apparent conjunction in ecliptic longitude.
#[derive(Clone, Debug, PartialEq)]
pub struct EclipticLongitudeConjunction {
    first: ApparentBody,
    second: ApparentBody,
    interval: EventInterval,
    angular_separation: Angle,
    provider_model: Model,
    provider_snapshot: Option<String>,
}

impl EclipticLongitudeConjunction {
    pub fn first(&self) -> ApparentBody {
        self.first
    }

    pub fn second(&self) -> ApparentBody {
        self.second
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    /// Great-circle center separation at the interval midpoint.
    pub fn angular_separation(&self) -> Angle {
        self.angular_separation
    }

    pub fn provider_model(&self) -> Model {
        self.provider_model
    }

    /// Runtime data revision retained from the provider, such as a kernel
    /// digest. Kernel-free analytical results return `None`.
    pub fn provider_snapshot(&self) -> Option<&str> {
        self.provider_snapshot.as_ref().map(String::as_str)
    }
}

/// The apparent ecliptic-longitude direction on one side of a station.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LongitudeMotion {
    Direct,
    Retrograde,
}

/// A reversal of apparent ecliptic-longitude motion.
#[derive(Clone, Debug, PartialEq)]
pub struct EclipticLongitudeStation {
    body: ApparentBody,
    interval: EventInterval,
    longitude: Longitude,
    motion_before: LongitudeMotion,
    motion_after: LongitudeMotion,
    velocity_span_days: f64,
    provider_model: Model,
    provider_snapshot: Option<String>,
}

impl EclipticLongitudeStation {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    /// Apparent ecliptic longitude at the interval midpoint.
    pub fn longitude(&self) -> Longitude {
        self.longitude
    }

    pub fn motion_before(&self) -> LongitudeMotion {
        self.motion_before
    }

    pub fn motion_after(&self) -> LongitudeMotion {
        self.motion_after
    }

    pub fn velocity_span_days(&self) -> f64 {
        self.velocity_span_days
    }

    pub fn provider_model(&self) -> Model {
        self.provider_model
    }

    pub fn provider_snapshot(&self) -> Option<&str> {
        self.provider_snapshot.as_ref().map(String::as_str)
    }
}

/// One of the four apparent Moon-Sun ecliptic-longitude quarter angles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunarPhase {
    NewMoon,
    FirstQuarter,
    FullMoon,
    LastQuarter,
}

impl LunarPhase {
    pub fn name(self) -> &'static str {
        match self {
            LunarPhase::NewMoon => "new moon",
            LunarPhase::FirstQuarter => "first quarter",
            LunarPhase::FullMoon => "full moon",
            LunarPhase::LastQuarter => "last quarter",
        }
    }

    /// Target apparent ecliptic longitude of the Moon east of the Sun.
    pub fn target_elongation(self) -> Angle {
        let degrees = match self {
            LunarPhase::NewMoon => 0.0,
            LunarPhase::FirstQuarter => 90.0,
            LunarPhase::FullMoon => 180.0,
            LunarPhase::LastQuarter => 270.0,
        };
        Angle::from_degrees(degrees).expect("quarter angles are finite")
    }
}

/// A lunar phase defined by apparent ecliptic longitude.
#[derive(Clone, Debug, PartialEq)]
pub struct LunarPhaseEvent {
    phase: LunarPhase,
    interval: EventInterval,
    angular_separation: Angle,
    provider_model: Model,
    provider_snapshot: Option<String>,
}

impl LunarPhaseEvent {
    pub fn phase(&self) -> LunarPhase {
        self.phase
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    /// Great-circle Sun-Moon center separation at the interval midpoint.
    pub fn angular_separation(&self) -> Angle {
        self.angular_separation
    }

    pub fn provider_model(&self) -> Model {
        self.provider_model
    }

    pub fn provider_snapshot(&self) -> Option<&str> {
        self.provider_snapshot.as_ref().map(String::as_str)
    }
}

/// The geometric class of an eclipse candidate.
///
/// `Solar` is intentionally not split into partial, annular, or total: that
/// classification requires a topocentric observer or a solved path across the
/// Earth. Lunar classes are geocentric intersections with the spherical Earth
/// shadow at the lunar phase midpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EclipseCandidateKind {
    Solar,
    PenumbralLunar,
    PartialLunar,
    TotalLunar,
}

/// Angular facts used to accept and classify an eclipse candidate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EclipseCandidateGeometry {
    Solar {
        /// Geocentric great-circle separation of the Sun and Moon centers.
        center_separation: Angle,
        sun_angular_radius: Angle,
        moon_angular_radius: Angle,
        /// Conservative sum of solar and lunar horizontal parallax.
        observer_parallax_allowance: Angle,
    },
    Lunar {
        /// Great-circle separation of the Moon center from the antisolar axis.
        shadow_axis_separation: Angle,
        moon_angular_radius: Angle,
        umbra_angular_radius: Angle,
        penumbra_angular_radius: Angle,
    },
}

/// A new- or full-moon event whose spherical geometry permits an eclipse.
#[derive(Clone, Debug, PartialEq)]
pub struct EclipseCandidate {
    kind: EclipseCandidateKind,
    interval: EventInterval,
    geometry: EclipseCandidateGeometry,
    geometry_model: Model,
    provider_model: Model,
    provider_snapshot: Option<String>,
}

impl EclipseCandidate {
    pub fn kind(&self) -> EclipseCandidateKind {
        self.kind
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    pub fn geometry(&self) -> EclipseCandidateGeometry {
        self.geometry
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
}

/// Atmosphere-free spherical shadow geometry at one lunar eclipse epoch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarEclipseGeometry {
    shadow_axis_separation: Angle,
    shadow_axis_offset: Distance,
    moon_angular_radius: Angle,
    umbra_angular_radius: Angle,
    umbra_radius: Distance,
    penumbra_angular_radius: Angle,
    penumbra_radius: Distance,
}

impl LunarEclipseGeometry {
    pub fn shadow_axis_separation(self) -> Angle {
        self.shadow_axis_separation
    }

    pub fn shadow_axis_offset(self) -> Distance {
        self.shadow_axis_offset
    }

    pub fn moon_angular_radius(self) -> Angle {
        self.moon_angular_radius
    }

    pub fn umbra_angular_radius(self) -> Angle {
        self.umbra_angular_radius
    }

    pub fn umbra_radius(self) -> Distance {
        self.umbra_radius
    }

    pub fn penumbra_angular_radius(self) -> Angle {
        self.penumbra_angular_radius
    }

    pub fn penumbra_radius(self) -> Distance {
        self.penumbra_radius
    }
}

/// Geocentric lunar eclipse class at greatest eclipse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunarEclipseKind {
    Penumbral,
    Partial,
    Total,
}

/// One conventional contact between the lunar disk and Earth's shadow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LunarEclipseContactKind {
    PenumbralIngress,
    UmbralIngress,
    TotalityBegins,
    TotalityEnds,
    UmbralEgress,
    PenumbralEgress,
}

impl LunarEclipseContactKind {
    pub fn abbreviation(self) -> &'static str {
        match self {
            LunarEclipseContactKind::PenumbralIngress => "P1",
            LunarEclipseContactKind::UmbralIngress => "U1",
            LunarEclipseContactKind::TotalityBegins => "U2",
            LunarEclipseContactKind::TotalityEnds => "U3",
            LunarEclipseContactKind::UmbralEgress => "U4",
            LunarEclipseContactKind::PenumbralEgress => "P4",
        }
    }
}

/// A bounded TT interval containing one lunar eclipse contact.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LunarEclipseContact {
    kind: LunarEclipseContactKind,
    interval: EventInterval,
}

impl LunarEclipseContact {
    pub fn kind(self) -> LunarEclipseContactKind {
        self.kind
    }

    pub fn interval(self) -> EventInterval {
        self.interval
    }
}

/// Greatest geometry and ordered contacts for one geocentric lunar eclipse.
#[derive(Clone, Debug, PartialEq)]
pub struct LunarEclipseCircumstances {
    kind: LunarEclipseKind,
    phase_interval: EventInterval,
    greatest_interval: EventInterval,
    greatest_geometry: LunarEclipseGeometry,
    contacts: Vec<LunarEclipseContact>,
    circumstance_span_days: f64,
    geometry_model: Model,
    provider_model: Model,
    provider_snapshot: Option<String>,
}

#[derive(Clone, Copy)]
struct LunarShadowGeometry {
    public: LunarEclipseGeometry,
    shadow_axis_offset_km: f64,
    umbra_radius_km: f64,
    penumbra_radius_km: f64,
}

#[derive(Clone, Copy)]
enum LunarContactBoundary {
    Penumbral,
    Umbral,
    Total,
}

impl LunarEclipseCircumstances {
    pub fn kind(&self) -> LunarEclipseKind {
        self.kind
    }

    pub fn phase_interval(&self) -> EventInterval {
        self.phase_interval
    }

    pub fn greatest_interval(&self) -> EventInterval {
        self.greatest_interval
    }

    pub fn greatest_geometry(&self) -> LunarEclipseGeometry {
        self.greatest_geometry
    }

    pub fn contacts(&self) -> &[LunarEclipseContact] {
        &self.contacts
    }

    pub fn circumstance_span_days(&self) -> f64 {
        self.circumstance_span_days
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
}

#[derive(Debug)]
pub enum EventError<E> {
    SameBody,
    DistanceTooSmall {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        distance_km: f64,
        required_greater_than_km: f64,
    },
    CircumstanceSpanTooShort {
        phase_epoch: JulianDate<TerrestrialTime>,
        span_days: f64,
    },
    Position {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        source: E,
    },
}

impl<E: fmt::Display> fmt::Display for EventError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EventError::SameBody => write!(formatter, "a conjunction requires two bodies"),
            EventError::DistanceTooSmall {
                body,
                epoch,
                distance_km,
                required_greater_than_km,
            } => write!(
                formatter,
                "{} distance {} km at TT JD {} must exceed {} km for eclipse geometry",
                body.name(),
                distance_km,
                epoch.day(),
                required_greater_than_km
            ),
            EventError::CircumstanceSpanTooShort {
                phase_epoch,
                span_days,
            } => write!(
                formatter,
                "lunar eclipse span {} TT days around phase JD {} does not reach both penumbral exterior states",
                span_days,
                phase_epoch.day()
            ),
            EventError::Position {
                body,
                epoch,
                ref source,
            } => write!(
                formatter,
                "could not position {} at TT JD {}: {}",
                body.name(),
                epoch.day(),
                source
            ),
        }
    }
}

impl<E: ::std::error::Error + 'static> ::std::error::Error for EventError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            EventError::Position { ref source, .. } => Some(source),
            EventError::SameBody
            | EventError::DistanceTooSmall { .. }
            | EventError::CircumstanceSpanTooShort { .. } => None,
        }
    }
}

/// A failure while composing position, Earth-orientation, and observer facts.
#[derive(Debug)]
pub enum AltitudeCrossingError<P, E> {
    Position {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        source: P,
    },
    EarthOrientation {
        epoch: JulianDate<TerrestrialTime>,
        source: E,
    },
    EarthOrientationIdentityMismatch {
        epoch: JulianDate<TerrestrialTime>,
        expected_authority: String,
        expected_snapshot: String,
        actual_authority: String,
        actual_snapshot: String,
    },
    Transform {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        source: ObserverTransformError,
    },
}

impl<P: fmt::Display, E: fmt::Display> fmt::Display for AltitudeCrossingError<P, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AltitudeCrossingError::Position {
                body,
                epoch,
                ref source,
            } => write!(
                formatter,
                "could not position {} at TT JD {}: {}",
                body.name(),
                epoch.day(),
                source
            ),
            AltitudeCrossingError::EarthOrientation { epoch, ref source } => write!(
                formatter,
                "could not obtain Earth orientation at TT JD {}: {}",
                epoch.day(),
                source
            ),
            AltitudeCrossingError::EarthOrientationIdentityMismatch {
                epoch,
                ref expected_authority,
                ref expected_snapshot,
                ref actual_authority,
                ref actual_snapshot,
            } => write!(
                formatter,
                "Earth-orientation identity changed at TT JD {} from {}/{} to {}/{}",
                epoch.day(),
                expected_authority,
                expected_snapshot,
                actual_authority,
                actual_snapshot
            ),
            AltitudeCrossingError::Transform {
                body,
                epoch,
                ref source,
            } => write!(
                formatter,
                "could not transform {} for an observer at TT JD {}: {}",
                body.name(),
                epoch.day(),
                source
            ),
        }
    }
}

impl<P, E> ::std::error::Error for AltitudeCrossingError<P, E>
where
    P: ::std::error::Error + 'static,
    E: ::std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            AltitudeCrossingError::Position { ref source, .. } => Some(source),
            AltitudeCrossingError::EarthOrientation { ref source, .. } => Some(source),
            AltitudeCrossingError::Transform { ref source, .. } => Some(source),
            AltitudeCrossingError::EarthOrientationIdentityMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Copy)]
struct AltitudeSample {
    epoch: JulianDate<TerrestrialTime>,
    signed_altitude: f64,
}

/// Find sampled crossings of a selected airless topocentric altitude.
///
/// The position and Earth-orientation sources are queried at every TT sample.
/// Strict sign changes are refined by bisection to the configured tolerance.
/// An isolated exact-zero sample is retained only when its neighboring signs
/// show a crossing; an interior same-sign touch is treated as grazing and is
/// omitted. Search-window boundary zeros use their available one-sided sign.
pub fn airless_altitude_crossings<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: AltitudeCrossingSearch,
) -> Result<Vec<AirlessAltitudeCrossing>, AltitudeCrossingError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let window = search.window();
    let threshold = search.threshold();
    let orientation_authority = earth_orientation.authority().to_owned();
    let orientation_snapshot = earth_orientation.data_snapshot().to_owned();
    let mut samples = Vec::new();
    let mut epoch = window.start();

    loop {
        samples.push(AltitudeSample {
            epoch,
            signed_altitude: observer_signed_altitude(
                positions,
                earth_orientation,
                observer,
                body,
                epoch,
                threshold,
                &orientation_authority,
                &orientation_snapshot,
            )?,
        });
        if epoch.day() >= window.end().day() {
            break;
        }
        let next_day = (epoch.day() + window.step_days()).min(window.end().day());
        epoch = JulianDate::from_julian_day(next_day)
            .expect("a bounded step between finite epochs remains finite");
    }

    let mut events = Vec::new();
    let mut index = 0;
    while index < samples.len() {
        let left = samples[index];
        if left.signed_altitude == 0.0 {
            let zero_start = index;
            let mut zero_end = index;
            while zero_end + 1 < samples.len() && samples[zero_end + 1].signed_altitude == 0.0 {
                zero_end += 1;
            }
            let before = if zero_start > 0 {
                Some(samples[zero_start - 1].signed_altitude)
            } else {
                None
            };
            let after = if zero_end + 1 < samples.len() {
                Some(samples[zero_end + 1].signed_altitude)
            } else {
                None
            };
            let kind = if zero_start != zero_end {
                None
            } else {
                exact_crossing_kind(before, after)
            };
            if let Some(kind) = kind {
                push_altitude_crossing(
                    &mut events,
                    body,
                    kind,
                    EventInterval {
                        start: left.epoch,
                        end: left.epoch,
                    },
                    observer,
                    threshold,
                    positions,
                    &orientation_authority,
                    &orientation_snapshot,
                );
            }
            index = zero_end + 1;
            continue;
        }

        if index + 1 < samples.len() {
            let right = samples[index + 1];
            if right.signed_altitude != 0.0
                && left.signed_altitude.signum() != right.signed_altitude.signum()
            {
                let kind = if right.signed_altitude > 0.0 {
                    AltitudeCrossingKind::Ascending
                } else {
                    AltitudeCrossingKind::Descending
                };
                let interval = refine_altitude_crossing(
                    positions,
                    earth_orientation,
                    observer,
                    body,
                    threshold,
                    left,
                    right,
                    window.tolerance_days(),
                    &orientation_authority,
                    &orientation_snapshot,
                )?;
                push_altitude_crossing(
                    &mut events,
                    body,
                    kind,
                    interval,
                    observer,
                    threshold,
                    positions,
                    &orientation_authority,
                    &orientation_snapshot,
                );
            }
        }
        index += 1;
    }

    Ok(events)
}

fn observer_signed_altitude<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    threshold: Angle,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<f64, AltitudeCrossingError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let state =
        positions
            .position(body, epoch)
            .map_err(|source| AltitudeCrossingError::Position {
                body,
                epoch,
                source,
            })?;
    let orientation = earth_orientation
        .at(epoch)
        .map_err(|source| AltitudeCrossingError::EarthOrientation { epoch, source })?;
    let actual_authority = earth_orientation.authority();
    let actual_snapshot = earth_orientation.data_snapshot();
    if actual_authority != expected_authority
        || actual_snapshot != expected_snapshot
        || orientation.authority() != expected_authority
        || orientation.snapshot() != expected_snapshot
    {
        return Err(AltitudeCrossingError::EarthOrientationIdentityMismatch {
            epoch,
            expected_authority: expected_authority.to_owned(),
            expected_snapshot: expected_snapshot.to_owned(),
            actual_authority: if actual_authority != expected_authority {
                actual_authority.to_owned()
            } else {
                orientation.authority().to_owned()
            },
            actual_snapshot: if actual_snapshot != expected_snapshot {
                actual_snapshot.to_owned()
            } else {
                orientation.snapshot().to_owned()
            },
        });
    }
    let observation = ObserverTransform::at(epoch, orientation, observer)
        .observe(state)
        .map_err(|source| AltitudeCrossingError::Transform {
            body,
            epoch,
            source,
        })?;
    Ok(observation.value().horizon().latitude().radians() - threshold.radians())
}

fn refine_altitude_crossing<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    threshold: Angle,
    mut left: AltitudeSample,
    mut right: AltitudeSample,
    tolerance_days: f64,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<EventInterval, AltitudeCrossingError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    while right.epoch.day() - left.epoch.day() > tolerance_days {
        let epoch = JulianDate::from_julian_day((left.epoch.day() + right.epoch.day()) / 2.0)
            .expect("a finite altitude bracket has a finite midpoint");
        let middle = AltitudeSample {
            epoch,
            signed_altitude: observer_signed_altitude(
                positions,
                earth_orientation,
                observer,
                body,
                epoch,
                threshold,
                expected_authority,
                expected_snapshot,
            )?,
        };
        if middle.signed_altitude == 0.0 {
            return Ok(EventInterval {
                start: epoch,
                end: epoch,
            });
        }
        if left.signed_altitude.signum() == middle.signed_altitude.signum() {
            left = middle;
        } else {
            right = middle;
        }
    }
    Ok(EventInterval {
        start: left.epoch,
        end: right.epoch,
    })
}

fn exact_crossing_kind(before: Option<f64>, after: Option<f64>) -> Option<AltitudeCrossingKind> {
    match (before, after) {
        (Some(left), Some(right)) if left.signum() != right.signum() => {
            if right > 0.0 {
                Some(AltitudeCrossingKind::Ascending)
            } else {
                Some(AltitudeCrossingKind::Descending)
            }
        }
        (None, Some(right)) => {
            if right > 0.0 {
                Some(AltitudeCrossingKind::Ascending)
            } else {
                Some(AltitudeCrossingKind::Descending)
            }
        }
        (Some(left), None) => {
            if left < 0.0 {
                Some(AltitudeCrossingKind::Ascending)
            } else {
                Some(AltitudeCrossingKind::Descending)
            }
        }
        _ => None,
    }
}

fn push_altitude_crossing<P>(
    events: &mut Vec<AirlessAltitudeCrossing>,
    body: ApparentBody,
    kind: AltitudeCrossingKind,
    interval: EventInterval,
    observer: Observer,
    threshold: Angle,
    positions: &P,
    orientation_authority: &str,
    orientation_snapshot: &str,
) where
    P: GeocentricPositionProvider,
{
    events.push(AirlessAltitudeCrossing {
        body,
        kind,
        interval,
        observer,
        threshold,
        provider_model: positions.model(),
        provider_snapshot: positions.data_snapshot().map(str::to_owned),
        transform_model: AIRLESS_TOPOCENTRIC_TRANSFORM,
        earth_orientation_authority: orientation_authority.to_owned(),
        earth_orientation_snapshot: orientation_snapshot.to_owned(),
    });
}

/// Find every apparent ecliptic-longitude conjunction in a TT search window.
///
/// The returned interval width is bounded by `window.tolerance_days()`. The
/// interval describes numerical search uncertainty; the event separately
/// records which position model supplied its facts.
pub fn ecliptic_longitude_conjunctions<P>(
    provider: &P,
    first: ApparentBody,
    second: ApparentBody,
    window: SearchWindow,
) -> Result<Vec<EclipticLongitudeConjunction>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    if first == second {
        return Err(EventError::SameBody);
    }

    let mut events = Vec::new();
    let mut left_epoch = window.start;
    let mut left_raw = relative_longitude(provider, first, second, left_epoch)?;
    let mut left_continuous = left_raw;

    while left_epoch.day() < window.end.day() {
        let right_day = (left_epoch.day() + window.step_days).min(window.end.day());
        let right_epoch = JulianDate::from_julian_day(right_day)
            .expect("a bounded step between finite epochs remains finite");
        let right_raw = relative_longitude(provider, first, second, right_epoch)?;
        let right_continuous = left_continuous + signed_angle(right_raw - left_raw);

        if let Some(target) = crossed_conjunction(left_continuous, right_continuous) {
            let interval = refine_relative_longitude(
                provider,
                first,
                second,
                left_epoch,
                right_epoch,
                left_continuous,
                right_continuous,
                target,
                window.tolerance_days,
            )?;
            let duplicate = events
                .last()
                .map(|event: &EclipticLongitudeConjunction| {
                    event.interval.end().day() >= interval.start().day()
                })
                .unwrap_or(false);
            if !duplicate {
                let midpoint = interval.midpoint();
                let first_state = provider_position(provider, first, midpoint)?;
                let second_state = provider_position(provider, second, midpoint)?;
                events.push(EclipticLongitudeConjunction {
                    first,
                    second,
                    interval,
                    angular_separation: angular_separation(first_state, second_state),
                    provider_model: provider.model(),
                    provider_snapshot: provider.data_snapshot().map(str::to_owned),
                });
            }
        }

        left_epoch = right_epoch;
        left_raw = right_raw;
        left_continuous = right_continuous;
    }

    Ok(events)
}

/// Find all four apparent ecliptic-longitude lunar phases in a TT window.
///
/// The phase angle is the Moon's apparent ecliptic longitude east of the Sun:
/// 0 degrees for new moon, then 90, 180, and 270 degrees for first quarter,
/// full moon, and last quarter. Latitude is not discarded: each event also
/// reports the great-circle center separation at its interval midpoint.
pub fn ecliptic_longitude_lunar_phases<P>(
    provider: &P,
    window: SearchWindow,
) -> Result<Vec<LunarPhaseEvent>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let mut events = Vec::new();
    let mut left_epoch = window.start;
    let mut left_raw =
        relative_longitude(provider, ApparentBody::Moon, ApparentBody::Sun, left_epoch)?;
    let mut left_continuous = left_raw;

    while left_epoch.day() < window.end.day() {
        let right_day = (left_epoch.day() + window.step_days).min(window.end.day());
        let right_epoch = JulianDate::from_julian_day(right_day)
            .expect("a bounded step between finite epochs remains finite");
        let right_raw =
            relative_longitude(provider, ApparentBody::Moon, ApparentBody::Sun, right_epoch)?;
        let right_continuous = left_continuous + signed_angle(right_raw - left_raw);

        if let Some((target, phase)) = crossed_lunar_phase(left_continuous, right_continuous) {
            let interval = refine_relative_longitude(
                provider,
                ApparentBody::Moon,
                ApparentBody::Sun,
                left_epoch,
                right_epoch,
                left_continuous,
                right_continuous,
                target,
                window.tolerance_days,
            )?;
            let duplicate = events
                .last()
                .map(|event: &LunarPhaseEvent| {
                    event.interval.end().day() >= interval.start().day() && event.phase == phase
                })
                .unwrap_or(false);
            if !duplicate {
                let midpoint = interval.midpoint();
                let moon = provider_position(provider, ApparentBody::Moon, midpoint)?;
                let sun = provider_position(provider, ApparentBody::Sun, midpoint)?;
                events.push(LunarPhaseEvent {
                    phase,
                    interval,
                    angular_separation: angular_separation(moon, sun),
                    provider_model: provider.model(),
                    provider_snapshot: provider.data_snapshot().map(str::to_owned),
                });
            }
        }

        left_epoch = right_epoch;
        left_raw = right_raw;
        left_continuous = right_continuous;
    }

    Ok(events)
}

/// Find new- and full-moon events whose geometry permits an eclipse.
///
/// Solar candidates use geocentric disk overlap plus a conservative allowance
/// for observer parallax. They establish global possibility, not local
/// visibility or contact times. Lunar candidates compare the Moon with the
/// atmosphere-free spherical umbra and penumbra at the phase midpoint. The
/// returned interval is the underlying ecliptic-longitude phase interval.
pub fn eclipse_candidates<P>(
    provider: &P,
    window: SearchWindow,
) -> Result<Vec<EclipseCandidate>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let phases = ecliptic_longitude_lunar_phases(provider, window)?;
    let mut candidates = Vec::new();

    for phase in phases {
        if phase.phase != LunarPhase::NewMoon && phase.phase != LunarPhase::FullMoon {
            continue;
        }
        let epoch = phase.interval.midpoint();
        let moon = provider_position(provider, ApparentBody::Moon, epoch)?;
        let sun = provider_position(provider, ApparentBody::Sun, epoch)?;
        let candidate = match phase.phase {
            LunarPhase::NewMoon => solar_eclipse_candidate(provider, phase.interval, moon, sun)?,
            LunarPhase::FullMoon => lunar_eclipse_candidate(provider, phase.interval, moon, sun)?,
            LunarPhase::FirstQuarter | LunarPhase::LastQuarter => None,
        };
        if let Some(candidate) = candidate {
            candidates.push(candidate);
        }
    }

    Ok(candidates)
}

/// Refine greatest eclipse and every shadow contact for lunar eclipses.
///
/// Greatest eclipse is the minimum geocentric Moon-to-shadow-axis offset
/// within the caller-selected circumstance span. Contacts are tangencies with
/// the revision-1 atmosphere-free spherical shadow. The span endpoints must
/// both lie outside the penumbra, which makes each ingress and egress root
/// independently bracketed against greatest eclipse.
pub fn lunar_eclipse_circumstances<P>(
    provider: &P,
    search: LunarEclipseSearch,
) -> Result<Vec<LunarEclipseCircumstances>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let phases = ecliptic_longitude_lunar_phases(provider, search.phase_window)?;
    let mut events = Vec::new();

    for phase in phases {
        if phase.phase != LunarPhase::FullMoon {
            continue;
        }
        let phase_epoch = phase.interval.midpoint();
        let half_span = search.circumstance_span_days / 2.0;
        let start = phase_epoch
            .offset_days(-half_span)
            .expect("validated phase and span produce a finite start");
        let end = phase_epoch
            .offset_days(half_span)
            .expect("validated phase and span produce a finite end");
        let greatest_interval =
            refine_lunar_eclipse_minimum(provider, start, end, search.phase_window.tolerance_days)?;
        let greatest_epoch = greatest_interval.midpoint();
        let greatest = lunar_shadow_geometry_at(provider, greatest_epoch)?;
        let kind = match lunar_eclipse_kind(greatest) {
            Some(kind) => kind,
            None => continue,
        };
        let start_geometry = lunar_shadow_geometry_at(provider, start)?;
        let end_geometry = lunar_shadow_geometry_at(provider, end)?;
        if lunar_contact_value(start_geometry, LunarContactBoundary::Penumbral) <= 0.0
            || lunar_contact_value(end_geometry, LunarContactBoundary::Penumbral) <= 0.0
        {
            return Err(EventError::CircumstanceSpanTooShort {
                phase_epoch,
                span_days: search.circumstance_span_days,
            });
        }

        let mut contacts: Vec<LunarEclipseContact> = Vec::new();
        push_lunar_contact_pair(
            provider,
            start,
            greatest_epoch,
            end,
            greatest,
            LunarContactBoundary::Penumbral,
            LunarEclipseContactKind::PenumbralIngress,
            LunarEclipseContactKind::PenumbralEgress,
            search.phase_window.tolerance_days,
            &mut contacts,
        )?;
        if kind == LunarEclipseKind::Partial || kind == LunarEclipseKind::Total {
            push_lunar_contact_pair(
                provider,
                start,
                greatest_epoch,
                end,
                greatest,
                LunarContactBoundary::Umbral,
                LunarEclipseContactKind::UmbralIngress,
                LunarEclipseContactKind::UmbralEgress,
                search.phase_window.tolerance_days,
                &mut contacts,
            )?;
        }
        if kind == LunarEclipseKind::Total {
            push_lunar_contact_pair(
                provider,
                start,
                greatest_epoch,
                end,
                greatest,
                LunarContactBoundary::Total,
                LunarEclipseContactKind::TotalityBegins,
                LunarEclipseContactKind::TotalityEnds,
                search.phase_window.tolerance_days,
                &mut contacts,
            )?;
        }
        contacts.sort_by(|first, second| {
            first
                .interval
                .midpoint()
                .day()
                .partial_cmp(&second.interval.midpoint().day())
                .expect("contact epochs are finite")
        });

        events.push(LunarEclipseCircumstances {
            kind,
            phase_interval: phase.interval,
            greatest_interval,
            greatest_geometry: greatest.public,
            contacts,
            circumstance_span_days: search.circumstance_span_days,
            geometry_model: SPHERICAL_ECLIPSE_GEOMETRY,
            provider_model: provider.model(),
            provider_snapshot: provider.data_snapshot().map(str::to_owned),
        });
    }

    Ok(events)
}

/// Find every reversal of apparent ecliptic-longitude motion in a TT window.
///
/// Speed is the signed longitude change across the caller-selected central
/// difference span. A positive-to-negative root begins retrograde motion; a
/// negative-to-positive root begins direct motion. Provider requests extend
/// half the velocity span beyond each evaluated epoch, and any resulting
/// range failure remains an [`EventError::Position`].
pub fn ecliptic_longitude_stations<P>(
    provider: &P,
    body: ApparentBody,
    search: StationSearch,
) -> Result<Vec<EclipticLongitudeStation>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let window = search.window;
    let mut events = Vec::new();
    let mut left_epoch = window.start;
    let sample_nudge_days = window.step_days / 1_048_576.0;
    let mut left_speed = sampled_longitude_speed(
        provider,
        body,
        left_epoch,
        search.velocity_span_days,
        sample_nudge_days,
    )?;

    while left_epoch.day() < window.end.day() {
        let right_day = (left_epoch.day() + window.step_days).min(window.end.day());
        let right_epoch = JulianDate::from_julian_day(right_day)
            .expect("a bounded step between finite epochs remains finite");
        let right_speed = sampled_longitude_speed(
            provider,
            body,
            right_epoch,
            search.velocity_span_days,
            sample_nudge_days,
        )?;

        if reverses_motion(left_speed, right_speed) {
            let motion_before = motion(left_speed);
            let motion_after = motion(right_speed);
            let interval = refine_station(
                provider,
                body,
                left_epoch,
                right_epoch,
                left_speed,
                right_speed,
                search.velocity_span_days,
                window.tolerance_days,
            )?;
            let duplicate = events
                .last()
                .map(|event: &EclipticLongitudeStation| {
                    event.interval.end().day() >= interval.start().day()
                })
                .unwrap_or(false);
            if !duplicate {
                let midpoint = interval.midpoint();
                let state = provider_position(provider, body, midpoint)?;
                events.push(EclipticLongitudeStation {
                    body,
                    interval,
                    longitude: state.direction().longitude(),
                    motion_before,
                    motion_after,
                    velocity_span_days: search.velocity_span_days,
                    provider_model: provider.model(),
                    provider_snapshot: provider.data_snapshot().map(str::to_owned),
                });
            }
        }

        left_epoch = right_epoch;
        left_speed = right_speed;
    }

    Ok(events)
}

fn refine_station<P>(
    provider: &P,
    body: ApparentBody,
    mut left_epoch: JulianDate<TerrestrialTime>,
    mut right_epoch: JulianDate<TerrestrialTime>,
    mut left_speed: f64,
    mut right_speed: f64,
    velocity_span_days: f64,
    tolerance_days: f64,
) -> Result<EventInterval, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    while right_epoch.day() - left_epoch.day() > tolerance_days {
        let midpoint = JulianDate::from_julian_day((left_epoch.day() + right_epoch.day()) / 2.0)
            .expect("a finite bracket has a finite midpoint");
        let middle_speed = longitude_speed(provider, body, midpoint, velocity_span_days)?;
        if left_speed == 0.0 {
            right_epoch = left_epoch;
            break;
        } else if right_speed == 0.0 {
            left_epoch = right_epoch;
            break;
        } else if left_speed.signum() == middle_speed.signum() {
            left_epoch = midpoint;
            left_speed = middle_speed;
        } else {
            right_epoch = midpoint;
            right_speed = middle_speed;
        }
    }
    Ok(EventInterval {
        start: left_epoch,
        end: right_epoch,
    })
}

fn longitude_speed<P>(
    provider: &P,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    velocity_span_days: f64,
) -> Result<f64, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let half_span = velocity_span_days / 2.0;
    let before_epoch = epoch
        .offset_days(-half_span)
        .expect("finite station sample offset");
    let after_epoch = epoch
        .offset_days(half_span)
        .expect("finite station sample offset");
    let before = provider_position(provider, body, before_epoch)?
        .direction()
        .longitude()
        .radians();
    let after = provider_position(provider, body, after_epoch)?
        .direction()
        .longitude()
        .radians();
    Ok(signed_angle(after - before) / velocity_span_days)
}

fn sampled_longitude_speed<P>(
    provider: &P,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    velocity_span_days: f64,
    nudge_days: f64,
) -> Result<f64, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let speed = longitude_speed(provider, body, epoch, velocity_span_days)?;
    if speed != 0.0 {
        return Ok(speed);
    }
    let just_after = epoch
        .offset_days(nudge_days)
        .expect("finite station sample nudge");
    longitude_speed(provider, body, just_after, velocity_span_days)
}

fn reverses_motion(left_speed: f64, right_speed: f64) -> bool {
    left_speed != 0.0 && right_speed != 0.0 && left_speed.signum() != right_speed.signum()
}

fn motion(speed: f64) -> LongitudeMotion {
    if speed < 0.0 {
        LongitudeMotion::Retrograde
    } else {
        LongitudeMotion::Direct
    }
}

fn refine_relative_longitude<P>(
    provider: &P,
    first: ApparentBody,
    second: ApparentBody,
    mut left_epoch: JulianDate<TerrestrialTime>,
    mut right_epoch: JulianDate<TerrestrialTime>,
    mut left_value: f64,
    mut right_value: f64,
    target: f64,
    tolerance_days: f64,
) -> Result<EventInterval, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    left_value -= target;
    right_value -= target;
    while right_epoch.day() - left_epoch.day() > tolerance_days {
        let midpoint = JulianDate::from_julian_day((left_epoch.day() + right_epoch.day()) / 2.0)
            .expect("a finite bracket has a finite midpoint");
        let raw = relative_longitude(provider, first, second, midpoint)?;
        let middle_value = unwrap_near(raw, target) - target;
        if left_value == 0.0 {
            right_epoch = left_epoch;
            break;
        } else if right_value == 0.0 {
            left_epoch = right_epoch;
            break;
        } else if left_value.signum() == middle_value.signum() {
            left_epoch = midpoint;
            left_value = middle_value;
        } else {
            right_epoch = midpoint;
            right_value = middle_value;
        }
    }
    Ok(EventInterval {
        start: left_epoch,
        end: right_epoch,
    })
}

fn relative_longitude<P>(
    provider: &P,
    first: ApparentBody,
    second: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<f64, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let first_state = provider_position(provider, first, epoch)?;
    let second_state = provider_position(provider, second, epoch)?;
    Ok(signed_angle(
        first_state.direction().longitude().radians()
            - second_state.direction().longitude().radians(),
    ))
}

fn provider_position<P>(
    provider: &P,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<State<TrueEclipticEquinoxOfDate>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    provider
        .position(body, epoch)
        .map_err(|source| EventError::Position {
            body,
            epoch,
            source,
        })
}

fn crossed_conjunction(left: f64, right: f64) -> Option<f64> {
    let lower = left.min(right);
    let upper = left.max(right);
    let target = (lower / TWO_PI).ceil() * TWO_PI;
    if target <= upper {
        Some(target)
    } else {
        None
    }
}

fn crossed_lunar_phase(left: f64, right: f64) -> Option<(f64, LunarPhase)> {
    let quarter_turn = PI / 2.0;
    let lower = left.min(right);
    let upper = left.max(right);
    let quarter_index = (lower / quarter_turn).ceil() as i64;
    let target = quarter_index as f64 * quarter_turn;
    if target > upper {
        return None;
    }
    let phase = match quarter_index.rem_euclid(4) {
        0 => LunarPhase::NewMoon,
        1 => LunarPhase::FirstQuarter,
        2 => LunarPhase::FullMoon,
        _ => LunarPhase::LastQuarter,
    };
    Some((target, phase))
}

fn signed_angle(angle: f64) -> f64 {
    (angle + PI).rem_euclid(TWO_PI) - PI
}

fn unwrap_near(angle: f64, target: f64) -> f64 {
    angle + ((target - angle) / TWO_PI).round() * TWO_PI
}

fn angular_separation(
    first: State<TrueEclipticEquinoxOfDate>,
    second: State<TrueEclipticEquinoxOfDate>,
) -> Angle {
    let first = first.direction();
    let second = second.direction();
    let cosine = first.latitude().radians().sin() * second.latitude().radians().sin()
        + first.latitude().radians().cos()
            * second.latitude().radians().cos()
            * (first.longitude().radians() - second.longitude().radians()).cos();
    Angle::from_radians(cosine.max(-1.0).min(1.0).acos())
        .expect("finite directions produce a finite separation")
}

fn solar_eclipse_candidate<P>(
    provider: &P,
    interval: EventInterval,
    moon: State<TrueEclipticEquinoxOfDate>,
    sun: State<TrueEclipticEquinoxOfDate>,
) -> Result<Option<EclipseCandidate>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let epoch = interval.midpoint();
    let moon_distance = eclipse_distance(
        ApparentBody::Moon,
        epoch,
        moon.distance().kilometers(),
        WGS84_EARTH_EQUATORIAL_RADIUS_KM.max(MEAN_LUNAR_RADIUS_KM),
    )?;
    let sun_distance = eclipse_distance(
        ApparentBody::Sun,
        epoch,
        sun.distance().kilometers(),
        NOMINAL_SOLAR_RADIUS_KM,
    )?;
    let center_separation = angular_separation(moon, sun);
    let moon_angular_radius = angular_radius(MEAN_LUNAR_RADIUS_KM, moon_distance);
    let sun_angular_radius = angular_radius(NOMINAL_SOLAR_RADIUS_KM, sun_distance);
    let observer_parallax_allowance = Angle::from_radians(
        (WGS84_EARTH_EQUATORIAL_RADIUS_KM / moon_distance).asin()
            + (WGS84_EARTH_EQUATORIAL_RADIUS_KM / sun_distance).asin(),
    )
    .expect("validated distances produce finite horizontal parallax");
    let candidate_limit = moon_angular_radius.radians()
        + sun_angular_radius.radians()
        + observer_parallax_allowance.radians();
    if center_separation.radians() > candidate_limit {
        return Ok(None);
    }

    Ok(Some(EclipseCandidate {
        kind: EclipseCandidateKind::Solar,
        interval,
        geometry: EclipseCandidateGeometry::Solar {
            center_separation,
            sun_angular_radius,
            moon_angular_radius,
            observer_parallax_allowance,
        },
        geometry_model: SPHERICAL_ECLIPSE_GEOMETRY,
        provider_model: provider.model(),
        provider_snapshot: provider.data_snapshot().map(str::to_owned),
    }))
}

fn lunar_eclipse_candidate<P>(
    provider: &P,
    interval: EventInterval,
    moon: State<TrueEclipticEquinoxOfDate>,
    sun: State<TrueEclipticEquinoxOfDate>,
) -> Result<Option<EclipseCandidate>, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let epoch = interval.midpoint();
    let geometry = lunar_shadow_geometry_from_states(epoch, moon, sun)?;
    let kind = match lunar_eclipse_kind(geometry) {
        Some(LunarEclipseKind::Penumbral) => EclipseCandidateKind::PenumbralLunar,
        Some(LunarEclipseKind::Partial) => EclipseCandidateKind::PartialLunar,
        Some(LunarEclipseKind::Total) => EclipseCandidateKind::TotalLunar,
        None => return Ok(None),
    };

    Ok(Some(EclipseCandidate {
        kind,
        interval,
        geometry: EclipseCandidateGeometry::Lunar {
            shadow_axis_separation: geometry.public.shadow_axis_separation,
            moon_angular_radius: geometry.public.moon_angular_radius,
            umbra_angular_radius: geometry.public.umbra_angular_radius,
            penumbra_angular_radius: geometry.public.penumbra_angular_radius,
        },
        geometry_model: SPHERICAL_ECLIPSE_GEOMETRY,
        provider_model: provider.model(),
        provider_snapshot: provider.data_snapshot().map(str::to_owned),
    }))
}

fn lunar_shadow_geometry_at<P>(
    provider: &P,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<LunarShadowGeometry, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let moon = provider_position(provider, ApparentBody::Moon, epoch)?;
    let sun = provider_position(provider, ApparentBody::Sun, epoch)?;
    lunar_shadow_geometry_from_states(epoch, moon, sun)
}

fn lunar_shadow_geometry_from_states<E>(
    epoch: JulianDate<TerrestrialTime>,
    moon: State<TrueEclipticEquinoxOfDate>,
    sun: State<TrueEclipticEquinoxOfDate>,
) -> Result<LunarShadowGeometry, EventError<E>> {
    let moon_distance = eclipse_distance(
        ApparentBody::Moon,
        epoch,
        moon.distance().kilometers(),
        MEAN_LUNAR_RADIUS_KM,
    )?;
    let sun_distance = eclipse_distance(
        ApparentBody::Sun,
        epoch,
        sun.distance().kilometers(),
        NOMINAL_SOLAR_RADIUS_KM,
    )?;
    let shadow_axis_separation = Angle::from_radians(
        (PI - angular_separation(moon, sun).radians())
            .max(0.0)
            .min(PI),
    )
    .expect("finite directions produce a finite antisolar separation");
    let moon_angular_radius = angular_radius(MEAN_LUNAR_RADIUS_KM, moon_distance);
    let axis = shadow_axis_separation.radians();
    let axial_distance_km = moon_distance * axis.cos();
    let shadow_axis_offset_km = moon_distance * axis.sin();
    let umbra_radius_km = WGS84_EARTH_EQUATORIAL_RADIUS_KM
        - axial_distance_km * (NOMINAL_SOLAR_RADIUS_KM - WGS84_EARTH_EQUATORIAL_RADIUS_KM)
            / sun_distance;
    let penumbra_radius_km = WGS84_EARTH_EQUATORIAL_RADIUS_KM
        + axial_distance_km * (NOMINAL_SOLAR_RADIUS_KM + WGS84_EARTH_EQUATORIAL_RADIUS_KM)
            / sun_distance;
    let umbra_angular_radius =
        Angle::from_radians(umbra_radius_km.max(0.0).atan2(axial_distance_km))
            .expect("validated distances produce a finite umbra radius");
    let penumbra_angular_radius = Angle::from_radians(penumbra_radius_km.atan2(axial_distance_km))
        .expect("validated distances produce a finite penumbra radius");

    Ok(LunarShadowGeometry {
        public: LunarEclipseGeometry {
            shadow_axis_separation,
            shadow_axis_offset: Distance::from_kilometers(shadow_axis_offset_km)
                .expect("a finite transverse offset is a distance"),
            moon_angular_radius,
            umbra_angular_radius,
            umbra_radius: Distance::from_kilometers(umbra_radius_km.max(0.0))
                .expect("a finite shadow radius is a distance"),
            penumbra_angular_radius,
            penumbra_radius: Distance::from_kilometers(penumbra_radius_km)
                .expect("a finite shadow radius is a distance"),
        },
        shadow_axis_offset_km,
        umbra_radius_km,
        penumbra_radius_km,
    })
}

fn lunar_eclipse_kind(geometry: LunarShadowGeometry) -> Option<LunarEclipseKind> {
    if geometry.shadow_axis_offset_km + MEAN_LUNAR_RADIUS_KM <= geometry.umbra_radius_km {
        Some(LunarEclipseKind::Total)
    } else if geometry.shadow_axis_offset_km <= geometry.umbra_radius_km + MEAN_LUNAR_RADIUS_KM {
        Some(LunarEclipseKind::Partial)
    } else if geometry.shadow_axis_offset_km <= geometry.penumbra_radius_km + MEAN_LUNAR_RADIUS_KM {
        Some(LunarEclipseKind::Penumbral)
    } else {
        None
    }
}

fn lunar_contact_value(geometry: LunarShadowGeometry, boundary: LunarContactBoundary) -> f64 {
    match boundary {
        LunarContactBoundary::Penumbral => {
            geometry.shadow_axis_offset_km - geometry.penumbra_radius_km - MEAN_LUNAR_RADIUS_KM
        }
        LunarContactBoundary::Umbral => {
            geometry.shadow_axis_offset_km - geometry.umbra_radius_km - MEAN_LUNAR_RADIUS_KM
        }
        LunarContactBoundary::Total => {
            geometry.shadow_axis_offset_km + MEAN_LUNAR_RADIUS_KM - geometry.umbra_radius_km
        }
    }
}

fn refine_lunar_eclipse_minimum<P>(
    provider: &P,
    mut left: JulianDate<TerrestrialTime>,
    mut right: JulianDate<TerrestrialTime>,
    tolerance_days: f64,
) -> Result<EventInterval, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    while right.day() - left.day() > tolerance_days {
        let third = (right.day() - left.day()) / 3.0;
        let first = left
            .offset_days(third)
            .expect("a finite minimum bracket has a finite interior point");
        let second = right
            .offset_days(-third)
            .expect("a finite minimum bracket has a finite interior point");
        let first_offset = lunar_shadow_geometry_at(provider, first)?.shadow_axis_offset_km;
        let second_offset = lunar_shadow_geometry_at(provider, second)?.shadow_axis_offset_km;
        if first_offset <= second_offset {
            right = second;
        } else {
            left = first;
        }
    }
    Ok(EventInterval {
        start: left,
        end: right,
    })
}

fn push_lunar_contact_pair<P>(
    provider: &P,
    start: JulianDate<TerrestrialTime>,
    greatest_epoch: JulianDate<TerrestrialTime>,
    end: JulianDate<TerrestrialTime>,
    greatest: LunarShadowGeometry,
    boundary: LunarContactBoundary,
    ingress_kind: LunarEclipseContactKind,
    egress_kind: LunarEclipseContactKind,
    tolerance_days: f64,
    contacts: &mut Vec<LunarEclipseContact>,
) -> Result<(), EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    if lunar_contact_value(greatest, boundary) > 0.0 {
        return Ok(());
    }
    let ingress = refine_lunar_contact(provider, start, greatest_epoch, boundary, tolerance_days)?;
    let egress = refine_lunar_contact(provider, greatest_epoch, end, boundary, tolerance_days)?;
    contacts.push(LunarEclipseContact {
        kind: ingress_kind,
        interval: ingress,
    });
    contacts.push(LunarEclipseContact {
        kind: egress_kind,
        interval: egress,
    });
    Ok(())
}

fn refine_lunar_contact<P>(
    provider: &P,
    mut left: JulianDate<TerrestrialTime>,
    mut right: JulianDate<TerrestrialTime>,
    boundary: LunarContactBoundary,
    tolerance_days: f64,
) -> Result<EventInterval, EventError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let mut left_value = lunar_contact_value(lunar_shadow_geometry_at(provider, left)?, boundary);
    let right_value = lunar_contact_value(lunar_shadow_geometry_at(provider, right)?, boundary);
    debug_assert!(
        left_value == 0.0 || right_value == 0.0 || left_value.signum() != right_value.signum()
    );
    while right.day() - left.day() > tolerance_days {
        let midpoint = JulianDate::from_julian_day((left.day() + right.day()) / 2.0)
            .expect("a finite contact bracket has a finite midpoint");
        let middle_value =
            lunar_contact_value(lunar_shadow_geometry_at(provider, midpoint)?, boundary);
        if left_value == 0.0 {
            right = left;
            break;
        } else if middle_value == 0.0 {
            left = midpoint;
            right = midpoint;
            break;
        } else if left_value.signum() == middle_value.signum() {
            left = midpoint;
            left_value = middle_value;
        } else {
            right = midpoint;
        }
    }
    Ok(EventInterval {
        start: left,
        end: right,
    })
}

fn eclipse_distance<E>(
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    distance_km: f64,
    required_greater_than_km: f64,
) -> Result<f64, EventError<E>> {
    if distance_km <= required_greater_than_km {
        return Err(EventError::DistanceTooSmall {
            body,
            epoch,
            distance_km,
            required_greater_than_km,
        });
    }
    Ok(distance_km)
}

fn angular_radius(radius_km: f64, distance_km: f64) -> Angle {
    Angle::from_radians((radius_km / distance_km).asin())
        .expect("validated radii and distances produce a finite angular radius")
}

#[cfg(test)]
mod altitude_crossing_tests {
    use super::{exact_crossing_kind, AltitudeCrossingKind};

    #[test]
    fn exact_sample_requires_crossing_signs_in_the_interior() {
        assert_eq!(
            exact_crossing_kind(Some(-1.0), Some(1.0)),
            Some(AltitudeCrossingKind::Ascending)
        );
        assert_eq!(
            exact_crossing_kind(Some(1.0), Some(-1.0)),
            Some(AltitudeCrossingKind::Descending)
        );
        assert_eq!(exact_crossing_kind(Some(1.0), Some(1.0)), None);
        assert_eq!(exact_crossing_kind(Some(-1.0), Some(-1.0)), None);
    }

    #[test]
    fn search_boundary_zero_uses_the_available_one_sided_sign() {
        assert_eq!(
            exact_crossing_kind(None, Some(1.0)),
            Some(AltitudeCrossingKind::Ascending)
        );
        assert_eq!(
            exact_crossing_kind(Some(1.0), None),
            Some(AltitudeCrossingKind::Descending)
        );
        assert_eq!(exact_crossing_kind(None, None), None);
    }
}
