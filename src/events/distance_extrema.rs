// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Sampled, bracketed extrema of a provider's geocentric apparent range.
//!
//! This module works only with the apparent Earth-body distance in a
//! [`GeocentricPositionProvider`]'s typed state. It does not establish an
//! orbital, barycentric, topocentric, or visibility distance.

use std::fmt;

use apparent::ApparentBody;
use foundation::{Distance, JulianDate, Model, TerrestrialTime};
use provider::GeocentricPositionProvider;

use super::{EventInterval, SearchWindow};

/// Revisioned model for sampled extrema of apparent geocentric range.
pub const GEOCENTRIC_APPARENT_DISTANCE_EXTREMA: Model = Model::new(
    "sampled central-difference geocentric apparent range extrema",
    "1",
);

/// Largest full central-difference interval accepted for a distance extremum.
pub const MAX_GEOCENTRIC_DISTANCE_EXTREMUM_DERIVATIVE_SPAN_DAYS: f64 = 1.0;

/// Numerical controls for sampled geocentric apparent-range extrema.
///
/// `derivative_span_days` is the full interval between the two position
/// samples used at each derivative evaluation. Requests may therefore extend
/// half that span beyond either end of the contained search window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeocentricDistanceExtremumSearch {
    window: SearchWindow,
    derivative_span_days: f64,
}

impl GeocentricDistanceExtremumSearch {
    pub fn new(
        window: SearchWindow,
        derivative_span_days: f64,
    ) -> Result<Self, GeocentricDistanceExtremumSearchError> {
        if !derivative_span_days.is_finite() {
            return Err(GeocentricDistanceExtremumSearchError::DerivativeSpanNotFinite);
        }
        if derivative_span_days <= 0.0 {
            return Err(GeocentricDistanceExtremumSearchError::DerivativeSpanNotPositive);
        }
        if derivative_span_days > MAX_GEOCENTRIC_DISTANCE_EXTREMUM_DERIVATIVE_SPAN_DAYS {
            return Err(GeocentricDistanceExtremumSearchError::DerivativeSpanTooLarge);
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
pub enum GeocentricDistanceExtremumSearchError {
    DerivativeSpanNotFinite,
    DerivativeSpanNotPositive,
    DerivativeSpanTooLarge,
}

impl fmt::Display for GeocentricDistanceExtremumSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            GeocentricDistanceExtremumSearchError::DerivativeSpanNotFinite => {
                "geocentric distance derivative span must be finite"
            }
            GeocentricDistanceExtremumSearchError::DerivativeSpanNotPositive => {
                "geocentric distance derivative span must be positive"
            }
            GeocentricDistanceExtremumSearchError::DerivativeSpanTooLarge => {
                "geocentric distance derivative span exceeds one TT day"
            }
        };
        formatter.write_str(message)
    }
}

impl ::std::error::Error for GeocentricDistanceExtremumSearchError {}

/// Direction of a sampled geocentric apparent-range reversal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeocentricDistanceExtremumKind {
    Minimum,
    Maximum,
}

/// One bounded TT interval containing a sampled geocentric range extremum.
#[derive(Clone, Debug, PartialEq)]
pub struct GeocentricDistanceExtremum {
    body: ApparentBody,
    kind: GeocentricDistanceExtremumKind,
    interval: EventInterval,
    midpoint_distance: Distance,
    derivative_span_days: f64,
    extrema_model: Model,
    provider_model: Model,
    provider_snapshot: Option<String>,
}

impl GeocentricDistanceExtremum {
    pub fn body(&self) -> ApparentBody {
        self.body
    }

    pub fn kind(&self) -> GeocentricDistanceExtremumKind {
        self.kind
    }

    pub fn interval(&self) -> EventInterval {
        self.interval
    }

    /// Evaluated geocentric apparent range at the interval midpoint.
    ///
    /// This is not a bound on the continuous extremum distance.
    pub fn midpoint_distance(&self) -> Distance {
        self.midpoint_distance
    }

    pub fn derivative_span_days(&self) -> f64 {
        self.derivative_span_days
    }

    pub fn extrema_model(&self) -> Model {
        self.extrema_model
    }

    pub fn provider_model(&self) -> Model {
        self.provider_model
    }

    pub fn provider_snapshot(&self) -> Option<&str> {
        self.provider_snapshot.as_ref().map(String::as_str)
    }
}

/// A failure while finding a geocentric apparent-range extremum.
#[derive(Debug)]
pub enum GeocentricDistanceExtremumError<P> {
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
}

impl<P: fmt::Display> fmt::Display for GeocentricDistanceExtremumError<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            GeocentricDistanceExtremumError::Position {
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
            GeocentricDistanceExtremumError::StateEpochMismatch {
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
        }
    }
}

impl<P> ::std::error::Error for GeocentricDistanceExtremumError<P>
where
    P: ::std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            GeocentricDistanceExtremumError::Position { ref source, .. } => Some(source),
            GeocentricDistanceExtremumError::StateEpochMismatch { .. } => None,
        }
    }
}

#[derive(Clone, Copy)]
struct DistanceSample {
    epoch: JulianDate<TerrestrialTime>,
    difference: f64,
}

/// Find sampled, bracketed extrema of one body's geocentric apparent range.
///
/// A result is a sign reversal of the caller-selected central range difference,
/// not proof that the full window contains no unsampled extrema. An empty result
/// establishes only that no sampled, bracketed reversal was found.
pub fn geocentric_distance_extrema<P>(
    positions: &P,
    body: ApparentBody,
    search: GeocentricDistanceExtremumSearch,
) -> Result<Vec<GeocentricDistanceExtremum>, GeocentricDistanceExtremumError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let window = search.window();
    let mut samples = Vec::new();
    let mut epoch = window.start();
    loop {
        samples.push(DistanceSample {
            epoch,
            difference: central_distance_difference(
                positions,
                body,
                epoch,
                search.derivative_span_days(),
            )?,
        });
        if epoch.day() >= window.end().day() {
            break;
        }
        let next_day = (epoch.day() + window.step_days()).min(window.end().day());
        epoch = JulianDate::from_julian_day(next_day)
            .expect("a bounded distance-extremum step remains finite");
    }

    let mut extrema = Vec::new();
    let mut index = 0;
    while index < samples.len() {
        let left = samples[index];
        if left.difference == 0.0 {
            let zero_start = index;
            let mut zero_end = index;
            while zero_end + 1 < samples.len() && samples[zero_end + 1].difference == 0.0 {
                zero_end += 1;
            }
            if zero_start == zero_end {
                if zero_start > 0 && zero_end + 1 < samples.len() {
                    let before = samples[zero_start - 1];
                    let after = samples[zero_end + 1];
                    if let Some(kind) =
                        exact_extremum_kind(Some(before.difference), Some(after.difference))
                    {
                        if isolated_exact_extremum(
                            positions,
                            body,
                            before,
                            left.epoch,
                            after,
                            search.derivative_span_days(),
                        )? {
                            push_extremum(
                                &mut extrema,
                                positions,
                                body,
                                kind,
                                EventInterval {
                                    start: left.epoch,
                                    end: left.epoch,
                                },
                                search.derivative_span_days(),
                            )?;
                        }
                    }
                }
            }
            index = zero_end + 1;
            continue;
        }

        if index + 1 < samples.len() {
            let right = samples[index + 1];
            if right.difference != 0.0 && left.difference.signum() != right.difference.signum() {
                let kind = if right.difference > 0.0 {
                    GeocentricDistanceExtremumKind::Minimum
                } else {
                    GeocentricDistanceExtremumKind::Maximum
                };
                if let Some(interval) = refine_difference_root(
                    positions,
                    body,
                    left,
                    right,
                    search.derivative_span_days(),
                    window.tolerance_days(),
                )? {
                    let duplicate = extrema
                        .last()
                        .map(|event: &GeocentricDistanceExtremum| {
                            event.interval.end().day() >= interval.start().day()
                        })
                        .unwrap_or(false);
                    if !duplicate {
                        push_extremum(
                            &mut extrema,
                            positions,
                            body,
                            kind,
                            interval,
                            search.derivative_span_days(),
                        )?;
                    }
                }
            }
        }
        index += 1;
    }
    Ok(extrema)
}

fn central_distance_difference<P>(
    positions: &P,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    derivative_span_days: f64,
) -> Result<f64, GeocentricDistanceExtremumError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let half_span = derivative_span_days / 2.0;
    let before = epoch
        .offset_days(-half_span)
        .expect("a finite central-difference sample has a finite earlier epoch");
    let after = epoch
        .offset_days(half_span)
        .expect("a finite central-difference sample has a finite later epoch");
    Ok(distance_at(positions, body, after)?.meters()
        - distance_at(positions, body, before)?.meters())
}

fn refine_difference_root<P>(
    positions: &P,
    body: ApparentBody,
    mut left: DistanceSample,
    mut right: DistanceSample,
    derivative_span_days: f64,
    tolerance_days: f64,
) -> Result<Option<EventInterval>, GeocentricDistanceExtremumError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    while right.epoch.day() - left.epoch.day() > tolerance_days {
        let epoch = JulianDate::from_julian_day((left.epoch.day() + right.epoch.day()) / 2.0)
            .expect("a finite distance-extremum bracket has a finite midpoint");
        let middle = DistanceSample {
            epoch,
            difference: central_distance_difference(positions, body, epoch, derivative_span_days)?,
        };
        if middle.difference == 0.0 {
            return Ok(
                if isolated_exact_extremum(
                    positions,
                    body,
                    left,
                    epoch,
                    right,
                    derivative_span_days,
                )? {
                    Some(EventInterval {
                        start: epoch,
                        end: epoch,
                    })
                } else {
                    None
                },
            );
        }
        if left.difference.signum() == middle.difference.signum() {
            left = middle;
        } else {
            right = middle;
        }
    }
    Ok(Some(EventInterval {
        start: left.epoch,
        end: right.epoch,
    }))
}

fn isolated_exact_extremum<P>(
    positions: &P,
    body: ApparentBody,
    before: DistanceSample,
    zero_epoch: JulianDate<TerrestrialTime>,
    after: DistanceSample,
    derivative_span_days: f64,
) -> Result<bool, GeocentricDistanceExtremumError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let expected = match exact_extremum_kind(Some(before.difference), Some(after.difference)) {
        Some(kind) => kind,
        None => return Ok(false),
    };
    let before_probe = central_distance_difference(
        positions,
        body,
        EventInterval {
            start: before.epoch,
            end: zero_epoch,
        }
        .midpoint(),
        derivative_span_days,
    )?;
    let after_probe = central_distance_difference(
        positions,
        body,
        EventInterval {
            start: zero_epoch,
            end: after.epoch,
        }
        .midpoint(),
        derivative_span_days,
    )?;
    Ok(exact_extremum_kind(Some(before_probe), Some(after_probe)) == Some(expected))
}

fn exact_extremum_kind(
    before: Option<f64>,
    after: Option<f64>,
) -> Option<GeocentricDistanceExtremumKind> {
    match (before, after) {
        (Some(left), Some(right)) if left < 0.0 && right > 0.0 => {
            Some(GeocentricDistanceExtremumKind::Minimum)
        }
        (Some(left), Some(right)) if left > 0.0 && right < 0.0 => {
            Some(GeocentricDistanceExtremumKind::Maximum)
        }
        _ => None,
    }
}

fn push_extremum<P>(
    extrema: &mut Vec<GeocentricDistanceExtremum>,
    positions: &P,
    body: ApparentBody,
    kind: GeocentricDistanceExtremumKind,
    interval: EventInterval,
    derivative_span_days: f64,
) -> Result<(), GeocentricDistanceExtremumError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let midpoint_distance = distance_at(positions, body, interval.midpoint())?;
    extrema.push(GeocentricDistanceExtremum {
        body,
        kind,
        interval,
        midpoint_distance,
        derivative_span_days,
        extrema_model: GEOCENTRIC_APPARENT_DISTANCE_EXTREMA,
        provider_model: positions.model(),
        provider_snapshot: positions.data_snapshot().map(str::to_owned),
    });
    Ok(())
}

fn distance_at<P>(
    positions: &P,
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<Distance, GeocentricDistanceExtremumError<P::Error>>
where
    P: GeocentricPositionProvider,
{
    let state = positions.position(body, epoch).map_err(|source| {
        GeocentricDistanceExtremumError::Position {
            body,
            epoch,
            source,
        }
    })?;
    if state.epoch() != epoch {
        return Err(GeocentricDistanceExtremumError::StateEpochMismatch {
            body,
            expected_epoch: epoch,
            actual_epoch: state.epoch(),
        });
    }
    Ok(state.distance())
}
