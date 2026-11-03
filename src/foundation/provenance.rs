// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

use super::*;

/// The named algorithm and revision that produced a value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    name: &'static str,
    revision: &'static str,
}

impl Model {
    pub const fn new(name: &'static str, revision: &'static str) -> Self {
        Self { name, revision }
    }

    pub fn name(self) -> &'static str {
        self.name
    }

    pub fn revision(self) -> &'static str {
        self.revision
    }
}

/// How an error ceiling was established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccuracyEvidence {
    /// Agreement with a canonical implementation's published vectors.
    Conformance,
    /// Comparison with an independent external calculation.
    ExternalComparison,
}

/// A bounded angular error and the scope in which it was established.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accuracy {
    max_angular_error: Angle,
    evidence: AccuracyEvidence,
    authority: &'static str,
    scope: &'static str,
}

impl Accuracy {
    pub fn new(
        max_angular_error: Angle,
        evidence: AccuracyEvidence,
        authority: &'static str,
        scope: &'static str,
    ) -> Result<Self, ValueError> {
        if max_angular_error.radians() < 0.0 {
            return Err(ValueError::Negative("angular error"));
        }
        Ok(Self {
            max_angular_error,
            evidence,
            authority,
            scope,
        })
    }

    pub fn max_angular_error(self) -> Angle {
        self.max_angular_error
    }

    pub fn evidence(self) -> AccuracyEvidence {
        self.evidence
    }

    pub fn authority(self) -> &'static str {
        self.authority
    }

    pub fn scope(self) -> &'static str {
        self.scope
    }
}

/// A value accompanied by its model and measured or conformance accuracy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Modelled<T> {
    value: T,
    model: Model,
    accuracy: Accuracy,
}

impl<T> Modelled<T> {
    pub fn new(value: T, model: Model, accuracy: Accuracy) -> Self {
        Self {
            value,
            model,
            accuracy,
        }
    }

    pub fn value(&self) -> &T {
        &self.value
    }

    pub fn into_value(self) -> T {
        self.value
    }

    pub fn model(&self) -> Model {
        self.model
    }

    pub fn accuracy(&self) -> Accuracy {
        self.accuracy
    }
}
