// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use turquet::apparent::{ANALYTICAL_APPARENT, ApparentBody};
use turquet::events::{
    EclipseCandidateGeometry, EclipseCandidateKind, EventError, LongitudeMotion,
    LunarEclipseContactKind, LunarEclipseKind, LunarEclipseSearch, LunarEclipseSearchError,
    LunarPhase, MAX_CONJUNCTION_STEP_DAYS, MAX_LUNAR_ECLIPSE_CIRCUMSTANCE_SPAN_DAYS,
    MAX_STATION_VELOCITY_SPAN_DAYS, SPHERICAL_ECLIPSE_GEOMETRY, SearchWindow, SearchWindowError,
    StationSearch, StationSearchError, eclipse_candidates, ecliptic_longitude_conjunctions,
    ecliptic_longitude_lunar_phases, ecliptic_longitude_stations, lunar_eclipse_circumstances,
};
use turquet::foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, ScaleAwareEpoch, State,
    TerrestrialTime, TrueEclipticEquinoxOfDate,
};
use turquet::provider::{AnalyticalEphemeris, GeocentricPositionProvider};

const VECTORS: &str = include_str!("vectors/eclipse_conjunction_horizons.tsv");
const STATION_VECTORS: &str = include_str!("vectors/mercury_station_horizons.tsv");
const PHASE_VECTORS: &str = include_str!("vectors/lunar_phases_horizons.tsv");
const ECLIPSE_GEOMETRY_VECTORS: &str = include_str!("vectors/eclipse_geometry_horizons.tsv");
const LUNAR_ECLIPSE_CIRCUMSTANCE_VECTORS: &str =
    include_str!("vectors/lunar_eclipse_circumstances_horizons.tsv");
const HORIZONS_FIXTURE: Model = Model::new("NASA/JPL Horizons DE441 fixture", "2026-08-23");

#[derive(Clone, Copy)]
struct FixtureRow {
    body: ApparentBody,
    tt_day: f64,
    longitude_degrees: f64,
    latitude_degrees: f64,
    distance_au: f64,
}

struct HorizonsFixtureProvider {
    rows: Vec<FixtureRow>,
    snapshot: &'static str,
}

impl HorizonsFixtureProvider {
    fn eclipse() -> Self {
        Self::from_vectors(
            VECTORS,
            6,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "reference: NASA GSFC 2024-04-08 ecliptic conjunction 18:20:46.8 UT",
                "fetch_horizons_conjunction_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / generated 2026-08-23",
        )
    }

    fn mercury_station() -> Self {
        Self::from_vectors(
            STATION_VECTORS,
            37,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "hourly apparent Mercury positions",
                "fetch_horizons_station_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / station fixture generated 2026-08-24",
        )
    }

    fn lunar_phases() -> Self {
        Self::from_vectors(
            PHASE_VECTORS,
            24,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "NASA GSFC April 2024 quarter-phase minute",
                "fetch_horizons_phase_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / phase fixture generated 2026-08-24",
        )
    }

    fn eclipse_geometry() -> Self {
        Self::from_vectors(
            ECLIPSE_GEOMETRY_VECTORS,
            42,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "2024-03-25 penumbral lunar",
                "09-18 partial lunar",
                "2025-03-14 total lunar",
                "03-29 partial solar",
                "fetch_horizons_eclipse_vectors.ps1",
            ],
            "Horizons API 1.2 / DE441 / eclipse geometry fixture generated 2026-08-24",
        )
    }

    fn lunar_eclipse_circumstances() -> Self {
        Self::from_vectors(
            LUNAR_ECLIPSE_CIRCUMSTANCE_VECTORS,
            162,
            &[
                "oracle: NASA/JPL Horizons API 1.2, DE441",
                "fifteen-minute Sun and Moon states",
                "NASA Danjon shadow enlargement",
                "fetch_horizons_lunar_eclipse_circumstances.ps1",
            ],
            "Horizons API 1.2 / DE441 / lunar eclipse circumstance fixture generated 2026-08-24",
        )
    }

    fn from_vectors(
        vectors: &str,
        expected_rows: usize,
        expected_headers: &[&str],
        snapshot: &'static str,
    ) -> Self {
        for expected in expected_headers {
            assert!(
                vectors.lines().any(|line| line.contains(expected)),
                "Horizons fixture header must record {}",
                expected
            );
        }
        let mut rows = Vec::new();
        for line in vectors.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 5);
            let body = match fields[0] {
                "sun" => ApparentBody::Sun,
                "moon" => ApparentBody::Moon,
                "mercury" => ApparentBody::Mercury,
                _ => panic!("unknown fixture body {}", fields[0]),
            };
            let utc_day = fields[1].parse::<f64>().expect("UTC Julian day");
            let epoch = ScaleAwareEpoch::from_jde_utc(utc_day);
            rows.push(FixtureRow {
                body,
                tt_day: JulianDate::<TerrestrialTime>::from_epoch(epoch).day(),
                longitude_degrees: fields[2].parse().expect("longitude"),
                latitude_degrees: fields[3].parse().expect("latitude"),
                distance_au: fields[4].parse().expect("distance"),
            });
        }
        assert_eq!(rows.len(), expected_rows);
        Self { rows, snapshot }
    }
}

#[derive(Clone, Copy, Debug)]
enum FixtureError {
    UnsupportedBody,
    OutsideFixture,
}

impl GeocentricPositionProvider for HorizonsFixtureProvider {
    type Error = FixtureError;

    fn model(&self) -> Model {
        HORIZONS_FIXTURE
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some(self.snapshot)
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let rows: Vec<&FixtureRow> = self.rows.iter().filter(|row| row.body == body).collect();
        if rows.is_empty() {
            return Err(FixtureError::UnsupportedBody);
        }
        for pair in rows.windows(2) {
            let left = pair[0];
            let right = pair[1];
            let rounding_margin_days = 1e-8;
            if epoch.day() >= left.tt_day - rounding_margin_days
                && epoch.day() <= right.tt_day + rounding_margin_days
            {
                let fraction = ((epoch.day() - left.tt_day) / (right.tt_day - left.tt_day))
                    .max(0.0)
                    .min(1.0);
                let longitude = left.longitude_degrees
                    + fraction * signed_degrees(right.longitude_degrees - left.longitude_degrees);
                let latitude = left.latitude_degrees
                    + fraction * (right.latitude_degrees - left.latitude_degrees);
                let distance = left.distance_au + fraction * (right.distance_au - left.distance_au);
                return Ok(State::new(
                    epoch,
                    Direction::new(
                        Longitude::from_degrees(longitude).expect("fixture longitude"),
                        Latitude::from_degrees(latitude).expect("fixture latitude"),
                    ),
                    Distance::from_astronomical_units(distance).expect("fixture distance"),
                ));
            }
        }
        Err(FixtureError::OutsideFixture)
    }
}

struct LinearProvider {
    origin: JulianDate<TerrestrialTime>,
    initial_degrees: f64,
    degrees_per_day: f64,
}

struct QuadraticProvider {
    origin: JulianDate<TerrestrialTime>,
    station_day: f64,
}

struct ZeroDistanceProvider {
    origin: JulianDate<TerrestrialTime>,
}

impl GeocentricPositionProvider for ZeroDistanceProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("zero-distance test provider", "1")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let longitude = if body == ApparentBody::Moon {
            350.0 + 20.0 * (epoch.day() - self.origin.day())
        } else {
            0.0
        };
        let distance = if body == ApparentBody::Moon {
            Distance::from_kilometers(0.0).unwrap()
        } else {
            Distance::from_astronomical_units(1.0).unwrap()
        };
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).unwrap(),
                Latitude::from_degrees(0.0).unwrap(),
            ),
            distance,
        ))
    }
}

impl GeocentricPositionProvider for QuadraticProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("quadratic test provider", "1")
    }

    fn position(
        &self,
        _body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let offset = epoch.day() - self.origin.day() - self.station_day;
        let longitude = 359.9 + 20.0 * offset * offset;
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).unwrap(),
                Latitude::from_degrees(0.0).unwrap(),
            ),
            Distance::from_astronomical_units(1.0).unwrap(),
        ))
    }
}

impl GeocentricPositionProvider for LinearProvider {
    type Error = ();

    fn model(&self) -> Model {
        Model::new("linear test provider", "1")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let longitude = if body == ApparentBody::Moon {
            self.initial_degrees + self.degrees_per_day * (epoch.day() - self.origin.day())
        } else {
            0.0
        };
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).unwrap(),
                Latitude::from_degrees(0.0).unwrap(),
            ),
            Distance::from_astronomical_units(1.0).unwrap(),
        ))
    }
}

fn tt_from_utc(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
) -> JulianDate<TerrestrialTime> {
    let whole = second.trunc() as u8;
    let nanos = ((second - f64::from(whole)) * 1e9).round() as u32;
    JulianDate::from_epoch(ScaleAwareEpoch::from_gregorian_utc(
        year, month, day, hour, minute, whole, nanos,
    ))
}

fn tt_from_parts(parts: (i32, u8, u8, u8, u8, f64)) -> JulianDate<TerrestrialTime> {
    tt_from_utc(parts.0, parts.1, parts.2, parts.3, parts.4, parts.5)
}

fn seconds_apart(first: JulianDate<TerrestrialTime>, second: JulianDate<TerrestrialTime>) -> f64 {
    (first.day() - second.day()).abs() * 86_400.0
}

fn signed_seconds(first: JulianDate<TerrestrialTime>, second: JulianDate<TerrestrialTime>) -> f64 {
    (first.day() - second.day()) * 86_400.0
}

fn signed_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

fn print_eclipse_geometry(
    year: i32,
    month: u8,
    day: u8,
    fixture: EclipseCandidateGeometry,
    analytical: EclipseCandidateGeometry,
) {
    match (fixture, analytical) {
        (
            EclipseCandidateGeometry::Solar {
                center_separation: fixture_separation,
                sun_angular_radius: fixture_sun,
                moon_angular_radius: fixture_moon,
                observer_parallax_allowance: fixture_parallax,
            },
            EclipseCandidateGeometry::Solar {
                center_separation: analytical_separation,
                sun_angular_radius: analytical_sun,
                moon_angular_radius: analytical_moon,
                observer_parallax_allowance: analytical_parallax,
            },
        ) => {
            eprintln!(
                "{year:04}-{month:02}-{day:02} solar: separation {:.6}/{:.6} deg; disk radii {:.6}+{:.6}/{:.6}+{:.6} deg; parallax allowance {:.6}/{:.6} deg",
                fixture_separation.degrees(),
                analytical_separation.degrees(),
                fixture_sun.degrees(),
                fixture_moon.degrees(),
                analytical_sun.degrees(),
                analytical_moon.degrees(),
                fixture_parallax.degrees(),
                analytical_parallax.degrees(),
            );
            assert!((fixture_separation.degrees() - analytical_separation.degrees()).abs() < 0.01);
            assert!((fixture_sun.degrees() - analytical_sun.degrees()).abs() < 0.001);
            assert!((fixture_moon.degrees() - analytical_moon.degrees()).abs() < 0.001);
            assert!((fixture_parallax.degrees() - analytical_parallax.degrees()).abs() < 0.001);
            if (year, month, day) == (2025, 3, 29) {
                assert!(
                    fixture_separation.radians() > fixture_sun.radians() + fixture_moon.radians()
                );
                assert!(
                    fixture_separation.radians()
                        <= fixture_sun.radians()
                            + fixture_moon.radians()
                            + fixture_parallax.radians()
                );
            }
        },
        (
            EclipseCandidateGeometry::Lunar {
                shadow_axis_separation: fixture_axis,
                moon_angular_radius: fixture_moon,
                umbra_angular_radius: fixture_umbra,
                penumbra_angular_radius: fixture_penumbra,
            },
            EclipseCandidateGeometry::Lunar {
                shadow_axis_separation: analytical_axis,
                moon_angular_radius: analytical_moon,
                umbra_angular_radius: analytical_umbra,
                penumbra_angular_radius: analytical_penumbra,
            },
        ) => {
            eprintln!(
                "{year:04}-{month:02}-{day:02} lunar: axis {:.6}/{:.6} deg; Moon radius {:.6}/{:.6} deg; umbra {:.6}/{:.6} deg; penumbra {:.6}/{:.6} deg",
                fixture_axis.degrees(),
                analytical_axis.degrees(),
                fixture_moon.degrees(),
                analytical_moon.degrees(),
                fixture_umbra.degrees(),
                analytical_umbra.degrees(),
                fixture_penumbra.degrees(),
                analytical_penumbra.degrees(),
            );
            assert!((fixture_axis.degrees() - analytical_axis.degrees()).abs() < 0.01);
            assert!((fixture_moon.degrees() - analytical_moon.degrees()).abs() < 0.001);
            assert!((fixture_umbra.degrees() - analytical_umbra.degrees()).abs() < 0.001);
            assert!((fixture_penumbra.degrees() - analytical_penumbra.degrees()).abs() < 0.001);
        },
        _ => panic!("providers returned different eclipse geometry variants"),
    }
}

#[path = "events/eclipses.rs"]
mod eclipses;

#[path = "events/searches.rs"]
mod searches;
