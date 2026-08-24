// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! The verification lane: an independent position provider built on the JPL
//! DE440s kernel, used to generate and check golden vectors.
//!
//! This module exists so Turquet's analytical engine can be measured against
//! an authority rather than against itself. It is deliberately built from
//! foreign parts: ANISE reads the SPK kernel and owns time conversion, and
//! SOFA-derived matrices supply precession and nutation. Nothing here routes
//! through Turquet's own inherited series, so agreement between the two lanes
//! is evidence rather than tautology.
//!
//! It is never part of a consumer's dependency graph. The `verify` feature is
//! opt-in tooling, and the kernel it reads is acquired by the maintainer, not
//! downloaded by a product. Consumers compare against committed vectors.

use anise::constants::frames::{
    EARTH_J2000, JUPITER_BARYCENTER_J2000, MARS_BARYCENTER_J2000, MERCURY_J2000, MOON_J2000,
    NEPTUNE_BARYCENTER_J2000, PLUTO_BARYCENTER_J2000, SATURN_BARYCENTER_J2000, SUN_J2000,
    URANUS_BARYCENTER_J2000, VENUS_J2000,
};
use anise::prelude::{Aberration, Almanac, Epoch, Frame, SPK};
use hifitime::TimeScale;
use sha2::{Digest, Sha256};

use std::fs::File;
use std::io::Read;

use apparent::ApparentBody;
use foundation::{
    Direction, Distance, JulianDate, Latitude, Longitude, Model, State, TerrestrialTime,
    TrueEclipticEquinoxOfDate,
};

const J2000_JD: f64 = 2_451_545.0;

/// The verifier implementation identity. Kernel provenance remains
/// caller-owned and must accompany any persisted verification receipt.
pub const JPL_SPK_VERIFIER: Model = Model::new("JPL SPK verifier through ANISE", "71e973a245e6");

/// The verifier's frame for each body. Barycenters are used for the outer
/// planets, matching the convention Horizons reports for chart work.
fn frame_for(body: &ApparentBody) -> Frame {
    match *body {
        ApparentBody::Sun => SUN_J2000,
        ApparentBody::Moon => MOON_J2000,
        ApparentBody::Mercury => MERCURY_J2000,
        ApparentBody::Venus => VENUS_J2000,
        ApparentBody::Mars => MARS_BARYCENTER_J2000,
        ApparentBody::Jupiter => JUPITER_BARYCENTER_J2000,
        ApparentBody::Saturn => SATURN_BARYCENTER_J2000,
        ApparentBody::Uranus => URANUS_BARYCENTER_J2000,
        ApparentBody::Neptune => NEPTUNE_BARYCENTER_J2000,
        ApparentBody::Pluto => PLUTO_BARYCENTER_J2000,
    }
}

#[derive(Debug)]
pub enum VerifyError {
    Kernel(String),
    Calculation { body: &'static str, detail: String },
}

impl ::std::fmt::Display for VerifyError {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            VerifyError::Kernel(ref detail) => write!(formatter, "kernel: {}", detail),
            VerifyError::Calculation { body, ref detail } => {
                write!(formatter, "could not calculate {}: {}", body, detail)
            }
        }
    }
}

impl ::std::error::Error for VerifyError {}

/// A DE440s-backed provider. The kernel stays caller-owned and offline.
pub struct JplVerifier {
    almanac: Almanac,
    kernel_snapshot: String,
}

impl JplVerifier {
    /// Load a JPL SPK kernel. The caller is responsible for its provenance;
    /// this lane records its SHA-256 identity but imposes no approved-digest
    /// policy.
    pub fn open(path: &str) -> Result<Self, VerifyError> {
        let kernel_snapshot = format!("sha256:{}", sha256_file(path)?);
        let spk = SPK::load(path).map_err(|error| VerifyError::Kernel(error.to_string()))?;
        Ok(JplVerifier {
            almanac: Almanac::from_spk(spk),
            kernel_snapshot,
        })
    }

    pub fn kernel_snapshot(&self) -> &str {
        &self.kernel_snapshot
    }

    /// Apparent geocentric ecliptic longitude and latitude on the true
    /// equinox of date, in radians, computed entirely from ANISE and SOFA.
    pub fn geocent_apparent_ecl_pos(
        &self,
        body: &ApparentBody,
        epoch: Epoch,
    ) -> Result<(f64, f64), VerifyError> {
        let (longitude, latitude, _) = self.geocent_apparent_components(body, epoch)?;
        Ok((longitude, latitude))
    }

    /// Typed state used by provider-neutral event algorithms.
    pub fn geocent_apparent_state(
        &self,
        body: &ApparentBody,
        epoch: JulianDate<TerrestrialTime>,
    ) -> Result<State<TrueEclipticEquinoxOfDate>, VerifyError> {
        let reference_epoch = Epoch::from_jde_in_time_scale(epoch.day(), TimeScale::TT);
        let (longitude, latitude, distance_km) =
            self.geocent_apparent_components(body, reference_epoch)?;
        Ok(State::new(
            epoch,
            Direction::new(
                Longitude::from_radians(longitude).expect("verifier longitude is finite"),
                Latitude::from_radians(latitude).expect("verifier latitude is finite and physical"),
            ),
            Distance::from_kilometers(distance_km)
                .expect("verifier distance is finite and nonnegative"),
        ))
    }

    fn geocent_apparent_components(
        &self,
        body: &ApparentBody,
        epoch: Epoch,
    ) -> Result<(f64, f64, f64), VerifyError> {
        let name = body.name();
        let state = self
            .almanac
            .translate(frame_for(body), EARTH_J2000, epoch, Aberration::CN_S)
            .map_err(|error| VerifyError::Calculation {
                body: name,
                detail: error.to_string(),
            })?;
        let jd_tt = epoch.to_jde_tt_days();
        let date2 = jd_tt - J2000_JD;
        let precession = sofars::pnp::pmat76(J2000_JD, date2);
        let nutation = sofars::pnp::nutm80(J2000_JD, date2);
        let mean_of_date = matrix_vector(precession, state.radius_km.into());
        let radius = matrix_vector(nutation, mean_of_date);
        let (_, nutation_in_obliquity) = sofars::pnp::nut80(J2000_JD, date2);
        let obliquity = sofars::pnp::obl80(J2000_JD, date2) + nutation_in_obliquity;
        let (sine, cosine) = obliquity.sin_cos();
        let x = radius[0];
        let y = radius[1] * cosine + radius[2] * sine;
        let z = -radius[1] * sine + radius[2] * cosine;
        let two_pi = 2.0 * ::std::f64::consts::PI;
        Ok((
            y.atan2(x).rem_euclid(two_pi),
            z.atan2(x.hypot(y)),
            norm(radius),
        ))
    }
}

fn matrix_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn norm(vector: [f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

fn sha256_file(path: &str) -> Result<String, VerifyError> {
    let mut file = File::open(path).map_err(|error| VerifyError::Kernel(error.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| VerifyError::Kernel(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
