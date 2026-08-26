// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use std::convert::Infallible;

use turquet::apparent::ApparentBody;
use turquet::foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, ScaleAwareEpoch, State,
    TerrestrialTime, TrueEclipticEquinoxOfDate,
};
use turquet::provider::GeocentricPositionProvider;

pub const HORIZONS_FIXTURE: Model = Model::new("NASA/JPL Horizons DE441 fixture", "2026-08-26");
const VECTORS: &str = include_str!("../vectors/distance_extrema_horizons.tsv");

#[derive(Clone, Copy)]
pub struct RangeRow {
    pub epoch: JulianDate<TerrestrialTime>,
    pub range: Distance,
    longitude_degrees: f64,
    latitude_degrees: f64,
}

pub struct HorizonsRangeCase {
    pub body: ApparentBody,
    pub rows: Vec<RangeRow>,
}

impl HorizonsRangeCase {
    pub fn provider(&self) -> HorizonsRangeProvider<'_> {
        HorizonsRangeProvider {
            body: self.body,
            rows: &self.rows,
        }
    }
}

pub fn horizons_case(name: &str) -> HorizonsRangeCase {
    let (body, start_epoch, start_jd_utc) = case_origin(name);
    let rows: Vec<RangeRow> = VECTORS
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 8, "expected eight Horizons range fields");
            if fields[0] != name {
                return None;
            }
            assert_eq!(body_name(body), fields[1], "fixture body");
            let jd_utc: f64 = fields[2].parse().expect("Horizons UTC JD");
            let epoch = start_epoch
                .offset_days(jd_utc - start_jd_utc)
                .expect("finite fixture epoch");
            Some(RangeRow {
                epoch,
                range: Distance::from_astronomical_units(
                    fields[3].parse().expect("Horizons range AU"),
                )
                .expect("positive Horizons range"),
                longitude_degrees: fields[5].parse().expect("Horizons ecliptic longitude"),
                latitude_degrees: fields[6].parse().expect("Horizons ecliptic latitude"),
            })
        })
        .collect();
    assert!(rows.len() >= 3, "fixture case must have three rows");
    HorizonsRangeCase { body, rows }
}

pub fn parabolic_reference(
    case: &HorizonsRangeCase,
    maximum: bool,
) -> (JulianDate<TerrestrialTime>, Distance) {
    let index = case
        .rows
        .iter()
        .enumerate()
        .skip(1)
        .take(case.rows.len() - 2)
        .reduce(|best, candidate| {
            let best_value = best.1.range.meters();
            let candidate_value = candidate.1.range.meters();
            if (maximum && candidate_value > best_value)
                || (!maximum && candidate_value < best_value)
            {
                candidate
            } else {
                best
            }
        })
        .expect("interior range row");
    let previous = case.rows[index.0 - 1];
    let middle = *index.1;
    let next = case.rows[index.0 + 1];
    let step_days = next.epoch.day() - middle.epoch.day();
    assert!((middle.epoch.day() - previous.epoch.day() - step_days).abs() < 1e-12);
    let left = previous.range.meters();
    let center = middle.range.meters();
    let right = next.range.meters();
    let curvature = left - 2.0 * center + right;
    assert!(curvature != 0.0, "reference range curvature");
    let offset_days = 0.5 * (left - right) * step_days / curvature;
    let vertex = center - (left - right) * (left - right) / (8.0 * curvature);
    (
        middle
            .epoch
            .offset_days(offset_days)
            .expect("finite parabolic vertex"),
        Distance::from_meters(vertex).expect("positive parabolic range"),
    )
}

pub struct HorizonsRangeProvider<'a> {
    body: ApparentBody,
    rows: &'a [RangeRow],
}

impl<'a> GeocentricPositionProvider for HorizonsRangeProvider<'a> {
    type Error = Infallible;

    fn model(&self) -> Model {
        HORIZONS_FIXTURE
    }

    fn data_snapshot(&self) -> Option<&str> {
        Some("Horizons API 1.2 / DE441 / generated 2026-08-26")
    }

    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        assert_eq!(body, self.body, "fixture body request");
        let (left, right) = self
            .rows
            .windows(2)
            .find_map(|pair| {
                if epoch.day() >= pair[0].epoch.day() && epoch.day() <= pair[1].epoch.day() {
                    Some((pair[0], pair[1]))
                } else {
                    None
                }
            })
            .expect("fixture request within captured span");
        let span_days = right.epoch.day() - left.epoch.day();
        let fraction = (epoch.day() - left.epoch.day()) / span_days;
        let longitude = left.longitude_degrees
            + fraction * signed_degrees(right.longitude_degrees - left.longitude_degrees);
        let latitude =
            left.latitude_degrees + fraction * (right.latitude_degrees - left.latitude_degrees);
        let range = left.range.meters() + fraction * (right.range.meters() - left.range.meters());
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_degrees(longitude).expect("finite fixture longitude"),
                Latitude::from_degrees(latitude).expect("finite fixture latitude"),
            ),
            Distance::from_meters(range).expect("positive fixture range"),
        ))
    }
}

fn case_origin(name: &str) -> (ApparentBody, JulianDate<TerrestrialTime>, f64) {
    match name {
        "moon_perigee" => (
            ApparentBody::Moon,
            tt_from_utc(2024, 4, 7, 0, 0),
            2_460_407.5,
        ),
        "moon_apogee" => (
            ApparentBody::Moon,
            tt_from_utc(2024, 4, 19, 0, 0),
            2_460_419.5,
        ),
        "mars_close" => (
            ApparentBody::Mars,
            tt_from_utc(2022, 11, 25, 0, 0),
            2_459_908.5,
        ),
        _ => panic!("unknown Horizons distance-extremum case"),
    }
}

fn body_name(body: ApparentBody) -> &'static str {
    match body {
        ApparentBody::Moon => "moon",
        ApparentBody::Mars => "mars",
        _ => panic!("unexpected Horizons fixture body"),
    }
}

fn tt_from_utc(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> JulianDate<TerrestrialTime> {
    JulianDate::from_epoch(ScaleAwareEpoch::from_gregorian_utc(
        year, month, day, hour, minute, 0, 0,
    ))
}

fn signed_degrees(value: f64) -> f64 {
    (value + 180.0).rem_euclid(360.0) - 180.0
}
