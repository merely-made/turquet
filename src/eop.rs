// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Immutable, caller-supplied final observed Earth orientation. No I/O is performed.
//!
//! `parse_finals2000a` accepts a bounded excerpt of the fixed-column format
//! documented at <https://maia.usno.navy.mil/ser7/readme.finals2000A>.
//! Only Bulletin B x, y and UT1-UTC (columns 135..165, one-based) are used.
//! Bulletin A polar-motion and UT1 flags must both be `I`; `P` is rejected.
//! Nutation flags do not classify these three independent quantities.
//! Every nonblank line must be a complete final record; no rows are silently
//! skipped and no rapid/predicted values substitute for missing B values.
//!
//! UTC MJD denotes midnight, from 1972 onwards. Interpolation is linear only
//! between consecutive daily records with the same TAI-UTC in hifitime's
//! retained leap table. Leap intervals and DUT1 jumps greater than 0.5 seconds
//! are refused. Updating hifitime's leap table remains an integration concern.
//! Formal errors are retained per sample, never promoted to an interpolation
//! accuracy claim. finals2000A supplies errors for A, not for B: B errors are
//! consequently `None`. Celestial pole offsets and LOD are outside this API.
//!
//! Authority and snapshot are nonempty caller assertions, preserved verbatim;
//! the parser neither authenticates an IERS origin nor computes a digest.
//! Callers should identify a retained source by URL and content hash. The exact
//! pasted source is retained by `source_text()` for replay and provenance.

use std::fmt;

use foundation::{Angle, JulianDate, ScaleAwareEpoch, TerrestrialTime, UniversalTime1};
use observer::EarthOrientation;
use provider::EarthOrientationProvider;

/// Resource limits apply before retaining source or allocating a sample series.
pub const MAX_EOP_SAMPLES: usize = 100_000;
pub const MAX_EOP_SOURCE_BYTES: usize = 20_000_000;

/// Supplied one-sigma formal errors, in the units named by each field.
/// These describe an input sample, not an interpolated result's accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EopFormalErrors {
    pub dut1_seconds: f64,
    pub xp_arcseconds: f64,
    pub yp_arcseconds: f64,
}

/// One final observed sample at UTC midnight. Construction asserts final
/// observed status; callers importing another format own that classification.
#[derive(Clone, Debug, PartialEq)]
pub struct EopSample {
    mjd_utc: f64,
    dut1_seconds: f64,
    xp_arcseconds: f64,
    yp_arcseconds: f64,
    formal_errors: Option<EopFormalErrors>,
}

impl EopSample {
    pub fn new(
        mjd_utc: f64,
        dut1_seconds: f64,
        xp_arcseconds: f64,
        yp_arcseconds: f64,
        formal_errors: Option<EopFormalErrors>,
    ) -> Result<Self, EopError> {
        if !mjd_utc.is_finite()
            || mjd_utc.fract() != 0.0
            || !(41_317.0..=1_000_000.0).contains(&mjd_utc)
        {
            return Err(EopError::InvalidValue("UTC midnight MJD (1972 onwards)"));
        }
        for &(name, value) in &[
            ("DUT1 seconds", dut1_seconds),
            ("xp arcseconds", xp_arcseconds),
            ("yp arcseconds", yp_arcseconds),
        ] {
            if !value.is_finite() {
                return Err(EopError::InvalidValue(name));
            }
        }
        // Broad sanity bounds for terrestrial observed data; also prevent
        // overflowing subsequent angle and time arithmetic.
        if dut1_seconds.abs() > 1.0 || xp_arcseconds.abs() > 10.0 || yp_arcseconds.abs() > 10.0 {
            return Err(EopError::InvalidValue("Earth orientation sanity bounds"));
        }
        if let Some(errors) = formal_errors {
            if [
                errors.dut1_seconds,
                errors.xp_arcseconds,
                errors.yp_arcseconds,
            ]
            .iter()
            .any(|e| !e.is_finite() || *e < 0.0)
            {
                return Err(EopError::InvalidValue("nonnegative finite formal errors"));
            }
        }
        Ok(Self {
            mjd_utc,
            dut1_seconds,
            xp_arcseconds,
            yp_arcseconds,
            formal_errors,
        })
    }

    pub fn mjd_utc(&self) -> f64 {
        self.mjd_utc
    }
    pub fn dut1_seconds(&self) -> f64 {
        self.dut1_seconds
    }
    pub fn xp_arcseconds(&self) -> f64 {
        self.xp_arcseconds
    }
    pub fn yp_arcseconds(&self) -> f64 {
        self.yp_arcseconds
    }
    pub fn formal_errors(&self) -> Option<EopFormalErrors> {
        self.formal_errors
    }

    fn tai_minus_utc(&self) -> f64 {
        ScaleAwareEpoch::from_mjd_utc(self.mjd_utc).leap_seconds_iers() as f64
    }

    /// TT at this sample's UTC midnight, retaining the small offset separately.
    pub fn epoch_tt(&self) -> JulianDate<TerrestrialTime> {
        JulianDate::from_parts(
            2_400_000.5 + self.mjd_utc,
            (self.tai_minus_utc() + 32.184) / 86_400.0,
        )
        .expect("validated sample")
    }
}

/// Rejections never fall back to extrapolation, rapid values, or predictions.
#[derive(Clone, Debug, PartialEq)]
pub enum EopError {
    EmptyIdentity,
    EmptySeries,
    ResourceLimit,
    InvalidValue(&'static str),
    /// One-based source line and field name.
    Parse {
        line: usize,
        field: &'static str,
    },
    NotFinalObserved {
        line: usize,
    },
    NotStrictlyIncreasing {
        index: usize,
    },
    OutOfRange,
    Gap {
        lower_index: usize,
        upper_index: usize,
    },
    LeapDiscontinuity {
        lower_index: usize,
        upper_index: usize,
    },
}

impl fmt::Display for EopError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EopError::EmptyIdentity => write!(f, "authority and snapshot must be nonempty"),
            EopError::EmptySeries => write!(f, "no final observed EOP samples"),
            EopError::ResourceLimit => write!(f, "retained EOP resource limit exceeded"),
            EopError::InvalidValue(field) => write!(f, "invalid {}", field),
            EopError::Parse { line, field } => write!(f, "invalid {} at EOP line {}", field, line),
            EopError::NotFinalObserved { line } => write!(
                f,
                "EOP line {} is not a complete final observed record",
                line
            ),
            EopError::NotStrictlyIncreasing { index } => {
                write!(f, "EOP sample {} is duplicate or out of order", index)
            },
            EopError::OutOfRange => write!(f, "epoch outside retained final observed EOP range"),
            EopError::Gap {
                lower_index,
                upper_index,
            } => write!(
                f,
                "EOP gap between samples {} and {}",
                lower_index, upper_index
            ),
            EopError::LeapDiscontinuity {
                lower_index,
                upper_index,
            } => write!(
                f,
                "EOP leap or DUT1 discontinuity between samples {} and {}",
                lower_index, upper_index
            ),
        }
    }
}

impl ::std::error::Error for EopError {}

/// A result tied to the exact requested two-part TT and retained sample indices.
/// At a knot both indices are identical and the fraction is zero. Endpoint
/// formal errors remain accessible through `EopSeries::samples()`.
#[derive(Clone, Debug, PartialEq)]
pub struct EopEvaluation {
    pub requested_tt: JulianDate<TerrestrialTime>,
    pub earth_orientation: EarthOrientation,
    pub lower_index: usize,
    pub upper_index: usize,
    pub fraction: f64,
    pub dut1_seconds: f64,
    pub xp_arcseconds: f64,
    pub yp_arcseconds: f64,
}

/// An owned immutable snapshot. Cloning preserves source and sample identity.
#[derive(Clone, Debug, PartialEq)]
pub struct EopSeries {
    authority: String,
    snapshot: String,
    source_text: String,
    samples: Vec<EopSample>,
}

impl EopSeries {
    /// Retain caller-classified final observations and their exact source text.
    /// Samples must be strictly ordered; gaps are retained but not interpolated.
    /// This constructor does not verify that the text encodes the supplied
    /// samples. Prefer `parse_finals2000a` for a checked text-to-sample mapping.
    pub fn new(
        source_text: &str,
        authority: &str,
        snapshot: &str,
        samples: Vec<EopSample>,
    ) -> Result<Self, EopError> {
        check_source(source_text, authority, snapshot)?;
        if samples.is_empty() {
            return Err(EopError::EmptySeries);
        }
        if samples.len() > MAX_EOP_SAMPLES {
            return Err(EopError::ResourceLimit);
        }
        for index in 1..samples.len() {
            if samples[index].mjd_utc <= samples[index - 1].mjd_utc {
                return Err(EopError::NotStrictlyIncreasing { index });
            }
        }
        Ok(Self {
            authority: authority.to_owned(),
            snapshot: snapshot.to_owned(),
            source_text: source_text.to_owned(),
            samples,
        })
    }

    /// Strict fixed-column finals2000A Bulletin B parser. Blank lines are
    /// allowed; headers, malformed lines and incomplete/nonfinal rows fail.
    /// Calendar fields must agree with the midnight MJD. All errors are atomic:
    /// a failed parse exposes no partial retained series.
    pub fn parse_finals2000a(
        source_text: &str,
        authority: &str,
        snapshot: &str,
    ) -> Result<Self, EopError> {
        check_source(source_text, authority, snapshot)?;
        let mut samples = Vec::new();
        for (index, row) in source_text.lines().enumerate() {
            let line = index + 1;
            if row.trim().is_empty() {
                continue;
            }
            if samples.len() == MAX_EOP_SAMPLES {
                return Err(EopError::ResourceLimit);
            }
            if !row.is_ascii() || row.len() < 165 {
                return Err(EopError::Parse {
                    line,
                    field: "ASCII fixed-column record (at least 165 columns)",
                });
            }
            if &row[16..17] != "I"
                || &row[57..58] != "I"
                || row[134..144].trim().is_empty()
                || row[144..154].trim().is_empty()
                || row[154..165].trim().is_empty()
            {
                return Err(EopError::NotFinalObserved { line });
            }
            let number = |start: usize, end: usize, field| -> Result<f64, EopError> {
                row[start..end]
                    .trim()
                    .parse::<f64>()
                    .map_err(|_| EopError::Parse { line, field })
            };
            let sample = EopSample::new(
                number(7, 15, "UTC MJD")?,
                number(154, 165, "Bulletin B DUT1 seconds")?,
                number(134, 144, "Bulletin B xp arcseconds")?,
                number(144, 154, "Bulletin B yp arcseconds")?,
                None,
            )
            .map_err(|_| EopError::Parse {
                line,
                field: "finite midnight EOP values within sanity bounds",
            })?;
            let (year, month, day, _, _, _, _) =
                ScaleAwareEpoch::from_mjd_utc(sample.mjd_utc).to_gregorian_utc();
            let year_base = if sample.mjd_utc <= 51_543.0 {
                1900.0
            } else {
                2000.0
            };
            if number(0, 2, "year")? + year_base != year as f64
                || number(2, 4, "month")? != month as f64
                || number(4, 6, "day")? != day as f64
            {
                return Err(EopError::Parse {
                    line,
                    field: "calendar date/MJD agreement",
                });
            }
            samples.push(sample);
        }
        Self::new(source_text, authority, snapshot, samples)
    }

    pub fn source_text(&self) -> &str {
        &self.source_text
    }
    pub fn samples(&self) -> &[EopSample] {
        &self.samples
    }
    pub fn authority(&self) -> &str {
        &self.authority
    }
    pub fn snapshot(&self) -> &str {
        &self.snapshot
    }

    /// Inclusive coverage endpoints in TT. Interior gaps/leap intervals may
    /// still refuse evaluation; this range is not a claim of continuous cover.
    pub fn coverage_tt(&self) -> (JulianDate<TerrestrialTime>, JulianDate<TerrestrialTime>) {
        (
            self.samples[0].epoch_tt(),
            self.samples[self.samples.len() - 1].epoch_tt(),
        )
    }

    pub fn evaluate(
        &self,
        requested_tt: JulianDate<TerrestrialTime>,
    ) -> Result<EopEvaluation, EopError> {
        // Difference parts before adding: do not collapse the requested JD to
        // a single large float or silently replace it with a rounded UTC time.
        let difference = |sample: &EopSample| {
            let (a, b) = requested_tt.parts();
            let (c, d) = sample.epoch_tt().parts();
            (a - c) + (b - d)
        };
        let (a, b) = requested_tt.parts();
        // Pathological cancelling JD parts can otherwise compare equal to
        // every sample after catastrophic cancellation. Conventional JD,
        // MJD, J2000 and date-and-fraction splits are all comfortably inside
        // this explicit supported representation bound.
        if !a.is_finite() || !b.is_finite() || a.abs() > 1e9 || b.abs() > 1e9 {
            return Err(EopError::InvalidValue("TT epoch"));
        }
        let upper = self
            .samples
            .partition_point(|sample| difference(sample) > 0.0);
        let (lower, upper, fraction) =
            if upper < self.samples.len() && difference(&self.samples[upper]) == 0.0 {
                (upper, upper, 0.0)
            } else {
                if upper == 0 || upper == self.samples.len() {
                    return Err(EopError::OutOfRange);
                }
                let lower = upper - 1;
                let left = &self.samples[lower];
                let right = &self.samples[upper];
                if right.mjd_utc - left.mjd_utc != 1.0 {
                    return Err(EopError::Gap {
                        lower_index: lower,
                        upper_index: upper,
                    });
                }
                if left.tai_minus_utc() != right.tai_minus_utc()
                    || (right.dut1_seconds - left.dut1_seconds).abs() > 0.5
                {
                    return Err(EopError::LeapDiscontinuity {
                        lower_index: lower,
                        upper_index: upper,
                    });
                }
                (lower, upper, difference(left)) // one SI day, with no leap
            };
        let left = &self.samples[lower];
        let right = &self.samples[upper];
        let interpolate = |l: f64, r: f64| l + fraction * (r - l);
        let dut1_seconds = interpolate(left.dut1_seconds, right.dut1_seconds);
        let xp_arcseconds = interpolate(left.xp_arcseconds, right.xp_arcseconds);
        let yp_arcseconds = interpolate(left.yp_arcseconds, right.yp_arcseconds);
        let ut1 = JulianDate::<UniversalTime1>::from_parts(
            a,
            b + (dut1_seconds - left.tai_minus_utc() - 32.184) / 86_400.0,
        )
        .map_err(|_| EopError::InvalidValue("UT1 epoch"))?;
        let earth_orientation = EarthOrientation::new(
            ut1,
            Angle::from_degrees(xp_arcseconds / 3600.0).expect("validated polar motion"),
            Angle::from_degrees(yp_arcseconds / 3600.0).expect("validated polar motion"),
            self.authority(),
            self.snapshot(),
        );
        Ok(EopEvaluation {
            requested_tt,
            earth_orientation,
            lower_index: lower,
            upper_index: upper,
            fraction,
            dut1_seconds,
            xp_arcseconds,
            yp_arcseconds,
        })
    }
}

impl EarthOrientationProvider for EopSeries {
    type Error = EopError;
    fn authority(&self) -> &str {
        self.authority()
    }
    fn data_snapshot(&self) -> &str {
        self.snapshot()
    }
    fn at(&self, epoch: JulianDate<TerrestrialTime>) -> Result<EarthOrientation, EopError> {
        self.evaluate(epoch)
            .map(|evaluation| evaluation.earth_orientation)
    }
}

fn check_source(source: &str, authority: &str, snapshot: &str) -> Result<(), EopError> {
    if source.len() > MAX_EOP_SOURCE_BYTES {
        return Err(EopError::ResourceLimit);
    }
    if authority.trim().is_empty() || snapshot.trim().is_empty() {
        return Err(EopError::EmptyIdentity);
    }
    Ok(())
}
