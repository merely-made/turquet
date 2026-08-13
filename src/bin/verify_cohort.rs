// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Sweeps the analytical engine against the JPL verifier across a date
//! cohort and reports the worst residual per body.
//!
//! This is the tool behind roadmap T3's evidence gate. It is maintainer
//! tooling: it needs a kernel the maintainer supplies, and its output is
//! committed as vectors so ordinary builds and CI never touch a kernel.
//!
//! ```text
//! cargo run --features verify --bin verify_cohort -- <kernel.bsp> [step_days]
//! ```

extern crate hifitime;
extern crate turquet;

use std::process;

use hifitime::{Epoch, Unit};
use turquet::apparent::{geocent_apparent_ecl_pos, APPARENT_BODIES};
use turquet::verify::JplVerifier;

/// The cohort's bounds, chosen as the intersection of the Pluto series'
/// stated validity and the DE440s kernel's coverage.
const FIRST_YEAR: i32 = 1885;
const LAST_YEAR: i32 = 2099;
const DEFAULT_STEP_DAYS: f64 = 90.0;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let kernel = match arguments.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: verify_cohort <kernel.bsp> [step_days]");
            process::exit(2);
        }
    };
    let step_days = arguments
        .next()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(DEFAULT_STEP_DAYS);

    let verifier = match JplVerifier::open(&kernel) {
        Ok(verifier) => verifier,
        Err(error) => {
            eprintln!("could not open {}: {}", kernel, error);
            process::exit(1);
        }
    };

    let start = Epoch::from_gregorian_utc(FIRST_YEAR, 1, 1, 12, 0, 0, 0);
    let end = Epoch::from_gregorian_utc(LAST_YEAR, 12, 31, 12, 0, 0, 0);

    // Worst longitude and latitude residual per body, in millidegrees, with
    // the epoch that produced it.
    let mut worst = vec![(0_i64, 0_i64, String::new()); APPARENT_BODIES.len()];
    let mut samples = 0_u32;
    let mut skipped = 0_u32;

    let mut epoch = start;
    while epoch <= end {
        let jde_tt = epoch.to_jde_tt_days();
        for (index, body) in APPARENT_BODIES.iter().enumerate() {
            let analytical = match geocent_apparent_ecl_pos(body, jde_tt) {
                Ok(value) => value,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let reference = match verifier.geocent_apparent_ecl_pos(body, epoch) {
                Ok(value) => value,
                Err(_) => {
                    skipped += 1;
                    continue;
                }
            };
            let longitude_error = circular_millidegrees(analytical.0, reference.0);
            let latitude_error = millidegrees(analytical.1) - millidegrees(reference.1);
            if longitude_error.abs() > worst[index].0 || latitude_error.abs() > worst[index].1 {
                let label = format!("{}", epoch);
                if longitude_error.abs() > worst[index].0 {
                    worst[index].0 = longitude_error.abs();
                    worst[index].2 = label.clone();
                }
                if latitude_error.abs() > worst[index].1 {
                    worst[index].1 = latitude_error.abs();
                    if worst[index].2.is_empty() {
                        worst[index].2 = label;
                    }
                }
            }
        }
        samples += 1;
        epoch = epoch + step_days * Unit::Day;
    }

    println!(
        "cohort {}..{} at {} day steps: {} epochs, {} skipped body-samples",
        FIRST_YEAR, LAST_YEAR, step_days, samples, skipped
    );
    println!("{:<9} {:>12} {:>12}  {}", "body", "worst d-lon", "worst d-lat", "at");
    let mut overall = 0_i64;
    for (index, body) in APPARENT_BODIES.iter().enumerate() {
        let (longitude, latitude, ref at) = worst[index];
        println!(
            "{:<9} {:>12} {:>12}  {}",
            body.name(),
            longitude,
            latitude,
            at
        );
        overall = overall.max(longitude).max(latitude);
    }
    println!("\nworst overall: {} millidegrees", overall);
}

fn millidegrees(radians: f64) -> i64 {
    (radians.to_degrees() * 1_000.0).round() as i64
}

fn circular_millidegrees(actual: f64, expected: f64) -> i64 {
    (millidegrees(actual) - millidegrees(expected) + 180_000).rem_euclid(360_000) - 180_000
}
