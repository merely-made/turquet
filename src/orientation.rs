// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! IAU 2006 precession with IAU 2000A nutation.
//!
//! The numerical routines are supplied by the pure-Rust `sofars` crate and
//! wrapped here so raw Julian dates and unlabelled matrices do not cross the
//! primary Turquet API. `sofars` is derived from the IAU SOFA collection and
//! carries the SOFA terms reproduced in its distribution. Turquet is neither
//! SOFA software nor endorsed by the IAU SOFA Board.

use foundation::{
    Accuracy, AccuracyEvidence, Angle, Gcrs, JulianDate, Model, Modelled, Rotation,
    TerrestrialTime, TrueEquatorEquinoxOfDate,
};

/// The model revision used by this module.
pub const IAU_2006_2000A: Model = Model::new(
    "IAU 2006 precession + IAU 2000A nutation",
    "SOFA 2023-10-11 via sofars 0.6.1",
);

/// Nutation in longitude and obliquity with respect to the IAU 2006 mean
/// equinox and ecliptic of date.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nutation {
    longitude: Angle,
    obliquity: Angle,
}

impl Nutation {
    pub fn longitude(self) -> Angle {
        self.longitude
    }

    pub fn obliquity(self) -> Angle {
        self.obliquity
    }
}

/// IAU 2000A nutation adjusted to match IAU 2006 precession.
pub fn nutation(epoch: JulianDate<TerrestrialTime>) -> Modelled<Nutation> {
    let (day1, day2) = epoch.parts();
    let (longitude, obliquity) = sofars::pnp::nut06a(day1, day2);
    Modelled::new(
        Nutation {
            longitude: Angle::from_radians(longitude).expect("SOFARS returns a finite angle"),
            obliquity: Angle::from_radians(obliquity).expect("SOFARS returns a finite angle"),
        },
        IAU_2006_2000A,
        conformance_accuracy(),
    )
}

/// Bias-precession-nutation rotation from GCRS into the true equator and
/// equinox of date.
///
/// The frame parameters prevent applying this matrix to a direction already
/// expressed in the destination frame.
///
/// ```compile_fail
/// use turquet::foundation::{JulianDate, TerrestrialTime, TrueEquatorEquinoxOfDate, UnitVector};
/// use turquet::orientation::gcrs_to_true_equator;
/// let epoch = JulianDate::<TerrestrialTime>::from_julian_day(2_451_545.0).unwrap();
/// let rotation = gcrs_to_true_equator(epoch).into_value();
/// let already_of_date = UnitVector::<TrueEquatorEquinoxOfDate>::new([1.0, 0.0, 0.0]).unwrap();
/// rotation.apply(already_of_date);
/// ```
///
/// ```compile_fail
/// use turquet::foundation::{JulianDate, UniversalTime1};
/// use turquet::orientation::gcrs_to_true_equator;
/// let ut1 = JulianDate::<UniversalTime1>::from_julian_day(2_451_545.0).unwrap();
/// gcrs_to_true_equator(ut1);
/// ```
pub fn gcrs_to_true_equator(
    epoch: JulianDate<TerrestrialTime>,
) -> Modelled<Rotation<Gcrs, TrueEquatorEquinoxOfDate>> {
    let (day1, day2) = epoch.parts();
    let matrix = sofars::pnp::pnm06a(day1, day2);
    Modelled::new(
        Rotation::from_matrix(matrix).expect("SOFARS returns a finite rotation matrix"),
        IAU_2006_2000A,
        conformance_accuracy(),
    )
}

fn conformance_accuracy() -> Accuracy {
    Accuracy::new(
        Angle::from_radians(1e-12).expect("positive finite tolerance"),
        AccuracyEvidence::Conformance,
        "IAU SOFA validation vectors, issue 2023-10-11",
        "published nut06a and pnm06a vectors",
    )
    .expect("positive conformance tolerance")
}
