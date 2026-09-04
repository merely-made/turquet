// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use std::fmt;

use turquet::apparent::ApparentBody;
use turquet::foundation::{
    Direction, Distance, EastLongitude, JulianDate, Latitude, Length, Longitude, Model, Observer,
    ScaleAwareEpoch, State, TerrestrialTime, TimeOffset, TrueEclipticEquinoxOfDate, UniversalTime1,
};
use turquet::observer::EarthOrientation;
use turquet::provider::{EarthOrientationProvider, GeocentricPositionProvider};

pub const HORIZONS_VECTORS: &str = include_str!("../vectors/altitude_crossings_horizons.tsv");
pub const HORIZONS_FIXTURE_MODEL: Model = Model::new(
    "NASA/JPL Horizons DE441 altitude and transit fixture",
    "2026-08-25",
);
pub const HORIZONS_FIXTURE_SNAPSHOT: &str =
    "Horizons API 1.2 / DE441 / altitude and transit fixture generated 2026-08-25";
pub const HORIZONS_EOP_AUTHORITY: &str = "NASA/JPL Horizons quantity 49";
pub const HORIZONS_EOP_SNAPSHOT: &str = "eop.260824.p261120; polar motion approximated as zero";

#[derive(Clone, Copy, Debug)]
pub struct HorizonsAltitudeRow {
    pub case: &'static str,
    pub longitude: f64,
    pub latitude: f64,
    pub height_meters: f64,
    pub body: ApparentBody,
    pub tt_day: f64,
    pub ut1_day: f64,
    pub ecliptic_longitude: f64,
    pub ecliptic_latitude: f64,
    pub distance_au: f64,
    pub direct_altitude: f64,
}

pub struct HorizonsAltitudeFixture {
    rows: Vec<HorizonsAltitudeRow>,
}

impl HorizonsAltitudeFixture {
    pub fn parse() -> Self {
        let mut rows = Vec::new();
        for line in HORIZONS_VECTORS.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 10, "altitude fixture column count");
            let case = match fields[0] {
                "boston_sun" => "boston_sun",
                "tromso_sun_empty" => "tromso_sun_empty",
                _ => continue,
            };
            let coordinates: Vec<f64> = fields[1]
                .split(',')
                .map(|field| field.parse().expect("site coordinate"))
                .collect();
            assert_eq!(coordinates.len(), 3, "site coordinate count");
            let utc_day: f64 = fields[2].parse().expect("UTC Julian day");
            let utc = ScaleAwareEpoch::from_jde_utc(utc_day);
            let tt = JulianDate::<TerrestrialTime>::from_epoch(utc);
            let dut1: f64 = fields[9].parse().expect("DUT1 seconds");
            let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
                utc,
                TimeOffset::from_seconds(dut1).expect("finite DUT1"),
            );
            rows.push(HorizonsAltitudeRow {
                case,
                longitude: coordinates[0],
                latitude: coordinates[1],
                height_meters: coordinates[2] * 1_000.0,
                body: match fields[3] {
                    "sun" => ApparentBody::Sun,
                    other => panic!("unexpected selected fixture body {}", other),
                },
                tt_day: tt.day(),
                ut1_day: ut1.day(),
                ecliptic_longitude: fields[4].parse().expect("ecliptic longitude"),
                ecliptic_latitude: fields[5].parse().expect("ecliptic latitude"),
                distance_au: fields[6].parse().expect("range AU"),
                direct_altitude: fields[7].parse().expect("direct altitude"),
            });
        }
        Self { rows }
    }

    pub fn case_rows(&self, case: &str) -> Vec<&HorizonsAltitudeRow> {
        self.rows.iter().filter(|row| row.case == case).collect()
    }

    pub fn for_case<'a>(&'a self, case: &'a str) -> HorizonsAltitudeCase<'a> {
        HorizonsAltitudeCase {
            fixture: self,
            case,
        }
    }

    pub fn observer(&self, case: &str) -> Observer {
        let row = self
            .case_rows(case)
            .into_iter()
            .next()
            .expect("fixture case has rows");
        Observer::new(
            EastLongitude::from_degrees(row.longitude).expect("fixture longitude"),
            Latitude::from_degrees(row.latitude).expect("fixture latitude"),
            Length::from_meters(row.height_meters).expect("fixture height"),
        )
    }
}

pub struct HorizonsAltitudeCase<'a> {
    fixture: &'a HorizonsAltitudeFixture,
    case: &'a str,
}

impl<'a> HorizonsAltitudeCase<'a> {
    fn bracket(
        &self,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<(&HorizonsAltitudeRow, &HorizonsAltitudeRow, f64), FixtureError> {
        for pair in self.fixture.case_rows(self.case).windows(2) {
            let left = pair[0];
            let right = pair[1];
            if epoch.day() >= left.tt_day - 1e-8 && epoch.day() <= right.tt_day + 1e-8 {
                let fraction = ((epoch.day() - left.tt_day) / (right.tt_day - left.tt_day))
                    .max(0.0)
                    .min(1.0);
                return Ok((left, right, fraction));
            }
        }
        Err(FixtureError::OutsideFixture)
    }

    pub fn rows(&self) -> Vec<&HorizonsAltitudeRow> {
        self.fixture.case_rows(self.case)
    }

    pub fn direct_threshold_crossings(
        &self,
        threshold_degrees: f64,
    ) -> Vec<DirectAltitudeCrossing> {
        let mut crossings = Vec::new();
        for pair in self.rows().windows(2) {
            let left = pair[0];
            let right = pair[1];
            let left_value = left.direct_altitude - threshold_degrees;
            let right_value = right.direct_altitude - threshold_degrees;
            if left_value.signum() == right_value.signum() {
                continue;
            }
            let fraction = -left_value / (right_value - left_value);
            crossings.push(DirectAltitudeCrossing {
                kind: if right_value > 0.0 {
                    DirectAltitudeCrossingKind::Ascending
                } else {
                    DirectAltitudeCrossingKind::Descending
                },
                tt_day: left.tt_day + fraction * (right.tt_day - left.tt_day),
            });
        }
        crossings
    }
}

impl<'a> GeocentricPositionProvider for HorizonsAltitudeCase<'a> {
    type Error = FixtureError;

    fn model(&self) -> Model {
        HORIZONS_FIXTURE_MODEL
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some(HORIZONS_FIXTURE_SNAPSHOT)
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let (left, right, fraction) = self.bracket(epoch)?;
        if left.body != body || right.body != body {
            return Err(FixtureError::UnsupportedBody);
        }
        let longitude = left.ecliptic_longitude
            + fraction * signed_degrees(right.ecliptic_longitude - left.ecliptic_longitude);
        let latitude =
            left.ecliptic_latitude + fraction * (right.ecliptic_latitude - left.ecliptic_latitude);
        let distance = left.distance_au + fraction * (right.distance_au - left.distance_au);
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).expect("fixture longitude"),
                Latitude::from_degrees(latitude).expect("fixture latitude"),
            ),
            Distance::from_astronomical_units(distance).expect("fixture distance"),
        ))
    }
}

impl<'a> EarthOrientationProvider for HorizonsAltitudeCase<'a> {
    type Error = FixtureError;

    fn authority(&self) -> &str {
        HORIZONS_EOP_AUTHORITY
    }

    fn data_snapshot(&self) -> &str {
        HORIZONS_EOP_SNAPSHOT
    }

    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        let (left, right, fraction) = self.bracket(epoch)?;
        let ut1_day = left.ut1_day + fraction * (right.ut1_day - left.ut1_day);
        Ok(EarthOrientation::zero_polar_motion(
            JulianDate::from_julian_day(ut1_day).expect("fixture UT1"),
            HORIZONS_EOP_AUTHORITY,
            HORIZONS_EOP_SNAPSHOT,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectAltitudeCrossingKind {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug)]
pub struct DirectAltitudeCrossing {
    pub kind: DirectAltitudeCrossingKind,
    pub tt_day: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureError {
    UnsupportedBody,
    OutsideFixture,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FixtureError::UnsupportedBody => formatter.write_str("unsupported fixture body"),
            FixtureError::OutsideFixture => formatter.write_str("epoch outside fixture"),
        }
    }
}

impl std::error::Error for FixtureError {}

fn signed_degrees(delta: f64) -> f64 {
    (delta + 180.0).rem_euclid(360.0) - 180.0
}
