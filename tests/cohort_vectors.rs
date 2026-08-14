// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Compares the analytical engine against committed NASA/JPL DE440s vectors.
//!
//! The vectors in `tests/vectors/cohort_de440s.tsv` were produced by the
//! `verify` lane from the kernel named in that file's header, so this is a
//! comparison against JPL rather than the engine agreeing with itself. The
//! test itself needs no kernel, no network, and no optional feature, which is
//! what lets the cohort's evidence run in ordinary CI.
//!
//! Regenerate the vectors with the command recorded in the file header.

extern crate turquet;

use std::collections::BTreeMap;

use turquet::apparent::{geocent_apparent_ecl_pos, ApparentBody, APPARENT_BODIES};

/// Ceiling for a committed vector. Measured worst across this file is 4
/// millidegrees, on the Moon; every other body holds 1 or better.
const MAX_ERROR_MILLIDEGREES: i64 = 10;

const VECTORS: &str = include_str!("vectors/cohort_de440s.tsv");

fn body_named(name: &str) -> Option<ApparentBody> {
    APPARENT_BODIES
        .iter()
        .find(|body| body.name() == name)
        .copied()
}

#[test]
fn committed_de440s_vectors_hold() {
    let mut compared = 0_u32;
    let mut worst = 0_i64;
    let mut worst_label = String::new();
    let mut per_body: BTreeMap<&str, i64> = BTreeMap::new();
    let mut failures = Vec::new();

    for line in VECTORS.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5, "malformed vector line: {line}");
        let body = body_named(fields[2]).unwrap_or_else(|| panic!("unknown body {}", fields[2]));
        // jde_tt is the authority: the label is human-readable only, so this
        // compares position math without re-deriving a time scale.
        let jde_tt: f64 = fields[1].parse().expect("jde_tt");
        let expected_longitude: i64 = fields[3].parse().expect("longitude");
        let expected_latitude: i64 = fields[4].parse().expect("latitude");

        let (longitude, latitude) = geocent_apparent_ecl_pos(&body, jde_tt)
            .unwrap_or_else(|_| panic!("vector epoch {} is inside {}'s range", fields[0], fields[2]));

        let longitude_error = circular_error(millidegrees(longitude), expected_longitude);
        let latitude_error = millidegrees(latitude) - expected_latitude;
        for error in [longitude_error.abs(), latitude_error.abs()] {
            let entry = per_body.entry(body.name()).or_insert(0);
            *entry = (*entry).max(error);
            if error > worst {
                worst = error;
                worst_label = format!("{} at {}", body.name(), fields[0]);
            }
            if error > MAX_ERROR_MILLIDEGREES {
                failures.push(format!(
                    "{} at {}: d-lon {longitude_error}, d-lat {latitude_error}",
                    body.name(),
                    fields[0]
                ));
            }
        }
        compared += 1;
    }

    // A silently empty or truncated vector file must not read as success.
    assert!(
        compared >= 5_000,
        "expected the committed cohort, compared only {compared} vectors"
    );
    println!("compared {compared} committed vectors");
    for (body, error) in &per_body {
        println!("{body:<9} worst {error} millidegrees");
    }
    println!("worst overall: {worst} millidegrees ({worst_label})");
    assert!(
        failures.is_empty(),
        "{} vectors exceeded {MAX_ERROR_MILLIDEGREES} millidegrees:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The header must keep naming the oracle that produced the file, so a
/// regenerated set cannot quietly lose its provenance.
#[test]
fn the_vector_file_records_its_oracle() {
    let header = VECTORS
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for expected in [
        "kernel-sha256:",
        "jde_tt is the authority",
        "merely-made/anise@",
        "SOFARS",
        "geocentric apparent ecliptic of date",
        "millidegrees",
        "regenerate:",
    ] {
        assert!(
            header.contains(expected),
            "vector header must record {expected}"
        );
    }
}

fn millidegrees(radians: f64) -> i64 {
    (radians.to_degrees() * 1_000.0).round() as i64
}

fn circular_error(actual: i64, expected: i64) -> i64 {
    (actual - expected + 180_000).rem_euclid(360_000) - 180_000
}
