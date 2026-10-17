// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

extern crate turquet;

use std::cell::RefCell;
use std::error::Error;
use turquet::apparent::{self, ANALYTICAL_APPARENT, APPARENT_BODIES, ApparentBody, ApparentError};
use turquet::foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, State, TerrestrialTime,
    TrueEclipticEquinoxOfDate,
};
use turquet::motion::{
    LongitudeRate, MotionClass, MotionError, NEAR_STATION_DEGREES_PER_DAY,
    SAMPLE_HALF_INTERVAL_DAYS, analytical_longitude_motion, longitude_motion,
};
use turquet::provider::GeocentricPositionProvider;

fn tt(day: f64) -> JulianDate<TerrestrialTime> {
    JulianDate::from_julian_day(day).unwrap()
}

fn class(name: &str) -> MotionClass {
    match name {
        "Direct" => MotionClass::Direct,
        "Retrograde" => MotionClass::Retrograde,
        "NearStation" => MotionClass::NearStation,
        _ => panic!("unknown class {}", name),
    }
}

#[test]
fn retained_swiss_motion_cohort() {
    let mut count = 0;
    let mut near = 0;
    let mut wraps = 0;
    let mut worst_sampled = (0.0_f64, String::new());
    let mut worst_speed = (0.0_f64, String::new());
    let mut bodies = [false; 10];
    for line in include_str!("vectors/motion/swiss.tsv").lines() {
        if line.starts_with('#') || line.starts_with("label\t") {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 10);
        let index = APPARENT_BODIES
            .iter()
            .position(|body| body.name() == fields[1])
            .unwrap();
        let body = APPARENT_BODIES[index];
        bodies[index] = true;
        let at = tt(fields[2].parse().unwrap());
        let before: f64 = fields[3].parse().unwrap();
        let after: f64 = fields[5].parse().unwrap();
        let sampled: f64 = fields[6].parse().unwrap();
        let speed: f64 = fields[7].parse().unwrap();
        let expected = class(fields[8]);
        assert_eq!(fields[9], "260", "explicit Swiss Moshier + speed flags");
        let retained_delta = (after - before + 180.0).rem_euclid(360.0) - 180.0;
        assert!((retained_delta / 0.02 - sampled).abs() < 1e-9);
        assert_eq!(
            LongitudeRate::from_degrees_per_day(speed)
                .unwrap()
                .classification(),
            expected
        );
        assert_eq!(
            LongitudeRate::from_degrees_per_day(sampled)
                .unwrap()
                .classification(),
            expected
        );
        let got = analytical_longitude_motion(body, at).unwrap();
        let rate = got.rate().degrees_per_day();
        let label = format!("{} {}", fields[0], fields[1]);
        // A cohort comparison gate, not a universal derivative accuracy claim.
        assert!(
            (rate - sampled).abs() < 0.001,
            "{}: sampled {} got {}",
            label,
            sampled,
            rate
        );
        assert!(
            (rate - speed).abs() < 0.001,
            "{}: speed {} got {}",
            label,
            speed,
            rate
        );
        assert_eq!(got.classification(), expected, "{}; rate={}", label, rate);
        assert_eq!(got.body(), body);
        assert_eq!(got.epoch(), at);
        assert_eq!(got.model(), ANALYTICAL_APPARENT);
        assert_eq!(got.data_snapshot(), None);
        assert_eq!(got.sample_interval_days(), 0.02);
        assert_eq!(got.near_station_threshold_degrees_per_day(), 0.001);
        if (rate - sampled).abs() > worst_sampled.0 {
            worst_sampled = ((rate - sampled).abs(), label.clone());
        }
        if (rate - speed).abs() > worst_speed.0 {
            worst_speed = ((rate - speed).abs(), label.clone());
        }
        if expected == MotionClass::NearStation {
            near += 1;
        }
        if fields[0].contains("wrap") {
            assert!((after - before).abs() > 359.0);
            wraps += 1;
        }
        // Preserve the existing sign-only behavior on its original interval.
        // Near-station rows deliberately have no equivalence assertion.
        if expected != MotionClass::NearStation {
            assert_eq!(
                apparent::is_retrograde(body, at).unwrap(),
                expected == MotionClass::Retrograde,
                "{}",
                label
            );
        }
        count += 1;
    }
    assert_eq!(count, 31);
    assert_eq!(near, 3);
    assert_eq!(wraps, 2);
    assert!(bodies.iter().all(|covered| *covered));
    println!(
        "{} Swiss rows; worst sampled residual {:.9} deg/day ({}); worst Swiss-speed residual {:.9} deg/day ({})",
        count, worst_sampled.0, worst_sampled.1, worst_speed.0, worst_speed.1
    );
}

#[test]
fn finite_signed_rate_and_inclusive_near_station_policy() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(LongitudeRate::from_degrees_per_day(bad).is_err());
    }
    for rate in [-0.001, -0.0005, -0.0, 0.0, 0.0005, 0.001] {
        let value = LongitudeRate::from_degrees_per_day(rate).unwrap();
        assert_eq!(value.degrees_per_day(), rate);
        assert_eq!(value.classification(), MotionClass::NearStation);
    }
    assert_eq!(
        LongitudeRate::from_degrees_per_day(-0.001000001)
            .unwrap()
            .classification(),
        MotionClass::Retrograde
    );
    assert_eq!(
        LongitudeRate::from_degrees_per_day(0.001000001)
            .unwrap()
            .classification(),
        MotionClass::Direct
    );
    assert_eq!(SAMPLE_HALF_INTERVAL_DAYS, 0.01);
    assert_eq!(NEAR_STATION_DEGREES_PER_DAY, 0.001);
}

struct LinearProvider {
    origin: JulianDate<TerrestrialTime>,
    longitude: f64,
    rate: f64,
    fail_sample: Option<usize>,
    wrong_epoch: bool,
    equivalent_epoch: bool,
    calls: RefCell<Vec<JulianDate<TerrestrialTime>>>,
}

impl LinearProvider {
    fn new(longitude: f64, rate: f64) -> Self {
        Self {
            origin: tt(2451545.0),
            longitude,
            rate,
            fail_sample: None,
            wrong_epoch: false,
            equivalent_epoch: false,
            calls: RefCell::new(Vec::new()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SampleFailure(usize);

impl std::fmt::Display for SampleFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "fixture sample {} unavailable", self.0)
    }
}

impl Error for SampleFailure {}

impl GeocentricPositionProvider for LinearProvider {
    type Error = SampleFailure;
    fn model(&self) -> Model {
        Model::new("linear motion test", "1")
    }
    fn data_snapshot(&self) -> Option<&str> {
        Some("immutable fixture v1")
    }
    fn position(
        &self,
        body: ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, Self::Error> {
        assert_eq!(body, ApparentBody::Mercury);
        let sample = self.calls.borrow().len();
        self.calls.borrow_mut().push(epoch);
        if self.fail_sample == Some(sample) {
            return Err(SampleFailure(sample));
        }
        let elapsed = epoch.parts().1 - self.origin.parts().1;
        let longitude = Longitude::from_degrees(self.longitude + elapsed * self.rate).unwrap();
        let returned = if self.wrong_epoch {
            self.origin
        } else if self.equivalent_epoch {
            JulianDate::from_parts(epoch.parts().0 + 1.0, epoch.parts().1 - 1.0).unwrap()
        } else {
            epoch
        };
        Ok(State::new(
            returned,
            Direction::new(longitude, Latitude::from_degrees(0.0).unwrap()),
            Distance::from_astronomical_units(1.0).unwrap(),
        ))
    }
}

#[test]
fn provider_neutral_signed_wrap_samples_and_source_identity() {
    for &(longitude, rate) in &[
        (0.0, 1.5),
        (0.0, -1.5),
        (120.0, 0.0),
        (359.999, 14.0),
        (0.0, -0.0005),
        (0.0, 0.0005),
    ] {
        let provider = LinearProvider::new(longitude, rate);
        let dynamic: &dyn GeocentricPositionProvider<Error = SampleFailure> = &provider;
        let got = longitude_motion(dynamic, ApparentBody::Mercury, provider.origin).unwrap();
        assert!((got.rate().degrees_per_day() - rate).abs() < 5e-12);
        assert_eq!(
            got.classification(),
            LongitudeRate::from_degrees_per_day(rate)
                .unwrap()
                .classification()
        );
        assert_eq!(got.model(), provider.model());
        assert_eq!(got.data_snapshot(), Some("immutable fixture v1"));
        assert_eq!(
            *provider.calls.borrow(),
            vec![
                provider.origin.offset_days(-0.01).unwrap(),
                provider.origin.offset_days(0.01).unwrap()
            ]
        );
    }
}

#[test]
fn endpoint_errors_are_preserved_with_sources() {
    for failed in 0..2 {
        let mut provider = LinearProvider::new(0.0, 1.0);
        provider.fail_sample = Some(failed);
        let error =
            longitude_motion(&provider, ApparentBody::Mercury, provider.origin).unwrap_err();
        assert_eq!(error, MotionError::Provider(SampleFailure(failed)));
        assert_eq!(
            error.source().unwrap().to_string(),
            SampleFailure(failed).to_string()
        );
        assert_eq!(provider.calls.borrow().len(), failed + 1);
    }
}

#[test]
fn wrong_sample_epoch_is_rejected_and_equivalent_parts_are_accepted() {
    let mut provider = LinearProvider::new(0.0, 1.0);
    provider.wrong_epoch = true;
    assert!(matches!(
        longitude_motion(&provider, ApparentBody::Mercury, provider.origin),
        Err(MotionError::SampleEpochMismatch { .. })
    ));
    provider.wrong_epoch = false;
    provider.equivalent_epoch = true;
    assert!(longitude_motion(&provider, ApparentBody::Mercury, provider.origin).is_ok());
}

#[test]
fn unresolvable_epochs_and_ambiguous_half_turn_are_errors() {
    let provider = LinearProvider::new(0.0, 1.0);
    for at in [
        tt(1e100),
        JulianDate::from_parts(f64::MAX, f64::MAX).unwrap(),
    ] {
        assert_eq!(
            longitude_motion(&provider, ApparentBody::Mercury, at),
            Err(MotionError::UnresolvableInterval)
        );
    }
    assert!(provider.calls.borrow().is_empty());
    let provider = LinearProvider::new(90.0, 9000.0);
    assert_eq!(
        longitude_motion(&provider, ApparentBody::Mercury, provider.origin),
        Err(MotionError::AmbiguousHalfTurn)
    );
}

#[test]
fn pluto_range_applies_to_both_samples() {
    for day in [
        2451545.0 - 115.0 * 365.25,
        2451545.0 + 99.0 * 365.25,
        2451545.0 + 150.0 * 365.25,
    ] {
        assert!(matches!(
            analytical_longitude_motion(ApparentBody::Pluto, tt(day)),
            Err(MotionError::Provider(ApparentError::OutsideSeriesRange {
                body: "pluto",
                ..
            }))
        ));
    }
    assert!(analytical_longitude_motion(ApparentBody::Pluto, tt(2451545.0)).is_ok());
}
