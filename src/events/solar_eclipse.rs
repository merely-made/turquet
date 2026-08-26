// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Observer-relative, atmosphere-free solar-eclipse circumstances.
//!
//! This module solves local disk contacts after the geocentric phase search.
//! It does not select refraction, terrain, obstructions, weather, civil time,
//! or an optical or human visibility policy. The retained visibility state is
//! the physical upper solar limb against the airless WGS84 horizon at greatest
//! eclipse only; a general visible-window search remains a separate slice.

use apparent::ApparentBody;
use foundation::{Angle, Distance, JulianDate, Observer, TerrestrialTime};
use observer::{ObserverTransform, AIRLESS_TOPOCENTRIC_TRANSFORM};
use provider::{EarthOrientationProvider, GeocentricPositionProvider};

use super::{
    ecliptic_longitude_lunar_phases, AltitudeCrossingError, EventInterval, LunarPhase,
    ScalarSample, MEAN_LUNAR_RADIUS_KM, NOMINAL_SOLAR_RADIUS_KM,
};

mod types;
pub use self::types::{
    LocalSolarEclipseCircumstances, LocalSolarEclipseContact, LocalSolarEclipseContactKind,
    LocalSolarEclipseError, LocalSolarEclipseGeometry, LocalSolarEclipseKind,
    LocalSolarEclipseSearch, LocalSolarEclipseSearchError, LocalSolarEclipseVisibility,
    LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCES, MAX_LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS,
};

#[derive(Clone, Copy)]
struct SolarGeometry {
    public: LocalSolarEclipseGeometry,
    outer_clearance: f64,
    central_clearance: f64,
}

#[derive(Clone, Copy)]
enum ContactBoundary {
    External,
    Central,
}

impl ContactBoundary {
    fn clearance(self, geometry: SolarGeometry) -> f64 {
        match self {
            ContactBoundary::External => geometry.outer_clearance,
            ContactBoundary::Central => geometry.central_clearance,
        }
    }
}

/// Find local solar-eclipse contacts around every new moon in a TT window.
///
/// A result establishes strict topocentric disk overlap at the selected
/// observer. A local grazing tangency has no nonzero-duration overlap and is
/// omitted. Contacts and greatest eclipse are bounded TT intervals. The
/// atmosphere-free visibility classification is intentionally limited to the
/// solar upper limb at greatest eclipse; it is not a visible-window result.
pub fn local_solar_eclipse_circumstances<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    search: LocalSolarEclipseSearch,
) -> Result<Vec<LocalSolarEclipseCircumstances>, LocalSolarEclipseError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let phases = ecliptic_longitude_lunar_phases(positions, search.phase_window())
        .map_err(LocalSolarEclipseError::Phase)?;
    let orientation_authority = earth_orientation.authority().to_owned();
    let orientation_snapshot = earth_orientation.data_snapshot().to_owned();
    let mut events = Vec::new();

    for phase in phases {
        if phase.phase() != LunarPhase::NewMoon {
            continue;
        }
        let phase_interval = phase.interval();
        let phase_epoch = phase_interval.midpoint();
        let half_span_days = search.circumstance_span_days() / 2.0;
        let start = JulianDate::from_julian_day(phase_epoch.day() - half_span_days)
            .expect("a bounded TT phase span has a finite start");
        let end = JulianDate::from_julian_day(phase_epoch.day() + half_span_days)
            .expect("a bounded TT phase span has a finite end");
        let start_geometry = geometry_at(
            positions,
            earth_orientation,
            observer,
            start,
            &orientation_authority,
            &orientation_snapshot,
        )?;
        let end_geometry = geometry_at(
            positions,
            earth_orientation,
            observer,
            end,
            &orientation_authority,
            &orientation_snapshot,
        )?;
        let greatest_interval = refine_greatest(
            positions,
            earth_orientation,
            observer,
            start,
            end,
            search.phase_window().tolerance_days(),
            &orientation_authority,
            &orientation_snapshot,
        )?;
        let greatest_geometry = geometry_at(
            positions,
            earth_orientation,
            observer,
            greatest_interval.midpoint(),
            &orientation_authority,
            &orientation_snapshot,
        )?;

        if greatest_geometry.outer_clearance >= 0.0 {
            continue;
        }
        if start_geometry.outer_clearance <= 0.0 || end_geometry.outer_clearance <= 0.0 {
            return Err(LocalSolarEclipseError::CircumstanceSpanTooShort {
                phase_epoch,
                span_days: search.circumstance_span_days(),
            });
        }

        let greatest_epoch = greatest_interval.midpoint();
        let mut contacts = Vec::with_capacity(4);
        contacts.push(LocalSolarEclipseContact {
            kind: LocalSolarEclipseContactKind::First,
            interval: refine_contact(
                positions,
                earth_orientation,
                observer,
                ScalarSample {
                    epoch: start,
                    value: start_geometry.outer_clearance,
                },
                ScalarSample {
                    epoch: greatest_epoch,
                    value: greatest_geometry.outer_clearance,
                },
                ContactBoundary::External,
                search.phase_window().tolerance_days(),
                &orientation_authority,
                &orientation_snapshot,
            )?,
        });

        let central = greatest_geometry.central_clearance < 0.0;
        if central {
            if start_geometry.central_clearance <= 0.0 || end_geometry.central_clearance <= 0.0 {
                return Err(LocalSolarEclipseError::CircumstanceSpanTooShort {
                    phase_epoch,
                    span_days: search.circumstance_span_days(),
                });
            }
            contacts.push(LocalSolarEclipseContact {
                kind: LocalSolarEclipseContactKind::Second,
                interval: refine_contact(
                    positions,
                    earth_orientation,
                    observer,
                    ScalarSample {
                        epoch: start,
                        value: start_geometry.central_clearance,
                    },
                    ScalarSample {
                        epoch: greatest_epoch,
                        value: greatest_geometry.central_clearance,
                    },
                    ContactBoundary::Central,
                    search.phase_window().tolerance_days(),
                    &orientation_authority,
                    &orientation_snapshot,
                )?,
            });
        }

        if central {
            contacts.push(LocalSolarEclipseContact {
                kind: LocalSolarEclipseContactKind::Third,
                interval: refine_contact(
                    positions,
                    earth_orientation,
                    observer,
                    ScalarSample {
                        epoch: greatest_epoch,
                        value: greatest_geometry.central_clearance,
                    },
                    ScalarSample {
                        epoch: end,
                        value: end_geometry.central_clearance,
                    },
                    ContactBoundary::Central,
                    search.phase_window().tolerance_days(),
                    &orientation_authority,
                    &orientation_snapshot,
                )?,
            });
        }

        contacts.push(LocalSolarEclipseContact {
            kind: LocalSolarEclipseContactKind::Fourth,
            interval: refine_contact(
                positions,
                earth_orientation,
                observer,
                ScalarSample {
                    epoch: greatest_epoch,
                    value: greatest_geometry.outer_clearance,
                },
                ScalarSample {
                    epoch: end,
                    value: end_geometry.outer_clearance,
                },
                ContactBoundary::External,
                search.phase_window().tolerance_days(),
                &orientation_authority,
                &orientation_snapshot,
            )?,
        });

        let kind = if !central {
            LocalSolarEclipseKind::Partial
        } else if greatest_geometry.public.moon_angular_radius
            >= greatest_geometry.public.sun_angular_radius
        {
            LocalSolarEclipseKind::Total
        } else {
            LocalSolarEclipseKind::Annular
        };
        let visibility = if greatest_geometry.public.sun_upper_limb_altitude.radians() > 0.0 {
            LocalSolarEclipseVisibility::SunUpperLimbAboveAirlessHorizonAtGreatest
        } else {
            LocalSolarEclipseVisibility::SunUpperLimbAtOrBelowAirlessHorizonAtGreatest
        };
        events.push(LocalSolarEclipseCircumstances {
            kind,
            phase_interval,
            greatest_interval,
            greatest_geometry: greatest_geometry.public,
            contacts,
            search,
            visibility,
            observer,
            geometry_model: LOCAL_SOLAR_ECLIPSE_CIRCUMSTANCES,
            provider_model: positions.model(),
            provider_snapshot: positions.data_snapshot().map(str::to_owned),
            transform_model: AIRLESS_TOPOCENTRIC_TRANSFORM,
            earth_orientation_authority: orientation_authority.clone(),
            earth_orientation_snapshot: orientation_snapshot.clone(),
        });
    }

    Ok(events)
}

fn refine_greatest<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    mut start: JulianDate<TerrestrialTime>,
    mut end: JulianDate<TerrestrialTime>,
    tolerance_days: f64,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<EventInterval, LocalSolarEclipseError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    while end.day() - start.day() > tolerance_days {
        let left = JulianDate::from_julian_day((2.0 * start.day() + end.day()) / 3.0)
            .expect("a finite greatest-eclipse bracket has a finite sample");
        let right = JulianDate::from_julian_day((start.day() + 2.0 * end.day()) / 3.0)
            .expect("a finite greatest-eclipse bracket has a finite sample");
        let left_value = geometry_at(
            positions,
            earth_orientation,
            observer,
            left,
            expected_authority,
            expected_snapshot,
        )?
        .outer_clearance;
        let right_value = geometry_at(
            positions,
            earth_orientation,
            observer,
            right,
            expected_authority,
            expected_snapshot,
        )?
        .outer_clearance;
        if left_value <= right_value {
            end = right;
        } else {
            start = left;
        }
    }
    Ok(EventInterval { start, end })
}

fn refine_contact<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    left: ScalarSample,
    right: ScalarSample,
    boundary: ContactBoundary,
    tolerance_days: f64,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<EventInterval, LocalSolarEclipseError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    super::refine_scalar_root(left, right, tolerance_days, |epoch| {
        geometry_at(
            positions,
            earth_orientation,
            observer,
            epoch,
            expected_authority,
            expected_snapshot,
        )
        .map(|geometry| boundary.clearance(geometry))
    })
}

fn geometry_at<P, E>(
    positions: &P,
    earth_orientation: &E,
    observer: Observer,
    epoch: JulianDate<TerrestrialTime>,
    expected_authority: &str,
    expected_snapshot: &str,
) -> Result<SolarGeometry, LocalSolarEclipseError<P::Error, E::Error>>
where
    P: GeocentricPositionProvider,
    E: EarthOrientationProvider,
{
    let moon = positions
        .position(ApparentBody::Moon, epoch)
        .map_err(|source| {
            LocalSolarEclipseError::Observation(AltitudeCrossingError::Position {
                body: ApparentBody::Moon,
                epoch,
                source,
            })
        })?;
    let sun = positions
        .position(ApparentBody::Sun, epoch)
        .map_err(|source| {
            LocalSolarEclipseError::Observation(AltitudeCrossingError::Position {
                body: ApparentBody::Sun,
                epoch,
                source,
            })
        })?;
    let orientation = earth_orientation.at(epoch).map_err(|source| {
        LocalSolarEclipseError::Observation(AltitudeCrossingError::EarthOrientation {
            epoch,
            source,
        })
    })?;
    let actual_authority = earth_orientation.authority();
    let actual_snapshot = earth_orientation.data_snapshot();
    if actual_authority != expected_authority
        || actual_snapshot != expected_snapshot
        || orientation.authority() != expected_authority
        || orientation.snapshot() != expected_snapshot
    {
        return Err(LocalSolarEclipseError::Observation(
            AltitudeCrossingError::EarthOrientationIdentityMismatch {
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
            },
        ));
    }
    let transform = ObserverTransform::at(epoch, orientation, observer);
    let moon = transform
        .observe(moon)
        .map_err(|source| {
            LocalSolarEclipseError::Observation(AltitudeCrossingError::Transform {
                body: ApparentBody::Moon,
                epoch,
                source,
            })
        })?
        .into_value();
    let sun = transform
        .observe(sun)
        .map_err(|source| {
            LocalSolarEclipseError::Observation(AltitudeCrossingError::Transform {
                body: ApparentBody::Sun,
                epoch,
                source,
            })
        })?
        .into_value();
    let moon_angular_radius = angular_radius(
        ApparentBody::Moon,
        MEAN_LUNAR_RADIUS_KM,
        moon.equatorial().distance(),
        epoch,
    )?;
    let sun_angular_radius = angular_radius(
        ApparentBody::Sun,
        NOMINAL_SOLAR_RADIUS_KM,
        sun.equatorial().distance(),
        epoch,
    )?;
    let center_separation = direction_separation(
        moon.equatorial().direction().to_unit_vector().components(),
        sun.equatorial().direction().to_unit_vector().components(),
    );
    let sun_center_altitude = sun.horizon().latitude().angle();
    let sun_upper_limb_altitude =
        Angle::from_radians(sun_center_altitude.radians() + sun_angular_radius.radians())
            .expect("the finite airless solar-limb altitude is a finite angle");
    let outer_clearance =
        center_separation.radians() - moon_angular_radius.radians() - sun_angular_radius.radians();
    let central_clearance = center_separation.radians()
        - (moon_angular_radius.radians() - sun_angular_radius.radians()).abs();
    Ok(SolarGeometry {
        public: LocalSolarEclipseGeometry {
            center_separation,
            sun_angular_radius,
            moon_angular_radius,
            sun_center_altitude,
            sun_upper_limb_altitude,
        },
        outer_clearance,
        central_clearance,
    })
}

fn angular_radius<P, E>(
    body: ApparentBody,
    physical_radius_kilometers: f64,
    topocentric_distance: Distance,
    epoch: JulianDate<TerrestrialTime>,
) -> Result<Angle, LocalSolarEclipseError<P, E>> {
    let physical_radius = Distance::from_kilometers(physical_radius_kilometers)
        .expect("a named solar-system physical radius is finite and positive");
    if physical_radius >= topocentric_distance {
        return Err(LocalSolarEclipseError::BodyContainsObserver {
            body,
            epoch,
            physical_radius,
            topocentric_distance,
        });
    }
    Ok(
        Angle::from_radians((physical_radius.meters() / topocentric_distance.meters()).asin())
            .expect("a physical-radius ratio below one produces a finite angle"),
    )
}

fn direction_separation(first: [f64; 3], second: [f64; 3]) -> Angle {
    let dot = first[0] * second[0] + first[1] * second[1] + first[2] * second[2];
    let cross = [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ];
    let cross_norm = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
    Angle::from_radians(cross_norm.atan2(dot))
        .expect("finite normalized topocentric directions have a finite separation")
}
