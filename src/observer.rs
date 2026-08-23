// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Observer-relative apparent positions.
//!
//! This module keeps the two clocks required by Earth-based observation
//! explicit: TT drives the ephemeris and precession-nutation model, while UT1
//! drives Earth rotation. Polar motion is caller-supplied observational data.
//! The default calculation is airless and uses the WGS84 reference ellipsoid.

use apparent::{ApparentBody, ApparentError, ApparentSky};
use foundation::{
    Accuracy, AccuracyEvidence, Angle, Direction, Distance, JulianDate, Latitude, Longitude, Model,
    Modelled, Observer, State, TerrestrialTime, TopocentricHorizon,
    TopocentricTrueEquatorEquinoxOfDate, TrueEclipticEquinoxOfDate, UnitVector, UniversalTime1,
};
use orientation;

/// The composed observer-relative model used by [`position`].
pub const ANALYTICAL_TOPOCENTRIC: Model =
    Model::new("Turquet analytical topocentric apparent", "1");

/// Observational Earth-orientation inputs for one instant.
///
/// `polar_motion_x` and `polar_motion_y` follow the IERS convention and are
/// normally supplied in arcseconds. The authority and snapshot identify the
/// external observation or explicit approximation used by the caller.
#[derive(Clone, Debug, PartialEq)]
pub struct EarthOrientation {
    ut1: JulianDate<UniversalTime1>,
    polar_motion_x: Angle,
    polar_motion_y: Angle,
    authority: String,
    snapshot: String,
}

impl EarthOrientation {
    /// Construct an Earth-orientation snapshot.
    ///
    /// ```compile_fail
    /// use turquet::foundation::{Angle, JulianDate, TerrestrialTime};
    /// use turquet::observer::EarthOrientation;
    /// let tt = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0).unwrap();
    /// EarthOrientation::new(tt, Angle::from_radians(0.0).unwrap(), Angle::from_radians(0.0).unwrap(), "IERS", "snapshot");
    /// ```
    pub fn new<A, S>(
        ut1: JulianDate<UniversalTime1>,
        polar_motion_x: Angle,
        polar_motion_y: Angle,
        authority: A,
        snapshot: S,
    ) -> Self
    where
        A: Into<String>,
        S: Into<String>,
    {
        Self {
            ut1,
            polar_motion_x,
            polar_motion_y,
            authority: authority.into(),
            snapshot: snapshot.into(),
        }
    }

    /// Construct an explicit zero-polar-motion approximation.
    pub fn zero_polar_motion<A, S>(
        ut1: JulianDate<UniversalTime1>,
        authority: A,
        snapshot: S,
    ) -> Self
    where
        A: Into<String>,
        S: Into<String>,
    {
        Self::new(
            ut1,
            Angle::from_radians(0.0).expect("zero is a finite angle"),
            Angle::from_radians(0.0).expect("zero is a finite angle"),
            authority,
            snapshot,
        )
    }

    pub fn ut1(&self) -> JulianDate<UniversalTime1> {
        self.ut1
    }

    pub fn polar_motion_x(&self) -> Angle {
        self.polar_motion_x
    }

    pub fn polar_motion_y(&self) -> Angle {
        self.polar_motion_y
    }

    pub fn authority(&self) -> &str {
        &self.authority
    }

    pub fn snapshot(&self) -> &str {
        &self.snapshot
    }
}

/// One airless observer-relative apparent result.
#[derive(Clone, Debug, PartialEq)]
pub struct Observation {
    observer: Observer,
    earth_orientation: EarthOrientation,
    equatorial: State<TopocentricTrueEquatorEquinoxOfDate>,
    horizon: Direction<TopocentricHorizon>,
}

impl Observation {
    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn earth_orientation(&self) -> &EarthOrientation {
        &self.earth_orientation
    }

    /// Topocentric right ascension and declination on the true equator and
    /// equinox of date. Longitude is right ascension.
    pub fn equatorial(&self) -> State<TopocentricTrueEquatorEquinoxOfDate> {
        self.equatorial
    }

    /// Airless azimuth and altitude. Azimuth is north-zero and east-positive.
    pub fn horizon(&self) -> Direction<TopocentricHorizon> {
        self.horizon
    }
}

/// Epoch- and site-scoped observer calculation context.
///
/// The expensive ephemeris orientation and terrestrial rotation matrices are
/// evaluated once and reused for every body in a sky view.
#[derive(Clone, Debug, PartialEq)]
pub struct ObserverSky {
    apparent: ApparentSky,
    observer: Observer,
    earth_orientation: EarthOrientation,
    gcrs_to_true_equator: [[f64; 3]; 3],
    gcrs_to_itrs: [[f64; 3]; 3],
    observer_true_equator_meters: [f64; 3],
}

impl ObserverSky {
    pub fn at(
        epoch: JulianDate<TerrestrialTime>,
        earth_orientation: EarthOrientation,
        observer: Observer,
    ) -> Self {
        let (tt1, tt2) = epoch.parts();
        let (ut11, ut12) = earth_orientation.ut1().parts();
        let gcrs_to_true_equator = orientation::gcrs_to_true_equator(epoch)
            .into_value()
            .matrix();
        let gcrs_to_itrs = sofars::pnp::c2t06a(
            tt1,
            tt2,
            ut11,
            ut12,
            earth_orientation.polar_motion_x().radians(),
            earth_orientation.polar_motion_y().radians(),
        );
        let observer_itrs = sofars::coords::gd2gc(
            1,
            observer.longitude().radians(),
            observer.latitude().radians(),
            observer.height().meters(),
        )
        .expect("a validated observer is representable on WGS84");
        let observer_gcrs = matrix_vector(transpose(gcrs_to_itrs), observer_itrs);
        let observer_true_equator = matrix_vector(gcrs_to_true_equator, observer_gcrs);

        Self {
            apparent: ApparentSky::at(epoch),
            observer,
            earth_orientation,
            gcrs_to_true_equator,
            gcrs_to_itrs,
            observer_true_equator_meters: observer_true_equator,
        }
    }

    pub fn epoch(&self) -> JulianDate<TerrestrialTime> {
        self.apparent.epoch()
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn earth_orientation(&self) -> &EarthOrientation {
        &self.earth_orientation
    }

    pub fn position(&self, body: ApparentBody) -> Result<Modelled<Observation>, ApparentError> {
        let geocentric = self.apparent.position(body)?.into_value();
        let geocentric_equatorial =
            ecliptic_to_equatorial(self.epoch(), geocentric.direction(), geocentric.distance());
        let topocentric_vector = subtract(geocentric_equatorial, self.observer_true_equator_meters);
        let topocentric_distance = norm(topocentric_vector);
        let equatorial_direction =
            UnitVector::<TopocentricTrueEquatorEquinoxOfDate>::new(topocentric_vector)
                .expect("a celestial body cannot coincide with the observer")
                .to_direction();
        let equatorial = State::new(
            self.epoch(),
            equatorial_direction,
            Distance::from_meters(topocentric_distance)
                .expect("topocentric distance is finite and nonnegative"),
        );

        let true_equator_to_gcrs = transpose(self.gcrs_to_true_equator);
        let topocentric_gcrs = matrix_vector(true_equator_to_gcrs, topocentric_vector);
        let topocentric_itrs = matrix_vector(self.gcrs_to_itrs, topocentric_gcrs);
        let horizon = horizon_direction(topocentric_itrs, self.observer);

        Ok(Modelled::new(
            Observation {
                observer: self.observer,
                earth_orientation: self.earth_orientation.clone(),
                equatorial,
                horizon,
            },
            ANALYTICAL_TOPOCENTRIC,
            observer_accuracy(),
        ))
    }
}

/// Calculate one observer-relative apparent position.
pub fn position(
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    earth_orientation: EarthOrientation,
    observer: Observer,
) -> Result<Modelled<Observation>, ApparentError> {
    ObserverSky::at(epoch, earth_orientation, observer).position(body)
}

fn ecliptic_to_equatorial(
    epoch: JulianDate<TerrestrialTime>,
    direction: Direction<TrueEclipticEquinoxOfDate>,
    distance: Distance,
) -> [f64; 3] {
    let unit = orientation::true_ecliptic_to_true_equator(epoch)
        .into_value()
        .apply(direction.to_unit_vector())
        .components();
    scale(unit, distance.meters())
}

fn horizon_direction(
    topocentric_itrs: [f64; 3],
    observer: Observer,
) -> Direction<TopocentricHorizon> {
    let longitude = observer.longitude().radians();
    let latitude = observer.latitude().radians();
    let (sin_longitude, cos_longitude) = longitude.sin_cos();
    let (sin_latitude, cos_latitude) = latitude.sin_cos();

    let east = [-sin_longitude, cos_longitude, 0.0];
    let north = [
        -sin_latitude * cos_longitude,
        -sin_latitude * sin_longitude,
        cos_latitude,
    ];
    let up = [
        cos_latitude * cos_longitude,
        cos_latitude * sin_longitude,
        sin_latitude,
    ];
    let east_component = dot(topocentric_itrs, east);
    let north_component = dot(topocentric_itrs, north);
    let up_component = dot(topocentric_itrs, up);

    Direction::new(
        Longitude::from_radians(east_component.atan2(north_component))
            .expect("finite topocentric azimuth"),
        Latitude::from_radians(up_component.atan2(east_component.hypot(north_component)))
            .expect("finite topocentric altitude"),
    )
}

fn observer_accuracy() -> Accuracy {
    Accuracy::new(
        Angle::from_degrees(0.010).expect("positive finite tolerance"),
        AccuracyEvidence::ExternalComparison,
        "NASA/JPL Horizons",
        "90 airless vectors at 3 sites and 3 epochs; underlying 1885-2099 date cohort",
    )
    .expect("positive observer accuracy")
}

fn matrix_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn transpose(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2]],
    ]
}

fn scale(vector: [f64; 3], magnitude: f64) -> [f64; 3] {
    [
        vector[0] * magnitude,
        vector[1] * magnitude,
        vector[2] * magnitude,
    ]
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn norm(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}
