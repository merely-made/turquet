// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Typed values shared by Turquet's primary calculation APIs.
//!
//! The types in this module make units, time scales, and reference frames
//! part of a function signature. Raw inherited calculations live under
//! [`compat`](crate::compat); new APIs should not accept an anonymous `f64`
//! where one of these types can state the contract.

use std::f64::consts::PI;
use std::marker::PhantomData;

pub use hifitime::Epoch as ScaleAwareEpoch;

const AU_METERS: f64 = 149_597_870_700.0;
const J2000_JD: f64 = 2_451_545.0;

/// A rejected physical or temporal value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueError {
    /// A value was NaN or infinite.
    NotFinite(&'static str),
    /// A value fell outside the stated inclusive range.
    OutOfRange {
        field: &'static str,
        value: f64,
        minimum: f64,
        maximum: f64,
    },
    /// A distance or error bound was negative.
    Negative(&'static str),
    /// A vector had zero length.
    ZeroVector,
}

/// Marker for Terrestrial Time (TT).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrestrialTime {}

/// Marker for Universal Time 1 (UT1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UniversalTime1 {}

/// A two-part Julian Date whose time scale is carried by `Scale`.
///
/// Two parts retain more precision than forcing every caller through one
/// large day number. Turquet generally uses J2000.0 as the first part.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JulianDate<Scale> {
    day1: f64,
    day2: f64,
    scale: PhantomData<Scale>,
}

impl<Scale> JulianDate<Scale> {
    /// Construct a Julian Date on the scale named by the type parameter.
    ///
    /// The caller is asserting the scale. Prefer a scale-aware conversion
    /// such as [`JulianDate<TerrestrialTime>::from_epoch`] when possible.
    pub fn from_parts(day1: f64, day2: f64) -> Result<Self, ValueError> {
        require_finite("Julian Date first part", day1)?;
        require_finite("Julian Date second part", day2)?;
        Ok(Self {
            day1,
            day2,
            scale: PhantomData,
        })
    }

    /// Construct from one Julian day value on the named scale.
    pub fn from_julian_day(day: f64) -> Result<Self, ValueError> {
        require_finite("Julian Date", day)?;
        Self::from_parts(J2000_JD, day - J2000_JD)
    }

    /// The two parts exactly as supplied.
    pub fn parts(self) -> (f64, f64) {
        (self.day1, self.day2)
    }

    /// The summed Julian day value.
    pub fn day(self) -> f64 {
        self.day1 + self.day2
    }

    /// Return an epoch offset by a duration expressed in SI days.
    pub fn offset_days(self, days: f64) -> Result<Self, ValueError> {
        require_finite("day offset", days)?;
        Self::from_parts(self.day1, self.day2 + days)
    }
}

impl JulianDate<TerrestrialTime> {
    /// Convert any scale-aware hifitime epoch to a TT Julian Date.
    pub fn from_epoch(epoch: ScaleAwareEpoch) -> Self {
        let day = epoch.to_jde_tt_days();
        Self::from_parts(J2000_JD, day - J2000_JD)
            .expect("hifitime epochs produce finite TT Julian Dates")
    }
}

/// A finite plane angle in radians.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Angle(f64);

impl Angle {
    pub fn from_radians(radians: f64) -> Result<Self, ValueError> {
        require_finite("angle", radians)?;
        Ok(Self(radians))
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, ValueError> {
        require_finite("angle", degrees)?;
        Self::from_radians(degrees.to_radians())
    }

    pub fn from_arcseconds(arcseconds: f64) -> Result<Self, ValueError> {
        require_finite("angle", arcseconds)?;
        Self::from_degrees(arcseconds / 3_600.0)
    }

    pub fn radians(self) -> f64 {
        self.0
    }

    pub fn degrees(self) -> f64 {
        self.0.to_degrees()
    }

    pub fn arcseconds(self) -> f64 {
        self.degrees() * 3_600.0
    }

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }
}

/// A longitude normalized to `[0, 2pi)`, positive toward the frame's east.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Longitude(Angle);

impl Longitude {
    pub fn from_radians(radians: f64) -> Result<Self, ValueError> {
        let angle = Angle::from_radians(radians)?;
        Ok(Self(Angle(angle.radians().rem_euclid(2.0 * PI))))
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, ValueError> {
        let angle = Angle::from_degrees(degrees)?;
        Self::from_radians(angle.radians())
    }

    pub fn angle(self) -> Angle {
        self.0
    }

    pub fn radians(self) -> f64 {
        self.0.radians()
    }

    pub fn degrees(self) -> f64 {
        self.0.degrees()
    }
}

/// A geographic longitude normalized to `[-pi, pi)`, positive east of the
/// reference meridian and negative west.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EastLongitude(Angle);

impl EastLongitude {
    pub fn from_radians(radians: f64) -> Result<Self, ValueError> {
        let angle = Angle::from_radians(radians)?;
        let normalized = (angle.radians() + PI).rem_euclid(2.0 * PI) - PI;
        Ok(Self(Angle(normalized)))
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, ValueError> {
        let angle = Angle::from_degrees(degrees)?;
        Self::from_radians(angle.radians())
    }

    pub fn angle(self) -> Angle {
        self.0
    }

    pub fn radians(self) -> f64 {
        self.0.radians()
    }

    pub fn degrees(self) -> f64 {
        self.0.degrees()
    }
}

/// A latitude in the closed interval `[-pi/2, pi/2]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Latitude(Angle);

impl Latitude {
    pub fn from_radians(radians: f64) -> Result<Self, ValueError> {
        let angle = Angle::from_radians(radians)?;
        if !(-PI / 2.0..=PI / 2.0).contains(&radians) {
            return Err(ValueError::OutOfRange {
                field: "latitude",
                value: radians,
                minimum: -PI / 2.0,
                maximum: PI / 2.0,
            });
        }
        Ok(Self(angle))
    }

    pub fn from_degrees(degrees: f64) -> Result<Self, ValueError> {
        let angle = Angle::from_degrees(degrees)?;
        Self::from_radians(angle.radians())
    }

    pub fn angle(self) -> Angle {
        self.0
    }

    pub fn radians(self) -> f64 {
        self.0.radians()
    }

    pub fn degrees(self) -> f64 {
        self.0.degrees()
    }
}

/// A finite signed length, stored in metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Length(f64);

impl Length {
    pub fn from_meters(meters: f64) -> Result<Self, ValueError> {
        require_finite("length", meters)?;
        Ok(Self(meters))
    }

    pub fn meters(self) -> f64 {
        self.0
    }

    pub fn kilometers(self) -> f64 {
        self.0 / 1_000.0
    }
}

/// A finite nonnegative distance, stored in metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Distance(Length);

impl Distance {
    pub fn from_meters(meters: f64) -> Result<Self, ValueError> {
        if meters < 0.0 {
            return Err(ValueError::Negative("distance"));
        }
        Ok(Self(Length::from_meters(meters)?))
    }

    pub fn from_kilometers(kilometers: f64) -> Result<Self, ValueError> {
        require_finite("distance", kilometers)?;
        Self::from_meters(kilometers * 1_000.0)
    }

    pub fn from_astronomical_units(astronomical_units: f64) -> Result<Self, ValueError> {
        require_finite("distance", astronomical_units)?;
        Self::from_meters(astronomical_units * AU_METERS)
    }

    pub fn meters(self) -> f64 {
        self.0.meters()
    }

    pub fn kilometers(self) -> f64 {
        self.0.kilometers()
    }

    pub fn astronomical_units(self) -> f64 {
        self.meters() / AU_METERS
    }
}

/// A geodetic observer. Longitude is east-positive and height is relative to
/// the reference ellipsoid selected by the consuming model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observer {
    longitude: EastLongitude,
    latitude: Latitude,
    height: Length,
}

impl Observer {
    pub fn new(longitude: EastLongitude, latitude: Latitude, height: Length) -> Self {
        Self {
            longitude,
            latitude,
            height,
        }
    }

    pub fn longitude(self) -> EastLongitude {
        self.longitude
    }

    pub fn latitude(self) -> Latitude {
        self.latitude
    }

    pub fn height(self) -> Length {
        self.height
    }
}

/// Geocentric Celestial Reference System.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gcrs {}

/// True equator and equinox of the observation date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrueEquatorEquinoxOfDate {}

/// True ecliptic and equinox of the observation date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TrueEclipticEquinoxOfDate {}

/// A spherical direction in a statically named reference frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Direction<Frame> {
    longitude: Longitude,
    latitude: Latitude,
    frame: PhantomData<Frame>,
}

impl<Frame> Direction<Frame> {
    pub fn new(longitude: Longitude, latitude: Latitude) -> Self {
        Self {
            longitude,
            latitude,
            frame: PhantomData,
        }
    }

    pub fn longitude(self) -> Longitude {
        self.longitude
    }

    pub fn latitude(self) -> Latitude {
        self.latitude
    }

    pub fn to_unit_vector(self) -> UnitVector<Frame> {
        let longitude = self.longitude.radians();
        let latitude = self.latitude.radians();
        UnitVector {
            components: [
                latitude.cos() * longitude.cos(),
                latitude.cos() * longitude.sin(),
                latitude.sin(),
            ],
            frame: PhantomData,
        }
    }
}

/// A normalized Cartesian direction in a statically named reference frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitVector<Frame> {
    components: [f64; 3],
    frame: PhantomData<Frame>,
}

impl<Frame> UnitVector<Frame> {
    pub fn new(components: [f64; 3]) -> Result<Self, ValueError> {
        for component in components.iter() {
            require_finite("vector component", *component)?;
        }
        let norm = (components[0] * components[0]
            + components[1] * components[1]
            + components[2] * components[2])
            .sqrt();
        if norm == 0.0 {
            return Err(ValueError::ZeroVector);
        }
        Ok(Self {
            components: [
                components[0] / norm,
                components[1] / norm,
                components[2] / norm,
            ],
            frame: PhantomData,
        })
    }

    pub fn components(self) -> [f64; 3] {
        self.components
    }

    pub fn to_direction(self) -> Direction<Frame> {
        let longitude = Longitude::from_radians(self.components[1].atan2(self.components[0]))
            .expect("finite normalized vector longitude");
        let latitude = Latitude::from_radians(self.components[2].clamp(-1.0, 1.0).asin())
            .expect("finite normalized vector latitude");
        Direction::new(longitude, latitude)
    }
}

/// A frame-safe rotation matrix from `From` into `To`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rotation<From, To> {
    matrix: [[f64; 3]; 3],
    frames: PhantomData<(From, To)>,
}

impl<From, To> Rotation<From, To> {
    pub(crate) fn from_matrix(matrix: [[f64; 3]; 3]) -> Result<Self, ValueError> {
        for row in matrix.iter() {
            for element in row.iter() {
                require_finite("rotation matrix element", *element)?;
            }
        }
        Ok(Self {
            matrix,
            frames: PhantomData,
        })
    }

    pub fn matrix(self) -> [[f64; 3]; 3] {
        self.matrix
    }

    pub fn apply(self, vector: UnitVector<From>) -> UnitVector<To> {
        let input = vector.components();
        UnitVector::new([
            self.matrix[0][0] * input[0]
                + self.matrix[0][1] * input[1]
                + self.matrix[0][2] * input[2],
            self.matrix[1][0] * input[0]
                + self.matrix[1][1] * input[1]
                + self.matrix[1][2] * input[2],
            self.matrix[2][0] * input[0]
                + self.matrix[2][1] * input[1]
                + self.matrix[2][2] * input[2],
        ])
        .expect("a finite rotation of a unit vector remains nonzero")
    }

    pub fn inverse(self) -> Rotation<To, From> {
        Rotation {
            matrix: [
                [self.matrix[0][0], self.matrix[1][0], self.matrix[2][0]],
                [self.matrix[0][1], self.matrix[1][1], self.matrix[2][1]],
                [self.matrix[0][2], self.matrix[1][2], self.matrix[2][2]],
            ],
            frames: PhantomData,
        }
    }
}

/// The named algorithm and revision that produced a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    name: &'static str,
    revision: &'static str,
}

impl Model {
    pub const fn new(name: &'static str, revision: &'static str) -> Self {
        Self { name, revision }
    }

    pub fn name(self) -> &'static str {
        self.name
    }

    pub fn revision(self) -> &'static str {
        self.revision
    }
}

/// How an error ceiling was established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccuracyEvidence {
    /// Agreement with a canonical implementation's published vectors.
    Conformance,
    /// Comparison with an independent external calculation.
    ExternalComparison,
}

/// A bounded angular error and the scope in which it was established.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accuracy {
    max_angular_error: Angle,
    evidence: AccuracyEvidence,
    authority: &'static str,
    scope: &'static str,
}

impl Accuracy {
    pub fn new(
        max_angular_error: Angle,
        evidence: AccuracyEvidence,
        authority: &'static str,
        scope: &'static str,
    ) -> Result<Self, ValueError> {
        if max_angular_error.radians() < 0.0 {
            return Err(ValueError::Negative("angular error"));
        }
        Ok(Self {
            max_angular_error,
            evidence,
            authority,
            scope,
        })
    }

    pub fn max_angular_error(self) -> Angle {
        self.max_angular_error
    }

    pub fn evidence(self) -> AccuracyEvidence {
        self.evidence
    }

    pub fn authority(self) -> &'static str {
        self.authority
    }

    pub fn scope(self) -> &'static str {
        self.scope
    }
}

/// A value accompanied by its model and measured or conformance accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Modelled<T> {
    value: T,
    model: Model,
    accuracy: Accuracy,
}

impl<T> Modelled<T> {
    pub fn new(value: T, model: Model, accuracy: Accuracy) -> Self {
        Self {
            value,
            model,
            accuracy,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn model(&self) -> Model {
        self.model
    }

    pub fn accuracy(&self) -> Accuracy {
        self.accuracy
    }
}

/// A geocentric celestial state in a statically named frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State<Frame> {
    epoch: JulianDate<TerrestrialTime>,
    direction: Direction<Frame>,
    distance: Distance,
}

impl<Frame> State<Frame> {
    pub fn new(
        epoch: JulianDate<TerrestrialTime>,
        direction: Direction<Frame>,
        distance: Distance,
    ) -> Self {
        Self {
            epoch,
            direction,
            distance,
        }
    }

    pub fn epoch(self) -> JulianDate<TerrestrialTime> {
        self.epoch
    }

    pub fn direction(self) -> Direction<Frame> {
        self.direction
    }

    pub fn distance(self) -> Distance {
        self.distance
    }
}

fn require_finite(field: &'static str, value: f64) -> Result<(), ValueError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ValueError::NotFinite(field))
    }
}
