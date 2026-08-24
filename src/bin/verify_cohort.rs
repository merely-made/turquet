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
//! cargo run --release --features verify --bin verify_cohort -- \
//!     <kernel.bsp> [step_days] [--emit <path>] [--emit-step <days>]
//! ```
//!
//! `--emit` writes a committed vector file holding the **oracle's** values,
//! not the analytical engine's, so the test that reads it back is a
//! comparison against JPL rather than the engine agreeing with itself.

extern crate hifitime;
extern crate sha2;
extern crate turquet;

use std::io::Write;
use std::process;

use hifitime::{Epoch, Unit};
use turquet::apparent::{ApparentError, ApparentSky, APPARENT_BODIES};
use turquet::foundation::{JulianDate, TerrestrialTime};
use turquet::verify::JplVerifier;

/// The cohort's bounds, chosen as the intersection of the Pluto series'
/// stated validity and the DE440s kernel's coverage.
const FIRST_YEAR: i32 = 1885;
const LAST_YEAR: i32 = 2099;
const DEFAULT_STEP_DAYS: f64 = 90.0;
/// Emit step. A prime number of days keeps the sampled epochs from aliasing
/// with the year, the synodic month, or the sidereal month, so a modest file
/// still covers a wide spread of solar and lunar phase.
const DEFAULT_EMIT_STEP_DAYS: f64 = 149.0;
const ANISE_REVISION: &str = "71e973a245e6701e14a5d4c88a3c4e7dedbf7702";

fn main() {
    std::thread::Builder::new()
        .name("turquet-cohort-verifier".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .expect("could not start verifier worker")
        .join()
        .expect("verifier worker panicked");
}

fn run() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    let positional = arguments
        .iter()
        .take_while(|value| !value.starts_with("--"))
        .cloned()
        .collect::<Vec<_>>();
    let kernel = match positional.first() {
        Some(path) => path.clone(),
        None => {
            eprintln!(
                "usage: verify_cohort <kernel.bsp> [step_days] [--emit <path>] [--emit-step <days>]"
            );
            process::exit(2);
        }
    };
    let step_days = positional
        .get(1)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(DEFAULT_STEP_DAYS);
    let emit_path = flag_value(&arguments, "--emit");
    let emit_step_days = flag_value(&arguments, "--emit-step")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(DEFAULT_EMIT_STEP_DAYS);

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
    // Skips are accounted by reason. A sweep that silently drops samples
    // cannot distinguish a working range guard from a broken calculation.
    let mut skipped_out_of_range = 0_u32;
    let mut skipped_analytical = 0_u32;
    let mut skipped_reference = 0_u32;
    let mut first_skip = String::new();

    let mut epoch = start;
    while epoch <= end {
        let typed_epoch = JulianDate::<TerrestrialTime>::from_epoch(epoch);
        let sky = ApparentSky::at(typed_epoch);
        for (index, body) in APPARENT_BODIES.iter().enumerate() {
            let analytical = match sky.position(*body) {
                Ok(value) => value,
                Err(error) => {
                    match error {
                        ApparentError::OutsideSeriesRange { .. } => skipped_out_of_range += 1,
                        _ => skipped_analytical += 1,
                    }
                    if first_skip.is_empty() {
                        first_skip = format!("{} {} at {}", body.name(), describe(&error), epoch);
                    }
                    continue;
                }
            };
            let reference = match verifier.geocent_apparent_ecl_pos(body, epoch) {
                Ok(value) => value,
                Err(_) => {
                    skipped_reference += 1;
                    continue;
                }
            };
            let analytical_direction = analytical.value().direction();
            let longitude_error =
                circular_millidegrees(analytical_direction.longitude().radians(), reference.0);
            let latitude_error =
                millidegrees(analytical_direction.latitude().radians()) - millidegrees(reference.1);
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

    let attempted = u64::from(samples) * APPARENT_BODIES.len() as u64;
    let skipped = u64::from(skipped_out_of_range + skipped_analytical + skipped_reference);
    println!(
        "cohort {}..{} at {} day steps: {} epochs, {} of {} body-samples compared",
        FIRST_YEAR,
        LAST_YEAR,
        step_days,
        samples,
        attempted - skipped,
        attempted
    );
    println!(
        "skipped: {} outside a declared series range, {} analytical failures, {} reference failures",
        skipped_out_of_range, skipped_analytical, skipped_reference
    );
    if !first_skip.is_empty() {
        println!("first skip: {}", first_skip);
    }
    println!(
        "{:<9} {:>12} {:>12}  {}",
        "body", "worst d-lon", "worst d-lat", "at"
    );
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

    if let Some(path) = emit_path {
        match emit_vectors(&verifier, &kernel, &path, emit_step_days, start, end) {
            Ok(count) => println!("wrote {} oracle vectors to {}", count, path),
            Err(error) => {
                eprintln!("could not write {}: {}", path, error);
                process::exit(1);
            }
        }
    }
}

/// Why a sample was skipped, so the summary distinguishes a working range
/// guard from a broken calculation.
fn describe(error: &ApparentError) -> String {
    match *error {
        ApparentError::OutsideSeriesRange { body, julian_year } => format!(
            "outside the {} series range at Julian year {:.2}",
            body, julian_year
        ),
        ApparentError::BeforeLeapSecondEra => "before the leap-second era".to_string(),
        ApparentError::InvalidCivilTime => "invalid civil time".to_string(),
    }
}

fn millidegrees(radians: f64) -> i64 {
    (radians.to_degrees() * 1_000.0).round() as i64
}

fn circular_millidegrees(actual: f64, expected: f64) -> i64 {
    (millidegrees(actual) - millidegrees(expected) + 180_000).rem_euclid(360_000) - 180_000
}

fn flag_value(arguments: &[String], flag: &str) -> Option<String> {
    arguments
        .iter()
        .position(|value| value == flag)
        .and_then(|index| arguments.get(index + 1))
        .cloned()
}

/// Write the oracle's own positions, with the provenance needed to regenerate
/// them, as a plain text table that stays diffable in Git.
fn emit_vectors(
    verifier: &JplVerifier,
    kernel: &str,
    path: &str,
    step_days: f64,
    start: Epoch,
    end: Epoch,
) -> std::io::Result<u32> {
    let digest = sha256_file(kernel)?;
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(file, "# Turquet cohort vectors")?;
    writeln!(
        file,
        "# oracle: NASA/JPL DE440s read by merely-made/anise@{}, IAU 1976/1980 frames from SOFARS 0.6.1",
        ANISE_REVISION
    )?;
    writeln!(file, "# kernel-sha256: {}", digest)?;
    writeln!(
        file,
        "# cohort: {}..{} at {} day steps, 12:00 UTC",
        FIRST_YEAR, LAST_YEAR, step_days
    )?;
    writeln!(
        file,
        "# frame: geocentric apparent ecliptic of date; units: millidegrees"
    )?;
    writeln!(
        file,
        "# regenerate: cargo run --release --features verify --bin verify_cohort -- <kernel> --emit {}",
        path
    )?;
    writeln!(
        file,
        "# pre-1972 labels are hifitime's UTC extrapolation; jde_tt is the authority"
    )?;
    writeln!(
        file,
        "# columns: utc, jde_tt, body, longitude, latitude (tab separated)"
    )?;

    let mut declined = 0_u32;
    let mut written = 0_u32;
    let mut epoch = start;
    while epoch <= end {
        let (year, month, day, hour, minute, second, _) = epoch.to_gregorian_utc();
        let sky = ApparentSky::at(JulianDate::<TerrestrialTime>::from_epoch(epoch));
        for body in APPARENT_BODIES.iter() {
            // Only emit where the analytical engine declares support. A
            // vector outside its stated range is a contradiction, not a test.
            if sky.position(*body).is_err() {
                declined += 1;
                continue;
            }
            let (longitude, latitude) = match verifier.geocent_apparent_ecl_pos(body, epoch) {
                Ok(value) => value,
                Err(_) => continue,
            };
            writeln!(
                file,
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z\t{:.9}\t{}\t{}\t{}",
                year,
                month,
                day,
                hour,
                minute,
                second,
                epoch.to_jde_tt_days(),
                body.name(),
                millidegrees(longitude),
                millidegrees(latitude)
            )?;
            written += 1;
        }
        epoch = epoch + step_days * Unit::Day;
    }
    if declined > 0 {
        println!(
            "declined {} oracle samples outside the analytical engine's declared range",
            declined
        );
    }
    file.flush()?;
    Ok(written)
}

fn sha256_file(path: &str) -> std::io::Result<String> {
    use sha2::{Digest, Sha256};
    let mut reader = std::io::BufReader::new(std::fs::File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = std::io::Read::read(&mut reader, &mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
