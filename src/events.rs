// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Provider-neutral celestial event searches.
//!
//! T4 event searches currently cover apparent ecliptic-longitude conjunctions
//! and stationary points. Every result is a bounded TT interval, not an
//! isolated floating-point instant.

use std::f64::consts::PI;
use std::fmt;

use apparent::ApparentBody;
use foundation::{
    Angle, JulianDate, Longitude, Model, State, TerrestrialTime, TrueEclipticEquinoxOfDate,
};
use provider::GeocentricPositionProvider;

const TWO_PI: f64 = 2.0 * PI;

/// Maximum sampling step accepted by the current event searches.
///
/// One TT day keeps even the Moon's relative motion safely below a half-turn,
/// so an opposition wrap cannot masquerade as a conjunction. Station callers
/// can select a smaller step when resolving faster changes in longitude speed.
pub const MAX_EVENT_STEP_DAYS: f64 = 1.0;

/// Backward-compatible name for the shared event sampling ceiling.
pub const MAX_CONJUNCTION_STEP_DAYS: f64 = MAX_EVENT_STEP_DAYS;

/// Maximum full interval accepted for a station's central-difference speed.
pub const MAX_STATION_VELOCITY_SPAN_DAYS: f64 = 1.0;

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

#[derive(Debug)]
pub enum EventError<E> {
    SameBody,
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
            EventError::SameBody => None,
        }
    }
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
            let interval = refine_conjunction(
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

fn refine_conjunction<P>(
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
