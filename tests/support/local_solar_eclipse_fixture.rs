// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use std::fmt;

use turquet::apparent::ApparentBody;
use turquet::events::SearchWindow;
use turquet::foundation::{
    Direction, Distance, EastLongitude, JulianDate, Latitude, Length, Longitude, Model, Observer,
    ScaleAwareEpoch, State, TerrestrialTime, TrueEclipticEquinoxOfDate, UniversalTime1,
};
use turquet::observer::EarthOrientation;
use turquet::provider::{EarthOrientationProvider, GeocentricPositionProvider};

pub const HORIZONS_MODEL: Model = Model::new(
    "NASA/JPL Horizons DE441 local solar eclipse fixture",
    "2026-08-26",
);
pub const HORIZONS_SNAPSHOT: &str =
    "Horizons API 1.2 / DE441 / five-minute local solar eclipse fixture generated 2026-08-26";
pub const HORIZONS_EOP_AUTHORITY: &str = "NASA/JPL Horizons quantity 49";
pub const HORIZONS_EOP_SNAPSHOT: &str = "eop.260825.p261121; polar motion approximated as zero";

const VECTORS: &str = include_str!("../vectors/local_solar_eclipse_horizons.tsv");

#[derive(Clone, Copy)]
struct Row {
    case: &'static str,
    longitude: f64,
    latitude: f64,
    height_meters: f64,
    body: ApparentBody,
    tt_day: f64,
    ut1_offset_days: f64,
    ecliptic_longitude: f64,
    ecliptic_latitude: f64,
    distance_au: f64,
    direct_altitude: f64,
}

pub struct HorizonsLocalFixture {
    rows: Vec<Row>,
}

impl HorizonsLocalFixture {
    pub fn parse() -> Self {
        for expected in &[
            "Horizons API 1.2, DE441, AIRLESS apparent observer ephemerides",
            "eop.260825.p261121",
            "fetch_horizons_local_solar_eclipse_vectors.ps1",
            "Cape Town outside-footprint control",
        ] {
            assert!(
                VECTORS.lines().any(|line| line.contains(expected)),
                "local solar eclipse fixture must retain {}",
                expected
            );
        }
        let mut rows = Vec::new();
        for line in VECTORS.lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 11, "local solar eclipse vector column count");
            let coordinates: Vec<f64> = fields[1]
                .split(',')
                .map(|field| field.parse().expect("fixture coordinate"))
                .collect();
            assert_eq!(coordinates.len(), 3, "fixture WGS84 coordinate count");
            let utc = ScaleAwareEpoch::from_jde_utc(fields[2].parse().expect("UTC Julian day"));
            let tt = JulianDate::<TerrestrialTime>::from_epoch(utc);
            let dut1_seconds: f64 = fields[10].parse().expect("DUT1 seconds");
            let ut1 = JulianDate::<UniversalTime1>::from_utc_epoch(
                utc,
                turquet::foundation::TimeOffset::from_seconds(dut1_seconds)
                    .expect("finite fixture DUT1"),
            );
            rows.push(Row {
                case: case_name(fields[0]),
                longitude: coordinates[0],
                latitude: coordinates[1],
                height_meters: coordinates[2] * 1_000.0,
                body: body(fields[3]),
                tt_day: tt.day(),
                ut1_offset_days: ut1.day() - tt.day(),
                ecliptic_longitude: fields[4].parse().expect("ecliptic longitude"),
                ecliptic_latitude: fields[5].parse().expect("ecliptic latitude"),
                distance_au: fields[6].parse().expect("geocentric range AU"),
                direct_altitude: fields[8].parse().expect("direct topocentric altitude"),
            });
        }
        assert_eq!(
            rows.len(),
            2_950,
            "five cases, two bodies, 295 samples each"
        );
        Self { rows }
    }

    pub fn case<'a>(&'a self, name: &'a str) -> HorizonsLocalCase<'a> {
        assert!(
            self.rows.iter().any(|row| row.case == name),
            "fixture case {} must exist",
            name
        );
        HorizonsLocalCase {
            fixture: self,
            name,
        }
    }
}

pub struct HorizonsLocalCase<'a> {
    fixture: &'a HorizonsLocalFixture,
    name: &'a str,
}

impl<'a> HorizonsLocalCase<'a> {
    pub fn observer(&self) -> Observer {
        let row = self.first();
        Observer::new(
            EastLongitude::from_degrees(row.longitude).expect("fixture longitude"),
            Latitude::from_degrees(row.latitude).expect("fixture latitude"),
            Length::from_meters(row.height_meters).expect("fixture height"),
        )
    }

    pub fn window(&self) -> SearchWindow {
        let rows = self.rows_for(ApparentBody::Sun);
        SearchWindow::new(
            JulianDate::from_julian_day(rows[0].tt_day).expect("fixture start"),
            JulianDate::from_julian_day(rows[rows.len() - 1].tt_day).expect("fixture end"),
            1.0 / 24.0,
            1.0 / 86_400.0,
        )
        .expect("fixture search window")
    }

    pub fn direct_altitude(&self, body: ApparentBody, epoch: JulianDate<TerrestrialTime>) -> f64 {
        self.interpolate(body, epoch, |row| row.direct_altitude)
    }

    fn first(&self) -> Row {
        self.fixture
            .rows
            .iter()
            .find(|row| row.case == self.name)
            .copied()
            .expect("known fixture case has rows")
    }

    fn rows_for(&self, body: ApparentBody) -> Vec<Row> {
        self.fixture
            .rows
            .iter()
            .filter(|row| row.case == self.name && row.body == body)
            .copied()
            .collect()
    }

    fn interpolate<F>(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
        value: F,
    ) -> f64
    where
        F: Fn(Row) -> f64,
    {
        let (left, right, fraction) = self.bracket(body, epoch).expect("fixture epoch");
        value(left) + fraction * (value(right) - value(left))
    }

    fn bracket(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<(Row, Row, f64), FixtureError> {
        let rows = self.rows_for(body);
        if rows.is_empty() {
            return Err(FixtureError::UnsupportedBody);
        }
        for pair in rows.windows(2) {
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
}

impl<'a> GeocentricPositionProvider for HorizonsLocalCase<'a> {
    type Error = FixtureError;

    fn model(&self) -> Model {
        HORIZONS_MODEL
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some(HORIZONS_SNAPSHOT)
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        let (left, right, fraction) = self.bracket(body, epoch)?;
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

impl<'a> EarthOrientationProvider for HorizonsLocalCase<'a> {
    type Error = FixtureError;

    fn authority(&self) -> &str {
        HORIZONS_EOP_AUTHORITY
    }

    fn data_snapshot(&self) -> &str {
        HORIZONS_EOP_SNAPSHOT
    }

    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, Self::Error> {
        let (left, right, fraction) = self.bracket(ApparentBody::Sun, epoch)?;
        let ut1_offset_days =
            left.ut1_offset_days + fraction * (right.ut1_offset_days - left.ut1_offset_days);
        let ut1 = JulianDate::<UniversalTime1>::from_julian_day(epoch.day() + ut1_offset_days)
            .expect("fixture UT1 epoch");
        Ok(EarthOrientation::zero_polar_motion(
            ut1,
            HORIZONS_EOP_AUTHORITY,
            HORIZONS_EOP_SNAPSHOT,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixtureError {
    UnsupportedBody,
    OutsideFixture,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(match *self {
            FixtureError::UnsupportedBody => "unsupported fixture body",
            FixtureError::OutsideFixture => "epoch outside fixture range",
        })
    }
}

impl ::std::error::Error for FixtureError {}

fn case_name(value: &str) -> &'static str {
    match value {
        "boston_partial" => "boston_partial",
        "dallas_total" => "dallas_total",
        "albuquerque_annular" => "albuquerque_annular",
        "galway_partial" => "galway_partial",
        "cape_town_control" => "cape_town_control",
        _ => panic!("unknown local solar eclipse fixture case {}", value),
    }
}

fn body(value: &str) -> ApparentBody {
    match value {
        "sun" => ApparentBody::Sun,
        "moon" => ApparentBody::Moon,
        _ => panic!("unknown local solar eclipse fixture body {}", value),
    }
}

fn signed_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}
