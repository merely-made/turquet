// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Observer-relative apparent positions.
//!
//! This module keeps the two clocks required by Earth-based observation
//! explicit: TT drives the ephemeris and precession-nutation model, while UT1
//! drives Earth rotation. Polar motion is caller-supplied observational data.
//! The default calculation is airless and uses the WGS84 reference ellipsoid.

use std::f64::consts::PI;
use std::fmt;

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

/// Provider-neutral WGS84 topocentric and airless horizon transform.
pub const AIRLESS_TOPOCENTRIC_TRANSFORM: Model =
    Model::new("Turquet WGS84 airless topocentric transform", "1");

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

    /// Signed local apparent hour angle on `[-pi, pi)`.
    ///
    /// This is local apparent sidereal time minus topocentric apparent right
    /// ascension. It is positive west of the local meridian and uses the
    /// observation's TT, UT1, polar-motion-adjusted observer longitude, and
    /// IAU 2006/2000A sidereal model.
    pub(crate) fn local_apparent_hour_angle(&self) -> Angle {
        let (ut1a, ut1b) = self.earth_orientation.ut1().parts();
        let (tta, ttb) = self.equatorial.epoch().parts();
        let greenwich = sofars::erst::gst06a(ut1a, ut1b, tta, ttb);
        let local = greenwich
            + polar_motion_adjusted_longitude(
                self.equatorial.epoch(),
                &self.earth_orientation,
                self.observer,
            );
        let right_ascension = self.equatorial.direction().longitude().radians();
        let signed = (local - right_ascension + PI).rem_euclid(2.0 * PI) - PI;
        Angle::from_radians(signed).expect("sidereal and right-ascension angles are finite")
    }
}

/// Return the observer longitude corrected onto the local meridian.
///
/// SOFA's `apio` couples the geodetic site with TT, UT1, and polar motion to
/// produce `along`. The meridian-transit path combines that adjusted longitude
/// with equinox-based Greenwich apparent sidereal time because Turquet's
/// topocentric right ascension is on the true equator and equinox of date.
fn polar_motion_adjusted_longitude(
    epoch: JulianDate<TerrestrialTime>,
    earth_orientation: &EarthOrientation,
    observer: Observer,
) -> f64 {
    let (tta, ttb) = epoch.parts();
    let (ut1a, ut1b) = earth_orientation.ut1().parts();
    local_meridian_angles(
        sofars::pnp::sp00(tta, ttb),
        sofars::erst::era00(ut1a, ut1b),
        observer.longitude().radians(),
        observer.latitude().radians(),
        observer.height().meters(),
        earth_orientation.polar_motion_x().radians(),
        earth_orientation.polar_motion_y().radians(),
    )
    .0
}

fn local_meridian_angles(
    sp: f64,
    theta: f64,
    longitude: f64,
    latitude: f64,
    height_meters: f64,
    polar_motion_x: f64,
    polar_motion_y: f64,
) -> (f64, f64) {
    let mut astrom = sofars::astro::IauAstrom::default();
    sofars::astro::apio(
        sp,
        theta,
        longitude,
        latitude,
        height_meters,
        polar_motion_x,
        polar_motion_y,
        0.0,
        0.0,
        &mut astrom,
    );
    (astrom.along, astrom.eral)
}

/// A rejected provider-neutral observer transformation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObserverTransformError {
    /// The supplied geocentric state was calculated for another TT epoch.
    EpochMismatch {
        transform_epoch: JulianDate<TerrestrialTime>,
        state_epoch: JulianDate<TerrestrialTime>,
    },
    /// A zero topocentric vector has no celestial direction.
    BodyAtObserver { epoch: JulianDate<TerrestrialTime> },
}

impl fmt::Display for ObserverTransformError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ObserverTransformError::EpochMismatch {
                transform_epoch,
                state_epoch,
            } => write!(
                formatter,
                "observer transform TT JD {} does not match state TT JD {}",
                transform_epoch.day(),
                state_epoch.day()
            ),
            ObserverTransformError::BodyAtObserver { epoch } => write!(
                formatter,
                "celestial body coincides with the observer at TT JD {}",
                epoch.day()
            ),
        }
    }
}

impl ::std::error::Error for ObserverTransformError {}

/// Epoch- and site-scoped transform for a supplied geocentric apparent state.
///
/// This is the provider-neutral half of [`ObserverSky`]. It applies WGS84 site
/// geometry, caller-supplied UT1 and polar motion, and the measured airless
/// horizon projection without selecting an ephemeris provider.
#[derive(Clone, Debug, PartialEq)]
pub struct ObserverTransform {
    epoch: JulianDate<TerrestrialTime>,
    observer: Observer,
    earth_orientation: EarthOrientation,
    gcrs_to_true_equator: [[f64; 3]; 3],
    gcrs_to_itrs: [[f64; 3]; 3],
    observer_true_equator_meters: [f64; 3],
}

impl ObserverTransform {
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
            epoch,
            observer,
            earth_orientation,
            gcrs_to_true_equator,
            gcrs_to_itrs,
            observer_true_equator_meters: observer_true_equator,
        }
    }

    pub fn epoch(&self) -> JulianDate<TerrestrialTime> {
        self.epoch
    }

    pub fn observer(&self) -> Observer {
        self.observer
    }

    pub fn earth_orientation(&self) -> &EarthOrientation {
        &self.earth_orientation
    }

    pub fn observe(
        &self,
        geocentric: State<TrueEclipticEquinoxOfDate>,
    ) -> Result<Modelled<Observation>, ObserverTransformError> {
        if geocentric.epoch() != self.epoch {
            return Err(ObserverTransformError::EpochMismatch {
                transform_epoch: self.epoch,
                state_epoch: geocentric.epoch(),
            });
        }
        let geocentric_equatorial =
            ecliptic_to_equatorial(self.epoch, geocentric.direction(), geocentric.distance());
        let topocentric_vector = subtract(geocentric_equatorial, self.observer_true_equator_meters);
        let topocentric_distance = norm(topocentric_vector);
        let equatorial_direction =
            UnitVector::<TopocentricTrueEquatorEquinoxOfDate>::new(topocentric_vector)
                .map_err(|_| ObserverTransformError::BodyAtObserver { epoch: self.epoch })?
                .to_direction();
        let equatorial = State::new(
            self.epoch,
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
            AIRLESS_TOPOCENTRIC_TRANSFORM,
            observer_accuracy(),
        ))
    }
}

/// Epoch- and site-scoped observer calculation context.
///
/// The expensive ephemeris orientation and terrestrial rotation matrices are
/// evaluated once and reused for every body in a sky view.
#[derive(Clone, Debug, PartialEq)]
pub struct ObserverSky {
    apparent: ApparentSky,
    transform: ObserverTransform,
}

impl ObserverSky {
    pub fn at(
        epoch: JulianDate<TerrestrialTime>,
        earth_orientation: EarthOrientation,
        observer: Observer,
    ) -> Self {
        Self {
            apparent: ApparentSky::at(epoch),
            transform: ObserverTransform::at(epoch, earth_orientation, observer),
        }
    }

    pub fn epoch(&self) -> JulianDate<TerrestrialTime> {
        self.apparent.epoch()
    }

    pub fn observer(&self) -> Observer {
        self.transform.observer()
    }

    pub fn earth_orientation(&self) -> &EarthOrientation {
        self.transform.earth_orientation()
    }

    pub fn position(&self, body: ApparentBody) -> Result<Modelled<Observation>, ApparentError> {
        let geocentric = self.apparent.position(body)?.into_value();
        let observation = self
            .transform
            .observe(geocentric)
            .expect("ObserverSky supplies a state at the transform epoch")
            .into_value();

        Ok(Modelled::new(
            observation,
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

#[cfg(test)]
mod tests {
    use super::local_meridian_angles;

    #[test]
    fn local_meridian_angles_match_sofa_apio_vector() {
        // IAU SOFA 2023-10-11, t_sofa_c iauApio.
        let (along, eral) = local_meridian_angles(
            -3.019_743_37e-11,
            3.145_409_71,
            -0.527_800_806,
            -1.234_585_6,
            2_738.0,
            2.472_307_37e-7,
            1.826_404_64e-6,
        );
        assert!(
            (along - -0.527_800_806_029_599_6).abs() < 1e-12,
            "adjusted longitude {}",
            along,
        );
        assert!(
            (eral - 2.617_608_903_970_400_4).abs() < 1e-12,
            "local ERA {}",
            eral
        );
    }
}
