// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Provider-neutral celestial event searches.
//!
//! T4 event searches currently cover apparent ecliptic-longitude conjunctions,
//! stationary points, lunar quarter phases, eclipse candidates, lunar eclipse
//! circumstances, airless observer-altitude crossings and extrema, sampled
//! altitude-threshold circumstances, airless and caller-composed conventional
//! rise/set naming, and topocentric meridian transits. Every event result is a
//! bounded TT interval, not an isolated floating-point instant.

use std::f64::consts::PI;
use std::fmt;

use apparent::ApparentBody;
use foundation::{
    Angle, Distance, JulianDate, Longitude, Model, Observer, State, TerrestrialTime,
    TrueEclipticEquinoxOfDate,
};
use observer::{
    Observation, ObserverTransform, ObserverTransformError, AIRLESS_TOPOCENTRIC_TRANSFORM,
};
use provider::{EarthOrientationProvider, GeocentricPositionProvider};

mod solar_eclipse;
pub use self::solar_eclipse::{
    local_solar_eclipse_circumstances, LocalSolarEclipseCircumstances,
    LocalSolarEclipseContact, LocalSolarEclipseContactKind, LocalSolarEclipseError,
    LocalSolarEclipseGeometry, LocalSolarEclipseKind, LocalSolarEclipseSearch,
    LocalSolarEclipseSearchError, LocalSolarEclipseVisibility,
    LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCES, MAX_LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS,
};

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

/// Maximum sampling step accepted by an altitude-extremum search.
pub const MAX_ALTITUDE_EXTREMUM_STEP_DAYS: f64 = MAX_ALTITUDE_CROSSING_STEP_DAYS;

/// Maximum full central-difference span used to classify altitude motion.
pub const MAX_ALTITUDE_DERIVATIVE_SPAN_DAYS: f64 = 1.0 / 24.0;

/// Maximum sampling step accepted by a meridian-transit search.
///
/// The one-hour ceiling bounds the sampled local-hour-angle contract. It does
/// not prove the absence of tangencies or multiple meridian crossings.
pub const MAX_MERIDIAN_TRANSIT_STEP_DAYS: f64 = 1.0 / 24.0;

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

/// Naming model for a caller-selected airless center-altitude crossing.
///
/// It deliberately excludes refraction, apparent limb, horizon dip, terrain,
/// obstruction, civil convention, and visibility policy.
pub const AIRLESS_RISE_SET_NAMING: Model =
    Model::new("caller-threshold airless center rise/set naming", "1");

/// Composition model for conventional observer-relative rise and set events.
///
/// The selected refraction, limb, and horizon-dip models remain separate in
/// each result. Turquet does not select a civil, terrain, obstruction, or
/// visibility convention.
pub const CONVENTIONAL_RISE_SET_CIRCUMSTANCES: Model =
    Model::new("caller-composed conventional rise/set circumstances", "1");

/// A model which applies no atmospheric refraction at the selected crossing.
pub const NO_REFRACTION: Model = Model::new("no atmospheric refraction", "1");

/// USNO's fixed 34-arcminute standard-horizon refraction convention.
pub const USNO_STANDARD_REFRACTION: Model =
    Model::new("USNO standard-horizon fixed refraction", "2026-08-25");

/// USNO's fixed 16-arcminute average upper solar-limb convention.
pub const USNO_STANDARD_SOLAR_LIMB: Model = Model::new(
    "USNO standard-horizon average solar upper limb",
    "2026-08-25",
);

/// A model which selects the body's center rather than either limb.
pub const CENTER_LIMB: Model = Model::new("center limb selection", "1");

/// A model which takes the observer's astronomical horizon as level.
pub const LEVEL_HORIZON_DIP: Model = Model::new("level astronomical horizon", "1");

/// Topocentric apparent local-meridian event model.
pub const TOPOCENTRIC_MERIDIAN_TRANSIT_MODEL: Model =
    Model::new("topocentric apparent local-meridian transit", "1");

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

/// Validated controls for a conventional rise/set search.
///
/// Conventional circumstances solve the selected policy directly, so they do
/// not take a caller altitude threshold. The airless center altitude at the
/// event can vary with a physical-radius limb model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConventionalRiseSetSearch {
    window: SearchWindow,
}

impl ConventionalRiseSetSearch {
    pub fn new(window: SearchWindow) -> Result<Self, ConventionalRiseSetSearchError> {
        if window.step_days() > MAX_ALTITUDE_CROSSING_STEP_DAYS {
            return Err(ConventionalRiseSetSearchError::StepTooLarge);
        }
        Ok(Self { window })
    }

    pub fn window(self) -> SearchWindow {
        self.window
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConventionalRiseSetSearchError {
    StepTooLarge,
}

impl fmt::Display for ConventionalRiseSetSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConventionalRiseSetSearchError::StepTooLarge => {
                formatter.write_str("conventional rise/set search step exceeds one TT hour")
            }
        }
    }
}

impl ::std::error::Error for ConventionalRiseSetSearchError {}

/// A constant apparent lift applied to the airless center altitude.
///
/// The value is evaluated at the event rather than inferred from weather. A
/// caller can supply zero through [`RefractionModel::none`] or name and select
/// a published fixed-target refraction convention through [`RefractionModel::constant`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RefractionModel {
    apparent_lift: Angle,
    model: Model,
}

impl RefractionModel {
    pub fn none() -> Self {
        Self {
            apparent_lift: Angle::from_radians(0.0).expect("zero is a finite angle"),
            model: NO_REFRACTION,
        }
    }

    pub fn constant(
        apparent_lift: Angle,
        model: Model,
    ) -> Result<Self, ConventionalRiseSetPolicyError> {
        if apparent_lift.radians() < 0.0 || apparent_lift.radians() > PI / 2.0 {
            return Err(ConventionalRiseSetPolicyError::RefractionOutOfRange);
        }
        Ok(Self {
            apparent_lift,
            model,
        })
    }

    /// USNO's 34-arcminute fixed refraction used in its standard sea-level
    /// Sun and Moon rise/set convention.
    pub fn usno_standard() -> Self {
        Self {
            apparent_lift: Angle::from_arcseconds(34.0 * 60.0)
                .expect("34 arcminutes is a finite angle"),
            model: USNO_STANDARD_REFRACTION,
        }
    }

    pub fn apparent_lift(self) -> Angle {
        self.apparent_lift
    }

    pub fn model(self) -> Model {
        self.model
    }
}

/// Which apparent limb is selected at the conventional horizon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LimbModel {
    kind: LimbModelKind,
    model: Model,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum LimbModelKind {
    Center,
    UpperAngularRadius(Angle),
    UpperPhysicalRadius(Distance),
}

impl LimbModel {
    pub fn center() -> Self {
        Self {
            kind: LimbModelKind::Center,
            model: CENTER_LIMB,
        }
    }

    pub fn upper_physical_radius(
        physical_radius: Distance,
        model: Model,
    ) -> Result<Self, ConventionalRiseSetPolicyError> {
        if physical_radius.meters() <= 0.0 {
            return Err(ConventionalRiseSetPolicyError::LimbRadiusNotPositive);
        }
        Ok(Self {
            kind: LimbModelKind::UpperPhysicalRadius(physical_radius),
            model,
        })
    }

    pub fn upper_angular_radius(
        angular_radius: Angle,
        model: Model,
    ) -> Result<Self, ConventionalRiseSetPolicyError> {
        if angular_radius.radians() <= 0.0 || angular_radius.radians() > PI / 2.0 {
            return Err(ConventionalRiseSetPolicyError::LimbAngularRadiusOutOfRange);
        }
        Ok(Self {
            kind: LimbModelKind::UpperAngularRadius(angular_radius),
            model,
        })
    }

    /// USNO's fixed 16-arcminute average upper solar-limb convention.
    pub fn usno_standard_solar() -> Self {
        Self::upper_angular_radius(
            Angle::from_arcseconds(16.0 * 60.0).expect("16 arcminutes is a finite angle"),
            USNO_STANDARD_SOLAR_LIMB,
        )
        .expect("16 arcminutes is a positive upper-limb angle")
    }

    pub fn model(self) -> Model {
        self.model
    }

    pub fn physical_radius(self) -> Option<Distance> {
        match self.kind {
            LimbModelKind::UpperPhysicalRadius(physical_radius) => Some(physical_radius),
            LimbModelKind::Center | LimbModelKind::UpperAngularRadius(_) => None,
        }
    }

    pub fn angular_radius(self) -> Option<Angle> {
        match self.kind {
            LimbModelKind::UpperAngularRadius(angular_radius) => Some(angular_radius),
            LimbModelKind::Center | LimbModelKind::UpperPhysicalRadius(_) => None,
        }
    }
}

/// The selected depression of the visible horizon below the astronomical one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HorizonDipModel {
    kind: HorizonDipModelKind,
    model: Model,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum HorizonDipModelKind {
    Level,
    Constant(Angle),
    Spherical(Distance),
}

impl HorizonDipModel {
    pub fn level() -> Self {
        Self {
            kind: HorizonDipModelKind::Level,
            model: LEVEL_HORIZON_DIP,
        }
    }

    pub fn constant(dip: Angle, model: Model) -> Result<Self, ConventionalRiseSetPolicyError> {
        if dip.radians() < 0.0 || dip.radians() > PI / 2.0 {
            return Err(ConventionalRiseSetPolicyError::HorizonDipOutOfRange);
        }
        Ok(Self {
            kind: HorizonDipModelKind::Constant(dip),
            model,
        })
    }

    pub fn spherical(
        radius: Distance,
        model: Model,
    ) -> Result<Self, ConventionalRiseSetPolicyError> {
        if radius.meters() <= 0.0 {
            return Err(ConventionalRiseSetPolicyError::HorizonRadiusNotPositive);
        }
        Ok(Self {
            kind: HorizonDipModelKind::Spherical(radius),
            model,
        })
    }

    pub fn model(self) -> Model {
        self.model
    }

    pub fn constant_dip(self) -> Option<Angle> {
        match self.kind {
            HorizonDipModelKind::Constant(dip) => Some(dip),
            HorizonDipModelKind::Level | HorizonDipModelKind::Spherical(_) => None,
        }
    }

    pub fn spherical_radius(self) -> Option<Distance> {
        match self.kind {
            HorizonDipModelKind::Spherical(radius) => Some(radius),
            HorizonDipModelKind::Level | HorizonDipModelKind::Constant(_) => None,
        }
    }
}

/// Explicit conventional circumstances selected by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConventionalRiseSetPolicy {
    refraction: RefractionModel,
    limb: LimbModel,
    horizon_dip: HorizonDipModel,
}

impl ConventionalRiseSetPolicy {
    pub fn new(refraction: RefractionModel, limb: LimbModel, horizon_dip: HorizonDipModel) -> Self {
        Self {
            refraction,
            limb,
            horizon_dip,
        }
    }

    pub fn refraction(self) -> RefractionModel {
        self.refraction
    }

    pub fn limb(self) -> LimbModel {
        self.limb
    }

    pub fn horizon_dip(self) -> HorizonDipModel {
        self.horizon_dip
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConventionalRiseSetPolicyError {
    RefractionOutOfRange,
    LimbRadiusNotPositive,
    LimbAngularRadiusOutOfRange,
    HorizonDipOutOfRange,
    HorizonRadiusNotPositive,
}

impl fmt::Display for ConventionalRiseSetPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            ConventionalRiseSetPolicyError::RefractionOutOfRange => {
                "constant refraction must be between zero and 90 degrees"
            }
            ConventionalRiseSetPolicyError::LimbRadiusNotPositive => {
                "physical limb radius must be positive"
            }
            ConventionalRiseSetPolicyError::LimbAngularRadiusOutOfRange => {
                "upper-limb angular radius must be greater than zero and at most 90 degrees"
            }
            ConventionalRiseSetPolicyError::HorizonDipOutOfRange => {
                "horizon dip must be between zero and 90 degrees"
            }
            ConventionalRiseSetPolicyError::HorizonRadiusNotPositive => {
                "spherical horizon radius must be positive"
            }
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for ConventionalRiseSetPolicyError {}

/// Validated numerical controls for sampled topocentric meridian transits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeridianTransitSearch {
    window: SearchWindow,
}

impl MeridianTransitSearch {
    pub fn new(window: SearchWindow) -> Result<Self, MeridianTransitSearchError> {
        if window.step_days() > MAX_MERIDIAN_TRANSIT_STEP_DAYS {
            return Err(MeridianTransitSearchError::StepTooLarge);
        }
        Ok(Self { window })
    }

    pub fn window(self) -> SearchWindow {
        self.window
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeridianTransitSearchError {
    StepTooLarge,
}

impl fmt::Display for MeridianTransitSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MeridianTransitSearchError::StepTooLarge => {
                formatter.write_str("meridian-transit search step exceeds one TT hour")
            }
        }
    }
}

impl ::std::error::Error for MeridianTransitSearchError {}

/// Numerical controls for sampled, bracketed airless altitude extrema.
///
/// `derivative_span_days` is the full interval between the two altitude
/// evaluations in the central difference. Provider requests can therefore
/// extend half this span beyond both ends of the search window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AltitudeExtremumSearch {
    window: SearchWindow,
    derivative_span_days: f64,
}

impl AltitudeExtremumSearch {
    pub fn new(
        window: SearchWindow,
        derivative_span_days: f64,
    ) -> Result<Self, AltitudeExtremumSearchError> {
        if window.step_days() > MAX_ALTITUDE_EXTREMUM_STEP_DAYS {
            return Err(AltitudeExtremumSearchError::StepTooLarge);
        }
        if !derivative_span_days.is_finite() {
            return Err(AltitudeExtremumSearchError::DerivativeSpanNotFinite);
        }
        if derivative_span_days <= 0.0 {
            return Err(AltitudeExtremumSearchError::DerivativeSpanNotPositive);
        }
        if derivative_span_days > MAX_ALTITUDE_DERIVATIVE_SPAN_DAYS {
            return Err(AltitudeExtremumSearchError::DerivativeSpanTooLarge);
        }
        Ok(Self {
            window,
            derivative_span_days,
        })
    }

    pub fn window(self) -> SearchWindow {
        self.window
    }

    pub fn derivative_span_days(self) -> f64 {
        self.derivative_span_days
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeExtremumSearchError {
    StepTooLarge,
    DerivativeSpanNotFinite,
    DerivativeSpanNotPositive,
    DerivativeSpanTooLarge,
}

impl fmt::Display for AltitudeExtremumSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            AltitudeExtremumSearchError::StepTooLarge => {
                "altitude-extremum search step exceeds one TT hour"
            }
            AltitudeExtremumSearchError::DerivativeSpanNotFinite => {
                "altitude derivative span must be finite"
            }
            AltitudeExtremumSearchError::DerivativeSpanNotPositive => {
                "altitude derivative span must be positive"
            }
            AltitudeExtremumSearchError::DerivativeSpanTooLarge => {
                "altitude derivative span exceeds one TT hour"
            }
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for AltitudeExtremumSearchError {}

/// Controls for extrema plus sampled state around one physical altitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AltitudeCircumstanceSearch {
    extrema: AltitudeExtremumSearch,
    threshold: Angle,
    altitude_tolerance: Angle,
}

impl AltitudeCircumstanceSearch {
    pub fn new(
        extrema: AltitudeExtremumSearch,
        threshold: Angle,
        altitude_tolerance: Angle,
    ) -> Result<Self, AltitudeCircumstanceSearchError> {
        if threshold.radians() < -PI / 2.0 || threshold.radians() > PI / 2.0 {
            return Err(AltitudeCircumstanceSearchError::ThresholdOutOfRange);
        }
        if altitude_tolerance.radians() <= 0.0 {
            return Err(AltitudeCircumstanceSearchError::AltitudeToleranceNotPositive);
        }
        if altitude_tolerance.radians() > PI / 2.0 {
            return Err(AltitudeCircumstanceSearchError::AltitudeToleranceTooLarge);
        }
        Ok(Self {
            extrema,
            threshold,
            altitude_tolerance,
        })
    }

    pub fn extrema(self) -> AltitudeExtremumSearch {
        self.extrema
    }

    pub fn threshold(self) -> Angle {
        self.threshold
    }

    pub fn altitude_tolerance(self) -> Angle {
        self.altitude_tolerance
    }

    pub fn crossing(self) -> AltitudeCrossingSearch {
        AltitudeCrossingSearch::new(self.extrema.window(), self.threshold)
            .expect("an extrema search already satisfies the crossing step ceiling")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeCircumstanceSearchError {
    ThresholdOutOfRange,
    AltitudeToleranceNotPositive,
    AltitudeToleranceTooLarge,
}

impl fmt::Display for AltitudeCircumstanceSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            AltitudeCircumstanceSearchError::ThresholdOutOfRange => {
                "altitude threshold must be between minus and plus 90 degrees"
            }
            AltitudeCircumstanceSearchError::AltitudeToleranceNotPositive => {
                "altitude classification tolerance must be positive"
            }
            AltitudeCircumstanceSearchError::AltitudeToleranceTooLarge => {
                "altitude classification tolerance exceeds 90 degrees"
            }
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for AltitudeCircumstanceSearchError {}

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

/// Named direction through a caller-selected airless center altitude.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirlessRiseSetKind {
    Rise,
    Set,
}

/// One named projection of an airless altitude crossing.
///
/// This is not a conventional civil sunrise, sunset, moonrise, or moonset.
/// The nested crossing retains the caller-selected physical threshold and all
/// position, transform, and Earth-orientation provenance.
#[derive(Clone, Debug, PartialEq)]
pub struct AirlessRiseSetEvent {
    kind: AirlessRiseSetKind,
    crossing: AirlessAltitudeCrossing,
    naming_model: Model,
}

impl AirlessRiseSetEvent {
    pub fn kind(&self) -> AirlessRiseSetKind {
        self.kind
    }

    pub fn crossing(&self) -> &AirlessAltitudeCrossing {
        &self.crossing
    }

    pub fn naming_model(&self) -> Model {
        self.naming_model
    }
}

/// Direction through the caller-selected conventional horizon circumstance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConventionalRiseSetKind {
    Rise,
    Set,
}

/// One sampled, bracketed conventional rise or set circumstance.
///
/// The event solves `airless center altitude + refraction + upper-limb offset
/// + horizon dip = 0`. The three reported terms and the derived center
/// altitude are evaluated at the interval midpoint, so they explain the
/// selected model but are not angular bounds over the whole TT interval.
#[derive(Clone, Debug, PartialEq)]
pub struct ConventionalRiseSetEvent {
    body: ApparentBody,
    kind: ConventionalRiseSetKind,
    interval: EventInterval,
    observer: Observer,
    airless_center_altitude: Angle,
    refraction_offset: Angle,
    limb_offset: Angle,
    horizon_dip_offset: Angle,
    policy: ConventionalRiseSetPolicy,
    circumstance_model: Model,
    refraction_model: Model,
    limb_model: Model,
    horizon_dip_model: Model,
    provider_model: Model,
    provider_snapshot: Option<String>,
    transform_model: Model,
    earth_orientation_authority: String,
    earth_orientation_snapshot: String,
}

impl ConventionalRiseSetEvent {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn kind(&self) -> ConventionalRiseSetKind {
        self.kind
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn airless_center_altitude(&self) -> Angle {
        self.airless_center_altitude
    }

    pub fn refraction_offset(&self) -> Angle {
        self.refraction_offset
    }

    pub fn limb_offset(&self) -> Angle {
        self.limb_offset
    }

    pub fn horizon_dip_offset(&self) -> Angle {
        self.horizon_dip_offset
    }

    /// The complete selected policy, including physical limb and spherical
    /// horizon radii where those models were selected.
    pub fn policy(&self) -> ConventionalRiseSetPolicy {
        self.policy
    }

    pub fn circumstance_model(&self) -> Model {
        self.circumstance_model
    }

    pub fn refraction_model(&self) -> Model {
        self.refraction_model
    }

    pub fn limb_model(&self) -> Model {
        self.limb_model
    }

    pub fn horizon_dip_model(&self) -> Model {
        self.horizon_dip_model
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

/// Which local meridian the topocentric apparent body crosses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeridianTransitKind {
    Upper,
    Lower,
}

/// One sampled topocentric apparent meridian transit.
///
/// `midpoint_altitude` is an airless estimate at the TT interval midpoint; it
/// is neither a visibility classification nor an angular bound over the whole
/// interval. Upper and lower transits remain valid below the horizon.
#[derive(Clone, Debug, PartialEq)]
pub struct MeridianTransitEvent {
    body: ApparentBody,
    kind: MeridianTransitKind,
    interval: EventInterval,
    midpoint_altitude: Angle,
    observer: Observer,
    transit_model: Model,
    provider_model: Model,
    provider_snapshot: Option<String>,
    transform_model: Model,
    earth_orientation_authority: String,
    earth_orientation_snapshot: String,
}

impl MeridianTransitEvent {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn kind(&self) -> MeridianTransitKind {
        self.kind
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    pub fn midpoint_altitude(&self) -> Angle {
        self.midpoint_altitude
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn transit_model(&self) -> Model {
        self.transit_model
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

/// Direction change at one sampled, bracketed airless altitude extremum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltitudeExtremumKind {
    Maximum,
    Minimum,
}

/// One root of the selected central difference of airless altitude.
///
/// `midpoint_altitude` is an evaluated estimate at the TT interval midpoint,
/// not an angular bound over the complete interval.
#[derive(Clone, Debug, PartialEq)]
pub struct AirlessAltitudeExtremum {
    body: ApparentBody,
    kind: AltitudeExtremumKind,
    interval: EventInterval,
    midpoint_altitude: Angle,
    observer: Observer,
    derivative_span_days: f64,
    provider_model: Model,
    provider_snapshot: Option<String>,
    transform_model: Model,
    earth_orientation_authority: String,
    earth_orientation_snapshot: String,
}

impl AirlessAltitudeExtremum {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn kind(&self) -> AltitudeExtremumKind {
        self.kind
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    pub fn midpoint_altitude(&self) -> Angle {
        self.midpoint_altitude
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn derivative_span_days(&self) -> f64 {
        self.derivative_span_days
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

/// One evaluated airless altitude retained to explain a sampled state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirlessAltitudeSample {
    epoch: JulianDate<TerrestrialTime>,
    altitude: Angle,
}

impl AirlessAltitudeSample {
    pub fn epoch(self) -> JulianDate<TerrestrialTime> {
        self.epoch
    }

    pub fn altitude(self) -> Angle {
        self.altitude
    }
}

/// What the selected samples and refined events establish about a threshold.
///
/// This deliberately avoids `always`, `persistent`, and `circumpolar` names.
/// A generic provider can vary arbitrarily between samples, so finite sampling
/// is not a proof of the continuous state between them.
#[derive(Clone, Debug, PartialEq)]
pub enum AltitudeThresholdState {
    Crosses,
    GrazingCandidate {
        extremum: AirlessAltitudeExtremum,
        offset: Angle,
    },
    AboveAtAllSamples {
        lowest: AirlessAltitudeSample,
    },
    BelowAtAllSamples {
        highest: AirlessAltitudeSample,
    },
    Unresolved {
        closest: AirlessAltitudeSample,
    },
}

/// Crossings, extrema, and sampled threshold state for one bounded TT window.
#[derive(Clone, Debug, PartialEq)]
pub struct AirlessAltitudeCircumstances {
    body: ApparentBody,
    observer: Observer,
    search: AltitudeCircumstanceSearch,
    state: AltitudeThresholdState,
    crossings: Vec<AirlessAltitudeCrossing>,
    extrema: Vec<AirlessAltitudeExtremum>,
    provider_model: Model,
    provider_snapshot: Option<String>,
    transform_model: Model,
    earth_orientation_authority: String,
    earth_orientation_snapshot: String,
}

impl AirlessAltitudeCircumstances {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn search(&self) -> AltitudeCircumstanceSearch {
        self.search
    }

    pub fn state(&self) -> &AltitudeThresholdState {
        &self.state
    }

    pub fn crossings(&self) -> &[AirlessAltitudeCrossing] {
        &self.crossings
    }

    pub fn extrema(&self) -> &[AirlessAltitudeExtremum] {
        &self.extrema
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

/// Error boundary shared by sampled airless altitude extrema.
pub type AltitudeExtremumError<P, E> = AltitudeCrossingError<P, E>;

/// Error boundary shared by bounded airless altitude circumstances.
pub type AltitudeCircumstanceError<P, E> = AltitudeCrossingError<P, E>;

/// Error boundary shared by named airless rise/set events.
pub type AirlessRiseSetError<P, E> = AltitudeCrossingError<P, E>;

/// A failure while searching a caller-composed conventional rise/set event.
#[derive(Debug)]
pub enum ConventionalRiseSetError<P, E> {
    Observation(AltitudeCrossingError<P, E>),
    LimbContainsObserver {
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        physical_radius: Distance,
        topocentric_distance: Distance,
    },
    ObserverBelowHorizonReference {
        epoch: JulianDate<TerrestrialTime>,
        observer_height_meters: f64,
    },
}

impl<P: fmt::Display, E: fmt::Display> fmt::Display for ConventionalRiseSetError<P, E> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConventionalRiseSetError::Observation(ref source) => source.fmt(formatter),
            ConventionalRiseSetError::LimbContainsObserver {
                body,
                epoch,
                physical_radius,
                topocentric_distance,
            } => write!(
                formatter,
                "{} physical limb radius {} km is not smaller than topocentric distance {} km at TT JD {}",
                body.name(),
                physical_radius.kilometers(),
                topocentric_distance.kilometers(),
                epoch.day()
            ),
            ConventionalRiseSetError::ObserverBelowHorizonReference {
                epoch,
                observer_height_meters,
            } => write!(
                formatter,
                "observer height {} m is below the spherical-horizon reference at TT JD {}",
                observer_height_meters,
                epoch.day()
            ),
        }
    }
}

impl<P, E> ::std::error::Error for ConventionalRiseSetError<P, E>
where
    P: ::std::error::Error + 'static,
    E: ::std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            ConventionalRiseSetError::Observation(ref source) => Some(source),
            ConventionalRiseSetError::LimbContainsObserver { .. }
            | ConventionalRiseSetError::ObserverBelowHorizonReference { .. } => None,
        }
    }
}

/// Error boundary shared by topocentric meridian-transit events.
pub type MeridianTransitError<P, E> = AltitudeCrossingError<P, E>;

#[derive(Clone, Copy)]
struct AltitudeSample {
    epoch: JulianDate<TerrestrialTime>,
    altitude: f64,
}

impl AltitudeSample {
    fn signed(self, threshold: Angle) -> f64 {
        self.altitude - threshold.radians()
    }
}

#[derive(Clone, Copy)]
struct ScalarSample {
    epoch: JulianDate<TerrestrialTime>,
    value: f64,
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
    let orientation_authority = earth_orientation.authority().to_owned();
    let orientation_snapshot = earth_orientation.data_snapshot().to_owned();
    let samples = sample_observer_altitudes(
        positions,
        earth_orientation,
        observer,
        body,
        window,
        &orientation_authority,
        &orientation_snapshot,
    )?;
    altitude_crossings_from_samples(
        positions,
        earth_orientation,
        observer,
        body,
        search,
        &samples,
        &orientation_authority,
        &orientation_snapshot,
    )
}

/// Name sampled airless center-altitude crossings as rises or sets.
///
/// `search.threshold()` is caller policy. This does not apply refraction,
/// limb, horizon dip, terrain, obstruction, civil convention, or visibility
/// selection. An empty result has the same limited sampled meaning as
/// [`airless_altitude_crossings`].
pub fn airless_rise_set_events<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: AltitudeCrossingSearch,
) -> Result<Vec<AirlessRiseSetEvent>, AirlessRiseSetError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    airless_altitude_crossings(positions, earth_orientation, observer, body, search).map(
        |crossings| {
            crossings
                .into_iter()
                .map(|crossing| AirlessRiseSetEvent {
                    kind: match crossing.kind() {
                        AltitudeCrossingKind::Ascending => AirlessRiseSetKind::Rise,
                        AltitudeCrossingKind::Descending => AirlessRiseSetKind::Set,
                    },
                    crossing,
                    naming_model: AIRLESS_RISE_SET_NAMING,
                })
                .collect()
        },
    )
}

/// Find conventional rise and set circumstances under an explicit policy.
///
/// At each sample the search evaluates the airless topocentric center altitude
/// plus the selected constant refraction, upper-limb offset, and horizon dip.
/// An ascending zero is `Rise`; a descending zero is `Set`. A physical-radius
/// limb is evaluated from each topocentric range, so its threshold can change
/// across the search. A spherical horizon dip uses the observer's ellipsoid
/// height and the caller-selected radius.
///
/// This is a sampled, sign-changing event search with the same one-hour
/// ceiling and empty-result meaning as [`airless_altitude_crossings`]. It
/// selects neither terrain, obstructions, a civil date, weather inputs, nor a
/// visibility criterion. The refraction value is a constant target lift, not
/// an atmospheric ray-tracing model.
pub fn conventional_rise_set_events<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: ConventionalRiseSetSearch,
    policy: ConventionalRiseSetPolicy,
) -> Result<Vec<ConventionalRiseSetEvent>, ConventionalRiseSetError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let orientation_authority = earth_orientation.authority().to_owned();
    let orientation_snapshot = earth_orientation.data_snapshot().to_owned();
    let mut samples = Vec::new();
    let mut epoch = search.window().start();
    loop {
        samples.push(ScalarSample {
            epoch,
            value: conventional_rise_set_terms(
                positions,
                earth_orientation,
                observer,
                body,
                epoch,
                policy,
                &orientation_authority,
                &orientation_snapshot,
            )?
            .value(),
        });
        if epoch.day() >= search.window().end().day() {
            break;
        }
        let next_day = (epoch.day() + search.window().step_days()).min(search.window().end().day());
        epoch = JulianDate::from_julian_day(next_day)
            .expect("a bounded step between finite epochs remains finite");
    }

    let mut events = Vec::new();
    let mut index = 0;
    while index < samples.len() {
        let left = samples[index];
        if left.value == 0.0 {
            let zero_start = index;
            let mut zero_end = index;
            while zero_end + 1 < samples.len() && samples[zero_end + 1].value == 0.0 {
                zero_end += 1;
            }
            let before = if zero_start > 0 {
                Some(samples[zero_start - 1].value)
            } else {
                None
            };
            let after = if zero_end + 1 < samples.len() {
                Some(samples[zero_end + 1].value)
            } else {
                None
            };
            if zero_start == zero_end {
                if let Some(kind) = exact_crossing_kind(before, after) {
                    push_conventional_rise_set_event(
                        &mut events,
                        positions,
                        earth_orientation,
                        observer,
                        body,
                        conventional_kind(kind),
                        EventInterval {
                            start: left.epoch,
                            end: left.epoch,
                        },
                        policy,
                        &orientation_authority,
                        &orientation_snapshot,
                    )?;
                }
            }
            index = zero_end + 1;
            continue;
        }

        if index + 1 < samples.len() {
            let right = samples[index + 1];
            if right.value != 0.0 && left.value.signum() != right.value.signum() {
                let kind = if right.value > 0.0 {
                    ConventionalRiseSetKind::Rise
                } else {
                    ConventionalRiseSetKind::Set
                };
                let interval =
                    refine_scalar_root(left, right, search.window().tolerance_days(), |epoch| {
                        conventional_rise_set_terms(
                            positions,
                            earth_orientation,
                            observer,
                            body,
                            epoch,
                            policy,
                            &orientation_authority,
                            &orientation_snapshot,
                        )
                        .map(ConventionalRiseSetTerms::value)
                    })?;
                push_conventional_rise_set_event(
                    &mut events,
                    positions,
                    earth_orientation,
                    observer,
                    body,
                    kind,
                    interval,
                    policy,
                    &orientation_authority,
                    &orientation_snapshot,
                )?;
            }
        }
        index += 1;
    }
    Ok(events)
}

#[derive(Clone, Copy)]
struct ConventionalRiseSetTerms {
    airless_center_altitude: Angle,
    refraction: Angle,
    limb: Angle,
    horizon_dip: Angle,
}

impl ConventionalRiseSetTerms {
    fn value(self) -> f64 {
        self.airless_center_altitude.radians()
            + self.refraction.radians()
            + self.limb.radians()
            + self.horizon_dip.radians()
    }
}

fn conventional_kind(kind: AltitudeCrossingKind) -> ConventionalRiseSetKind {
    match kind {
        AltitudeCrossingKind::Ascending => ConventionalRiseSetKind::Rise,
        AltitudeCrossingKind::Descending => ConventionalRiseSetKind::Set,
    }
}

fn conventional_rise_set_terms<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    policy: ConventionalRiseSetPolicy,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<ConventionalRiseSetTerms, ConventionalRiseSetError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let observation = observer_observation(
        positions,
        earth_orientation,
        observer,
        body,
        epoch,
        expected_authority,
        expected_snapshot,
    )
    .map_err(ConventionalRiseSetError::Observation)?;
    let limb = match policy.limb().kind {
        LimbModelKind::Center => Angle::from_radians(0.0).expect("zero is a finite angle"),
        LimbModelKind::UpperAngularRadius(angular_radius) => angular_radius,
        LimbModelKind::UpperPhysicalRadius(physical_radius) => {
            let distance = observation.equatorial().distance();
            if physical_radius >= distance {
                return Err(ConventionalRiseSetError::LimbContainsObserver {
                    body,
                    epoch,
                    physical_radius,
                    topocentric_distance: distance,
                });
            }
            Angle::from_radians((physical_radius.meters() / distance.meters()).asin())
                .expect("a bounded physical-radius ratio produces a finite angle")
        }
    };
    let horizon_dip = match policy.horizon_dip().kind {
        HorizonDipModelKind::Level => Angle::from_radians(0.0).expect("zero is a finite angle"),
        HorizonDipModelKind::Constant(dip) => dip,
        HorizonDipModelKind::Spherical(radius) => {
            let height = observer.height().meters();
            if height < 0.0 {
                return Err(ConventionalRiseSetError::ObserverBelowHorizonReference {
                    epoch,
                    observer_height_meters: height,
                });
            }
            Angle::from_radians((radius.meters() / (radius.meters() + height)).acos()).expect(
                "a nonnegative height over a positive spherical radius produces a finite dip",
            )
        }
    };
    Ok(ConventionalRiseSetTerms {
        airless_center_altitude: observation.horizon().latitude().angle(),
        refraction: policy.refraction().apparent_lift(),
        limb,
        horizon_dip,
    })
}

fn push_conventional_rise_set_event<P, E>(
    events: &mut Vec<ConventionalRiseSetEvent>,
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    kind: ConventionalRiseSetKind,
    interval: EventInterval,
    policy: ConventionalRiseSetPolicy,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<(), ConventionalRiseSetError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let terms = conventional_rise_set_terms(
        positions,
        earth_orientation,
        observer,
        body,
        interval.midpoint(),
        policy,
        expected_authority,
        expected_snapshot,
    )?;
    events.push(ConventionalRiseSetEvent {
        body,
        kind,
        interval,
        observer,
        airless_center_altitude: terms.airless_center_altitude,
        refraction_offset: terms.refraction,
        limb_offset: terms.limb,
        horizon_dip_offset: terms.horizon_dip,
        policy,
        circumstance_model: CONVENTIONAL_RISE_SET_CIRCUMSTANCES,
        refraction_model: policy.refraction().model(),
        limb_model: policy.limb().model(),
        horizon_dip_model: policy.horizon_dip().model(),
        provider_model: positions.model(),
        provider_snapshot: positions.data_snapshot().map(str::to_owned),
        transform_model: AIRLESS_TOPOCENTRIC_TRANSFORM,
        earth_orientation_authority: expected_authority.to_owned(),
        earth_orientation_snapshot: expected_snapshot.to_owned(),
    });
    Ok(())
}

fn sample_observer_altitudes<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    window: SearchWindow,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<Vec<AltitudeSample>, AltitudeCrossingError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let mut samples = Vec::new();
    let mut epoch = window.start();
    loop {
        samples.push(AltitudeSample {
            epoch,
            altitude: observer_altitude(
                positions,
                earth_orientation,
                observer,
                body,
                epoch,
                expected_authority,
                expected_snapshot,
            )?,
        });
        if epoch.day() >= window.end().day() {
            break;
        }
        let next_day = (epoch.day() + window.step_days()).min(window.end().day());
        epoch = JulianDate::from_julian_day(next_day)
            .expect("a bounded step between finite epochs remains finite");
    }
    Ok(samples)
}

fn altitude_crossings_from_samples<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: AltitudeCrossingSearch,
    samples: &[AltitudeSample],
    orientation_authority: &str,
    orientation_snapshot: &str,
) -> Result<Vec<AirlessAltitudeCrossing>, AltitudeCrossingError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let threshold = search.threshold();
    let mut events = Vec::new();
    let mut index = 0;
    while index < samples.len() {
        let left = samples[index];
        let left_signed = left.signed(threshold);
        if left_signed == 0.0 {
            let zero_start = index;
            let mut zero_end = index;
            while zero_end + 1 < samples.len() && samples[zero_end + 1].signed(threshold) == 0.0 {
                zero_end += 1;
            }
            let before = if zero_start > 0 {
                Some(samples[zero_start - 1].signed(threshold))
            } else {
                None
            };
            let after = if zero_end + 1 < samples.len() {
                Some(samples[zero_end + 1].signed(threshold))
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
                    orientation_authority,
                    orientation_snapshot,
                );
            }
            index = zero_end + 1;
            continue;
        }

        if index + 1 < samples.len() {
            let right = samples[index + 1];
            let right_signed = right.signed(threshold);
            if right_signed != 0.0 && left_signed.signum() != right_signed.signum() {
                let kind = if right_signed > 0.0 {
                    AltitudeCrossingKind::Ascending
                } else {
                    AltitudeCrossingKind::Descending
                };
                let interval = refine_scalar_root(
                    ScalarSample {
                        epoch: left.epoch,
                        value: left_signed,
                    },
                    ScalarSample {
                        epoch: right.epoch,
                        value: right_signed,
                    },
                    search.window().tolerance_days(),
                    |epoch| {
                        observer_altitude(
                            positions,
                            earth_orientation,
                            observer,
                            body,
                            epoch,
                            orientation_authority,
                            orientation_snapshot,
                        )
                        .map(|altitude| altitude - threshold.radians())
                    },
                )?;
                push_altitude_crossing(
                    &mut events,
                    body,
                    kind,
                    interval,
                    observer,
                    threshold,
                    positions,
                    orientation_authority,
                    orientation_snapshot,
                );
            }
        }
        index += 1;
    }

    Ok(events)
}

fn observer_altitude<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<f64, AltitudeCrossingError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    Ok(observer_observation(
        positions,
        earth_orientation,
        observer,
        body,
        epoch,
        expected_authority,
        expected_snapshot,
    )?
    .horizon()
    .latitude()
    .radians())
}

fn observer_observation<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<Observation, AltitudeCrossingError<P::Error, E::Error>>
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
    Ok(observation.into_value())
}

fn refine_scalar_root<F, T>(
    mut left: ScalarSample,
    mut right: ScalarSample,
    tolerance_days: f64,
    mut value_at: F,
) -> Result<EventInterval, T>
where
    F: FnMut(JulianDate<TerrestrialTime>) -> Result<f64, T>,
{
    while right.epoch.day() - left.epoch.day() > tolerance_days {
        let epoch = JulianDate::from_julian_day((left.epoch.day() + right.epoch.day()) / 2.0)
            .expect("a finite scalar bracket has a finite midpoint");
        let middle = ScalarSample {
            epoch,
            value: value_at(epoch)?,
        };
        if middle.value == 0.0 {
            return Ok(EventInterval {
                start: epoch,
                end: epoch,
            });
        }
        if left.value.signum() == middle.value.signum() {
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

/// Find sampled upper and lower topocentric meridian transits.
///
/// The root scalar is `cos(declination) * sin(local apparent hour angle)`.
/// It crosses the local meridian without an angular wrap and becomes a zero
/// plateau at the celestial pole rather than fabricating a transit. An upper
/// or lower result can be below the horizon.
///
/// A sign change is refined into a bounded TT interval. An exact root at an
/// interior sample needs opposite signs on either side; at a search boundary,
/// the available one-sided sign is sufficient, matching altitude crossings.
pub fn meridian_transits<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: MeridianTransitSearch,
) -> Result<Vec<MeridianTransitEvent>, MeridianTransitError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let orientation_authority = earth_orientation.authority().to_owned();
    let orientation_snapshot = earth_orientation.data_snapshot().to_owned();
    let window = search.window();
    let mut samples = Vec::new();
    let mut epoch = window.start();
    loop {
        samples.push(ScalarSample {
            epoch,
            value: meridian_components(
                positions,
                earth_orientation,
                observer,
                body,
                epoch,
                &orientation_authority,
                &orientation_snapshot,
            )?
            .0,
        });
        if epoch.day() >= window.end().day() {
            break;
        }
        let next_day = (epoch.day() + window.step_days()).min(window.end().day());
        epoch = JulianDate::from_julian_day(next_day)
            .expect("a bounded meridian-transit step remains finite");
    }

    let mut events = Vec::new();
    let mut index = 0;
    while index < samples.len() {
        let left = samples[index];
        if left.value == 0.0 {
            let zero_start = index;
            let mut zero_end = index;
            while zero_end + 1 < samples.len() && samples[zero_end + 1].value == 0.0 {
                zero_end += 1;
            }
            let before = if zero_start > 0 {
                Some(samples[zero_start - 1].value)
            } else {
                None
            };
            let after = if zero_end + 1 < samples.len() {
                Some(samples[zero_end + 1].value)
            } else {
                None
            };
            if exact_meridian_crossing(zero_end - zero_start + 1, before, after) {
                push_meridian_transit(
                    &mut events,
                    positions,
                    earth_orientation,
                    observer,
                    body,
                    EventInterval {
                        start: left.epoch,
                        end: left.epoch,
                    },
                    &orientation_authority,
                    &orientation_snapshot,
                )?;
            }
            index = zero_end + 1;
            continue;
        }

        if index + 1 < samples.len() {
            let right = samples[index + 1];
            if right.value != 0.0 && left.value.signum() != right.value.signum() {
                let interval = refine_scalar_root(left, right, window.tolerance_days(), |epoch| {
                    meridian_components(
                        positions,
                        earth_orientation,
                        observer,
                        body,
                        epoch,
                        &orientation_authority,
                        &orientation_snapshot,
                    )
                    .map(|components| components.0)
                })?;
                push_meridian_transit(
                    &mut events,
                    positions,
                    earth_orientation,
                    observer,
                    body,
                    interval,
                    &orientation_authority,
                    &orientation_snapshot,
                )?;
            }
        }
        index += 1;
    }

    Ok(events)
}

fn meridian_components<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<(f64, f64), MeridianTransitError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let observation = observer_observation(
        positions,
        earth_orientation,
        observer,
        body,
        epoch,
        expected_authority,
        expected_snapshot,
    )?;
    Ok(meridian_components_from_observation(&observation))
}

fn meridian_components_from_observation(observation: &Observation) -> (f64, f64) {
    let equatorial = observation.equatorial();
    meridian_components_from_angles(
        equatorial.direction().latitude().radians(),
        observation.local_apparent_hour_angle().radians(),
    )
}

fn meridian_components_from_angles(declination: f64, hour_angle: f64) -> (f64, f64) {
    let declination_cosine = if declination.abs() == PI / 2.0 {
        0.0
    } else {
        declination.cos()
    };
    (
        declination_cosine * hour_angle.sin(),
        declination_cosine * hour_angle.cos(),
    )
}

fn exact_meridian_crossing(
    zero_sample_count: usize,
    before: Option<f64>,
    after: Option<f64>,
) -> bool {
    zero_sample_count == 1
        && match (before, after) {
            (Some(left), Some(right)) => left.signum() != right.signum(),
            (None, Some(_)) | (Some(_), None) => true,
            (None, None) => false,
        }
}

fn push_meridian_transit<P, E>(
    events: &mut Vec<MeridianTransitEvent>,
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    interval: EventInterval,
    orientation_authority: &str,
    orientation_snapshot: &str,
) -> Result<(), MeridianTransitError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let observation = observer_observation(
        positions,
        earth_orientation,
        observer,
        body,
        interval.midpoint(),
        orientation_authority,
        orientation_snapshot,
    )?;
    let (_, classifier) = meridian_components_from_observation(&observation);
    let kind = match meridian_transit_kind(classifier) {
        Some(kind) => kind,
        None => return Ok(()),
    };
    let midpoint_altitude = observation.horizon().latitude().angle();
    events.push(MeridianTransitEvent {
        body,
        kind,
        interval,
        midpoint_altitude,
        observer,
        transit_model: TOPOCENTRIC_MERIDIAN_TRANSIT_MODEL,
        provider_model: positions.model(),
        provider_snapshot: positions.data_snapshot().map(str::to_owned),
        transform_model: AIRLESS_TOPOCENTRIC_TRANSFORM,
        earth_orientation_authority: orientation_authority.to_owned(),
        earth_orientation_snapshot: orientation_snapshot.to_owned(),
    });
    Ok(())
}

fn meridian_transit_kind(classifier: f64) -> Option<MeridianTransitKind> {
    if classifier > 0.0 {
        Some(MeridianTransitKind::Upper)
    } else if classifier < 0.0 {
        Some(MeridianTransitKind::Lower)
    } else {
        None
    }
}

/// Find sampled, bracketed roots of the selected airless-altitude derivative.
///
/// Like every numerical event search, this relies on the provider contract's
/// repeatability requirement for duplicate body/epoch requests.
pub fn airless_altitude_extrema<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: AltitudeExtremumSearch,
) -> Result<Vec<AirlessAltitudeExtremum>, AltitudeExtremumError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let orientation_authority = earth_orientation.authority().to_owned();
    let orientation_snapshot = earth_orientation.data_snapshot().to_owned();
    find_altitude_extrema(
        positions,
        earth_orientation,
        observer,
        body,
        search,
        &orientation_authority,
        &orientation_snapshot,
    )
}

fn find_altitude_extrema<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: AltitudeExtremumSearch,
    orientation_authority: &str,
    orientation_snapshot: &str,
) -> Result<Vec<AirlessAltitudeExtremum>, AltitudeExtremumError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let window = search.window();
    let mut samples = Vec::new();
    let mut epoch = window.start();
    loop {
        samples.push(ScalarSample {
            epoch,
            value: altitude_central_difference(
                positions,
                earth_orientation,
                observer,
                body,
                epoch,
                search.derivative_span_days(),
                orientation_authority,
                orientation_snapshot,
            )?,
        });
        if epoch.day() >= window.end().day() {
            break;
        }
        let next_day = (epoch.day() + window.step_days()).min(window.end().day());
        epoch =
            JulianDate::from_julian_day(next_day).expect("a bounded extremum step remains finite");
    }

    let mut extrema = Vec::new();
    let mut index = 0;
    while index < samples.len() {
        let left = samples[index];
        if left.value == 0.0 {
            let zero_start = index;
            let mut zero_end = index;
            while zero_end + 1 < samples.len() && samples[zero_end + 1].value == 0.0 {
                zero_end += 1;
            }
            let before = if zero_start > 0 {
                Some(samples[zero_start - 1].value)
            } else {
                None
            };
            let after = if zero_end + 1 < samples.len() {
                Some(samples[zero_end + 1].value)
            } else {
                None
            };
            if zero_start == zero_end {
                if let Some(kind) = exact_extremum_kind(before, after) {
                    push_altitude_extremum(
                        &mut extrema,
                        positions,
                        earth_orientation,
                        observer,
                        body,
                        kind,
                        EventInterval {
                            start: left.epoch,
                            end: left.epoch,
                        },
                        search.derivative_span_days(),
                        orientation_authority,
                        orientation_snapshot,
                    )?;
                }
            }
            index = zero_end + 1;
            continue;
        }

        if index + 1 < samples.len() {
            let right = samples[index + 1];
            if right.value != 0.0 && left.value.signum() != right.value.signum() {
                let kind = if right.value < 0.0 {
                    AltitudeExtremumKind::Maximum
                } else {
                    AltitudeExtremumKind::Minimum
                };
                let interval = refine_scalar_root(left, right, window.tolerance_days(), |epoch| {
                    altitude_central_difference(
                        positions,
                        earth_orientation,
                        observer,
                        body,
                        epoch,
                        search.derivative_span_days(),
                        orientation_authority,
                        orientation_snapshot,
                    )
                })?;
                let duplicate = extrema
                    .last()
                    .map(|event: &AirlessAltitudeExtremum| {
                        event.interval().end().day() >= interval.start().day()
                    })
                    .unwrap_or(false);
                if !duplicate {
                    push_altitude_extremum(
                        &mut extrema,
                        positions,
                        earth_orientation,
                        observer,
                        body,
                        kind,
                        interval,
                        search.derivative_span_days(),
                        orientation_authority,
                        orientation_snapshot,
                    )?;
                }
            }
        }
        index += 1;
    }
    Ok(extrema)
}

fn altitude_central_difference<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    span_days: f64,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<f64, AltitudeExtremumError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let half_span = span_days / 2.0;
    let before = epoch
        .offset_days(-half_span)
        .expect("a finite derivative sample has a finite earlier epoch");
    let after = epoch
        .offset_days(half_span)
        .expect("a finite derivative sample has a finite later epoch");
    let before_altitude = observer_altitude(
        positions,
        earth_orientation,
        observer,
        body,
        before,
        expected_authority,
        expected_snapshot,
    )?;
    let after_altitude = observer_altitude(
        positions,
        earth_orientation,
        observer,
        body,
        after,
        expected_authority,
        expected_snapshot,
    )?;
    Ok(after_altitude - before_altitude)
}

fn exact_extremum_kind(before: Option<f64>, after: Option<f64>) -> Option<AltitudeExtremumKind> {
    match (before, after) {
        (Some(left), Some(right)) if left > 0.0 && right < 0.0 => {
            Some(AltitudeExtremumKind::Maximum)
        }
        (Some(left), Some(right)) if left < 0.0 && right > 0.0 => {
            Some(AltitudeExtremumKind::Minimum)
        }
        _ => None,
    }
}

fn push_altitude_extremum<P, E>(
    extrema: &mut Vec<AirlessAltitudeExtremum>,
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    kind: AltitudeExtremumKind,
    interval: EventInterval,
    derivative_span_days: f64,
    orientation_authority: &str,
    orientation_snapshot: &str,
) -> Result<(), AltitudeExtremumError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let midpoint_altitude = observer_altitude(
        positions,
        earth_orientation,
        observer,
        body,
        interval.midpoint(),
        orientation_authority,
        orientation_snapshot,
    )?;
    extrema.push(AirlessAltitudeExtremum {
        body,
        kind,
        interval,
        midpoint_altitude: Angle::from_radians(midpoint_altitude)
            .expect("observer altitude is finite"),
        observer,
        derivative_span_days,
        provider_model: positions.model(),
        provider_snapshot: positions.data_snapshot().map(str::to_owned),
        transform_model: AIRLESS_TOPOCENTRIC_TRANSFORM,
        earth_orientation_authority: orientation_authority.to_owned(),
        earth_orientation_snapshot: orientation_snapshot.to_owned(),
    });
    Ok(())
}

/// Resolve sampled altitude crossings, extrema, and threshold state together.
///
/// Crossing samples are reused for classification. Extrema refinement makes
/// additional repeatable provider requests, including midpoint-altitude
/// evaluation, under the same provider identities.
pub fn airless_altitude_circumstances<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    body: ApparentBody,
    search: AltitudeCircumstanceSearch,
) -> Result<AirlessAltitudeCircumstances, AltitudeCircumstanceError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let orientation_authority = earth_orientation.authority().to_owned();
    let orientation_snapshot = earth_orientation.data_snapshot().to_owned();
    let samples = sample_observer_altitudes(
        positions,
        earth_orientation,
        observer,
        body,
        search.extrema().window(),
        &orientation_authority,
        &orientation_snapshot,
    )?;
    let crossings = altitude_crossings_from_samples(
        positions,
        earth_orientation,
        observer,
        body,
        search.crossing(),
        &samples,
        &orientation_authority,
        &orientation_snapshot,
    )?;
    let extrema = find_altitude_extrema(
        positions,
        earth_orientation,
        observer,
        body,
        search.extrema(),
        &orientation_authority,
        &orientation_snapshot,
    )?;
    let state = classify_altitude_threshold(
        &samples,
        &extrema,
        &crossings,
        search.threshold(),
        search.altitude_tolerance(),
    );
    Ok(AirlessAltitudeCircumstances {
        body,
        observer,
        search,
        state,
        crossings,
        extrema,
        provider_model: positions.model(),
        provider_snapshot: positions.data_snapshot().map(str::to_owned),
        transform_model: AIRLESS_TOPOCENTRIC_TRANSFORM,
        earth_orientation_authority: orientation_authority,
        earth_orientation_snapshot: orientation_snapshot,
    })
}

fn classify_altitude_threshold(
    samples: &[AltitudeSample],
    extrema: &[AirlessAltitudeExtremum],
    crossings: &[AirlessAltitudeCrossing],
    threshold: Angle,
    tolerance: Angle,
) -> AltitudeThresholdState {
    if !crossings.is_empty() {
        return AltitudeThresholdState::Crosses;
    }

    let grazing = extrema
        .iter()
        .map(|extremum| {
            let offset = extremum.midpoint_altitude().radians() - threshold.radians();
            (extremum, offset)
        })
        .filter(|&(_, offset)| offset.abs() <= tolerance.radians())
        .min_by(|&(_, left), &(_, right)| {
            left.abs()
                .partial_cmp(&right.abs())
                .expect("finite altitude offsets are ordered")
        });
    if let Some((extremum, offset)) = grazing {
        return AltitudeThresholdState::GrazingCandidate {
            extremum: extremum.clone(),
            offset: Angle::from_radians(offset).expect("finite altitude offset"),
        };
    }

    let mut evaluated: Vec<AirlessAltitudeSample> = samples
        .iter()
        .map(|sample| AirlessAltitudeSample {
            epoch: sample.epoch,
            altitude: Angle::from_radians(sample.altitude).expect("finite observer altitude"),
        })
        .collect();
    evaluated.extend(extrema.iter().map(|extremum| AirlessAltitudeSample {
        epoch: extremum.interval().midpoint(),
        altitude: extremum.midpoint_altitude(),
    }));
    let lowest = *evaluated
        .iter()
        .min_by(|left, right| {
            left.altitude
                .radians()
                .partial_cmp(&right.altitude.radians())
                .expect("finite altitudes are ordered")
        })
        .expect("a validated search samples both endpoints");
    let highest = *evaluated
        .iter()
        .max_by(|left, right| {
            left.altitude
                .radians()
                .partial_cmp(&right.altitude.radians())
                .expect("finite altitudes are ordered")
        })
        .expect("a validated search samples both endpoints");
    if lowest.altitude().radians() - threshold.radians() > tolerance.radians() {
        return AltitudeThresholdState::AboveAtAllSamples { lowest };
    }
    if highest.altitude().radians() - threshold.radians() < -tolerance.radians() {
        return AltitudeThresholdState::BelowAtAllSamples { highest };
    }
    let closest = *evaluated
        .iter()
        .min_by(|left, right| {
            let left_offset = (left.altitude.radians() - threshold.radians()).abs();
            let right_offset = (right.altitude.radians() - threshold.radians()).abs();
            left_offset
                .partial_cmp(&right_offset)
                .expect("finite altitude offsets are ordered")
        })
        .expect("a validated search samples both endpoints");
    AltitudeThresholdState::Unresolved { closest }
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
    use super::{
        classify_altitude_threshold, exact_crossing_kind, exact_extremum_kind,
        exact_meridian_crossing, meridian_components_from_angles, meridian_transit_kind,
        AltitudeCrossingKind, AltitudeExtremumKind, AltitudeSample, AltitudeThresholdState, Angle,
        JulianDate, MeridianTransitKind, TerrestrialTime,
    };
    use std::f64::consts::PI;

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

    #[test]
    fn exact_derivative_zero_requires_two_sided_motion_reversal() {
        assert_eq!(
            exact_extremum_kind(Some(1.0), Some(-1.0)),
            Some(AltitudeExtremumKind::Maximum)
        );
        assert_eq!(
            exact_extremum_kind(Some(-1.0), Some(1.0)),
            Some(AltitudeExtremumKind::Minimum)
        );
        assert_eq!(exact_extremum_kind(Some(1.0), Some(1.0)), None);
        assert_eq!(exact_extremum_kind(None, Some(-1.0)), None);
        assert_eq!(exact_extremum_kind(Some(1.0), None), None);
    }

    #[test]
    fn near_threshold_sample_without_an_extremum_is_unresolved() {
        let epoch = JulianDate::<TerrestrialTime>::from_julian_day(2_460_000.0).unwrap();
        let samples = [AltitudeSample {
            epoch,
            altitude: 0.001,
        }];
        let state = classify_altitude_threshold(
            &samples,
            &[],
            &[],
            Angle::from_radians(0.0).unwrap(),
            Angle::from_radians(0.01).unwrap(),
        );
        assert!(matches!(state, AltitudeThresholdState::Unresolved { .. }));
    }

    #[test]
    fn meridian_exact_root_requires_a_single_zero_sample_and_a_crossing() {
        assert!(exact_meridian_crossing(1, Some(-1.0), Some(1.0)));
        assert!(exact_meridian_crossing(1, None, Some(1.0)));
        assert!(!exact_meridian_crossing(1, Some(1.0), Some(1.0)));
        assert!(!exact_meridian_crossing(2, Some(-1.0), Some(1.0)));
    }

    #[test]
    fn meridian_pole_is_a_zero_plateau_not_a_transit() {
        assert_eq!(meridian_components_from_angles(PI / 2.0, -1.0), (0.0, 0.0));
        assert_eq!(meridian_components_from_angles(-PI / 2.0, 1.0), (0.0, 0.0));
        assert_eq!(meridian_transit_kind(0.0), None);
        assert_eq!(meridian_transit_kind(1.0), Some(MeridianTransitKind::Upper));
        assert_eq!(
            meridian_transit_kind(-1.0),
            Some(MeridianTransitKind::Lower)
        );
    }
}
