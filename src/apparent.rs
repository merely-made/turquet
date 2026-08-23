// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Apparent geocentric ecliptic positions for the ten classical chart bodies.
//!
//! This is the first Turquet-era module and the T3 down payment: one pipeline
//! that composes the inherited VSOP87D planetary theory, the partial
//! ELP-2000/82 lunar solution, the analytical Pluto series, IAU 2006/2000A
//! nutation, and the inherited ecliptic precession into apparent positions on
//! the true equinox of date. No runtime data file is involved.
//!
//! Validation authority: NASA/JPL Horizons observer-table quantity 31
//! (`tests/apparent.rs`), at J2000, the 2024-04-08 total solar eclipse, and
//! 2026-08-13. Measured residuals: the Sun, Moon, and eight planets agree
//! with Horizons at millidegree rounding (worst 2 millidegrees). A committed
//! 5,277-vector DE440s cohort spanning 1885 through 2099 is held below a
//! 10-millidegree gate (measured worst 3 millidegrees).
//!
//! Explicit stages, per the roadmap: heliocentric position of date
//! (ELP-2000/82 is directly geocentric; Pluto is precessed from its J2000
//! frame with the inherited ecliptic precession), light-time iteration,
//! annual aberration from a numerically differentiated Earth velocity, and
//! nutation in longitude. Solar gravitational deflection is not applied.
//!
//! Supported range: the Pluto series is stated for 1885 to 2099. Typed TT
//! inputs can represent earlier epochs directly. The compatibility UTC-field
//! conversion is defined from 1972, when UTC gained leap seconds and stopped
//! using rubber seconds. Requests outside a defined range are errors.
//!
//! Time scales come from `hifitime`, which owns the UTC-to-TT conversion and
//! the leap-second table. That delegates the table's maintenance to a crate
//! that tracks IERS bulletins; it does not make the table immortal, so a
//! result's disclosure should still name the engine revision it was computed
//! with.

use angle;
use foundation::{
    Accuracy, AccuracyEvidence, Angle, Direction, Distance, JulianDate, Latitude, Longitude, Model,
    Modelled, State, TerrestrialTime, TrueEclipticEquinoxOfDate,
};
use lunar;
use orientation;
use planet;
use pluto;
use precess;

/// Astronomical units travelled by light in one day.
const LIGHT_SPEED_AU_PER_DAY: f64 = 173.144_632_674_24;
/// Central-difference step for Earth's velocity, in days.
const VELOCITY_STEP_DAYS: f64 = 0.01;
/// Central-difference step for the retrograde test, in days.
const RETROGRADE_STEP_DAYS: f64 = 0.5;
const J2000_JD: f64 = 2_451_545.0;
/// The Pluto series' stated validity, in Julian years.
const PLUTO_RANGE_YEARS: (f64, f64) = (1885.0, 2099.0);
/// UTC gained leap seconds on this date; earlier UTC used rubber seconds and
/// is not the same time scale.
const UTC_LEAP_ERA_START: (i32, u32, u32) = (1972, 1, 1);

/// The composed analytical model used by [`position`].
pub const ANALYTICAL_APPARENT: Model = Model::new("Turquet analytical apparent ecliptic", "2");

/// Epoch-scoped apparent calculation context.
///
/// The full IAU 2000A nutation series is evaluated once and reused for every
/// body requested at this epoch. A chart or sky view should construct one of
/// these rather than call [`position`] repeatedly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApparentSky {
    epoch: JulianDate<TerrestrialTime>,
    nutation_longitude: Angle,
}

impl ApparentSky {
    pub fn at(epoch: JulianDate<TerrestrialTime>) -> Self {
        Self {
            epoch,
            nutation_longitude: orientation::nutation(epoch).value().longitude(),
        }
    }

    pub fn epoch(self) -> JulianDate<TerrestrialTime> {
        self.epoch
    }

    pub fn position(
        self,
        body: ApparentBody,
    ) -> Result<Modelled<State<TrueEclipticEquinoxOfDate>>, ApparentError> {
        let (longitude, latitude, distance) =
            calculate(&body, self.epoch, self.nutation_longitude)?;
        let direction = Direction::new(
            Longitude::from_radians(longitude).expect("analytical longitude is finite"),
            Latitude::from_radians(latitude).expect("analytical latitude is finite and physical"),
        );
        Ok(Modelled::new(
            State::new(self.epoch, direction, distance),
            ANALYTICAL_APPARENT,
            apparent_accuracy(),
        ))
    }
}

/// The ten bodies of the apparent pipeline, in conventional chart order.
pub const APPARENT_BODIES: [ApparentBody; 10] = [
    ApparentBody::Sun,
    ApparentBody::Moon,
    ApparentBody::Mercury,
    ApparentBody::Venus,
    ApparentBody::Mars,
    ApparentBody::Jupiter,
    ApparentBody::Saturn,
    ApparentBody::Uranus,
    ApparentBody::Neptune,
    ApparentBody::Pluto,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApparentBody {
    Sun,
    Moon,
    Mercury,
    Venus,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
    Pluto,
}

impl ApparentBody {
    pub fn name(&self) -> &'static str {
        match *self {
            ApparentBody::Sun => "sun",
            ApparentBody::Moon => "moon",
            ApparentBody::Mercury => "mercury",
            ApparentBody::Venus => "venus",
            ApparentBody::Mars => "mars",
            ApparentBody::Jupiter => "jupiter",
            ApparentBody::Saturn => "saturn",
            ApparentBody::Uranus => "uranus",
            ApparentBody::Neptune => "neptune",
            ApparentBody::Pluto => "pluto",
        }
    }

    /// Heliocentric rectangular coordinates on the mean ecliptic of date, in
    /// AU. The Sun sits at the origin by construction; the Moon is geocentric
    /// and never enters this path.
    fn heliocent_rect(&self, jde_tt: f64) -> [f64; 3] {
        match *self {
            ApparentBody::Sun => [0.0, 0.0, 0.0],
            ApparentBody::Moon => {
                unreachable!("the Moon is geocentric and handled in apparent position")
            }
            ApparentBody::Mercury => planet_rect(&planet::Planet::Mercury, jde_tt),
            ApparentBody::Venus => planet_rect(&planet::Planet::Venus, jde_tt),
            ApparentBody::Mars => planet_rect(&planet::Planet::Mars, jde_tt),
            ApparentBody::Jupiter => planet_rect(&planet::Planet::Jupiter, jde_tt),
            ApparentBody::Saturn => planet_rect(&planet::Planet::Saturn, jde_tt),
            ApparentBody::Uranus => planet_rect(&planet::Planet::Uranus, jde_tt),
            ApparentBody::Neptune => planet_rect(&planet::Planet::Neptune, jde_tt),
            ApparentBody::Pluto => pluto_rect_of_date(jde_tt),
        }
    }
}

/// An epoch outside a model's stated validity.
#[derive(Clone, Debug, PartialEq)]
pub enum ApparentError {
    /// The UTC instant precedes the 1972 leap-second era.
    BeforeLeapSecondEra,
    /// The calendar fields do not form a valid UTC instant.
    InvalidCivilTime,
    /// The requested epoch is outside the named body's series validity.
    OutsideSeriesRange {
        body: &'static str,
        julian_year: f64,
    },
}

/// Apparent geocentric state on the true ecliptic and equinox of date.
///
/// The epoch type requires TT and the state type records the output frame.
/// Distances are geocentric. Accuracy metadata reports the measured angular
/// ceiling of the existing Horizons cohort; it is not a distance claim.
pub fn position(
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<Modelled<State<TrueEclipticEquinoxOfDate>>, ApparentError> {
    ApparentSky::at(epoch).position(body)
}

/// Whether apparent ecliptic longitude is decreasing across a one-day
/// central-difference interval.
pub fn is_retrograde(
    body: ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<bool, ApparentError> {
    let before_epoch = epoch
        .offset_days(-RETROGRADE_STEP_DAYS)
        .expect("finite fixed sample offset");
    let before = ApparentSky::at(before_epoch)
        .position(body)?
        .value()
        .direction()
        .longitude()
        .radians();
    let after_epoch = epoch
        .offset_days(RETROGRADE_STEP_DAYS)
        .expect("finite fixed sample offset");
    let after = ApparentSky::at(after_epoch)
        .position(body)?
        .value()
        .direction()
        .longitude()
        .radians();
    let two_pi = 2.0 * ::std::f64::consts::PI;
    let delta =
        (after - before + ::std::f64::consts::PI).rem_euclid(two_pi) - ::std::f64::consts::PI;
    Ok(delta < 0.0)
}

pub(crate) fn legacy_jde_tt_frm_epoch(epoch: hifitime::Epoch) -> f64 {
    JulianDate::<TerrestrialTime>::from_epoch(epoch).day()
}

/// Compatibility conversion from Gregorian UTC fields to TT Julian day.
///
/// Defined from 1972: earlier instants are refused rather than converted,
/// because pre-1972 UTC is a different time scale rather than the same one
/// with a smaller offset.
pub(crate) fn legacy_jde_tt_frm_utc(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: f64,
) -> Result<f64, ApparentError> {
    if (year, month, day) < UTC_LEAP_ERA_START {
        return Err(ApparentError::BeforeLeapSecondEra);
    }
    if month > u32::from(u8::MAX)
        || day > u32::from(u8::MAX)
        || hour > u32::from(u8::MAX)
        || minute > u32::from(u8::MAX)
        || !(second >= 0.0 && second < 61.0)
    {
        return Err(ApparentError::InvalidCivilTime);
    }
    let whole_seconds = second.trunc();
    let nanos = ((second - whole_seconds) * 1e9).round() as u32;
    let epoch = hifitime::Epoch::maybe_from_gregorian_utc(
        year,
        month as u8,
        day as u8,
        hour as u8,
        minute as u8,
        whole_seconds as u8,
        nanos,
    )
    .map_err(|_| ApparentError::InvalidCivilTime)?;
    Ok(legacy_jde_tt_frm_epoch(epoch))
}

pub(crate) fn legacy_geocent_apparent_ecl_pos(
    body: &ApparentBody,
    jde_tt: f64,
) -> Result<(f64, f64), ApparentError> {
    let epoch = JulianDate::<TerrestrialTime>::from_julian_day(jde_tt)
        .expect("legacy Julian day must be finite");
    let state = position(*body, epoch)?;
    let direction = state.value().direction();
    Ok((
        direction.longitude().radians(),
        direction.latitude().radians(),
    ))
}

pub(crate) fn legacy_is_retrograde(
    body: &ApparentBody,
    jde_tt: f64,
) -> Result<bool, ApparentError> {
    let epoch = JulianDate::<TerrestrialTime>::from_julian_day(jde_tt)
        .expect("legacy Julian day must be finite");
    is_retrograde(*body, epoch)
}

fn calculate(
    body: &ApparentBody,
    epoch: JulianDate<TerrestrialTime>,
    nutation_longitude: Angle,
) -> Result<(f64, f64, Distance), ApparentError> {
    let jde_tt = epoch.day();
    check_range(body, jde_tt)?;
    if let ApparentBody::Moon = *body {
        return Ok(moon_apparent(epoch, nutation_longitude));
    }

    let earth = planet_rect(&planet::Planet::Earth, jde_tt);

    // Light-time: the body is seen where it was when the light left it.
    let mut offset = [0.0_f64; 3];
    let mut light_time_days = 0.0;
    for _ in 0..3 {
        let target = body.heliocent_rect(jde_tt - light_time_days);
        offset = [
            target[0] - earth[0],
            target[1] - earth[1],
            target[2] - earth[2],
        ];
        light_time_days = norm(&offset) / LIGHT_SPEED_AU_PER_DAY;
    }

    // Annual aberration displaces the apparent direction toward the
    // observer's motion. Earth's velocity comes from a central difference
    // rather than a memorised orbit constant.
    let velocity = earth_velocity(jde_tt);
    let direction = normalize(&offset);
    let aberrated = normalize(&[
        direction[0] + velocity[0] / LIGHT_SPEED_AU_PER_DAY,
        direction[1] + velocity[1] / LIGHT_SPEED_AU_PER_DAY,
        direction[2] + velocity[2] / LIGHT_SPEED_AU_PER_DAY,
    ]);

    // Mean equinox of date to true equinox of date.
    let nut_in_long = nutation_longitude.radians();
    let longitude = angle::limit_to_two_PI(aberrated[1].atan2(aberrated[0]) + nut_in_long);
    let latitude = aberrated[2].asin();
    let distance = Distance::from_astronomical_units(norm(&offset))
        .expect("analytical distance is finite and nonnegative");
    Ok((longitude, latitude, distance))
}

/// The Moon from the inherited partial ELP-2000/82: already geocentric on the
/// mean equinox of date, so no light-time loop or aberration term is added;
/// apparent position is the series value plus nutation.
fn moon_apparent(
    epoch: JulianDate<TerrestrialTime>,
    nutation_longitude: Angle,
) -> (f64, f64, Distance) {
    let (point, distance_km) = lunar::geocent_ecl_pos(epoch.day());
    let nut_in_long = nutation_longitude.radians();
    (
        angle::limit_to_two_PI(point.long + nut_in_long),
        point.lat,
        Distance::from_kilometers(distance_km).expect("lunar distance is finite and nonnegative"),
    )
}

fn apparent_accuracy() -> Accuracy {
    Accuracy::new(
        Angle::from_degrees(0.010).expect("positive finite tolerance"),
        AccuracyEvidence::ExternalComparison,
        "NASA/JPL Horizons",
        "5,277 DE440s vectors across 1885-2099; angular only",
    )
    .expect("positive apparent accuracy")
}

fn check_range(body: &ApparentBody, jde_tt: f64) -> Result<(), ApparentError> {
    if let ApparentBody::Pluto = *body {
        let julian_year = 2000.0 + (jde_tt - J2000_JD) / 365.25;
        if julian_year < PLUTO_RANGE_YEARS.0 || julian_year > PLUTO_RANGE_YEARS.1 {
            return Err(ApparentError::OutsideSeriesRange {
                body: "pluto",
                julian_year: julian_year,
            });
        }
    }
    Ok(())
}

fn planet_rect(planet: &planet::Planet, jde: f64) -> [f64; 3] {
    let (longitude, latitude, radius) = planet::heliocent_coords(planet, jde);
    spherical_to_rect(longitude, latitude, radius)
}

/// The inherited Pluto series is referred to the standard equinox of J2000.0;
/// the rest of the pipeline is referred to the equinox of date. The inherited
/// ecliptic precession carries it across before the frames are mixed.
fn pluto_rect_of_date(jde: f64) -> [f64; 3] {
    let (longitude_j2000, latitude_j2000, radius) = pluto::heliocent_pos(jde);
    let (longitude, latitude) =
        precess::precess_ecl_coords(longitude_j2000, latitude_j2000, J2000_JD, jde);
    spherical_to_rect(longitude, latitude, radius)
}

fn spherical_to_rect(longitude: f64, latitude: f64, radius: f64) -> [f64; 3] {
    [
        radius * latitude.cos() * longitude.cos(),
        radius * latitude.cos() * longitude.sin(),
        radius * latitude.sin(),
    ]
}

fn earth_velocity(jde: f64) -> [f64; 3] {
    let before = planet_rect(&planet::Planet::Earth, jde - VELOCITY_STEP_DAYS);
    let after = planet_rect(&planet::Planet::Earth, jde + VELOCITY_STEP_DAYS);
    let scale = 2.0 * VELOCITY_STEP_DAYS;
    [
        (after[0] - before[0]) / scale,
        (after[1] - before[1]) / scale,
        (after[2] - before[2]) / scale,
    ]
}

fn norm(vector: &[f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn normalize(vector: &[f64; 3]) -> [f64; 3] {
    let length = norm(vector);
    if length == 0.0 {
        return *vector;
    }
    [vector[0] / length, vector[1] / length, vector[2] / length]
}
