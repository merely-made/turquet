#![no_std]

//! Small, policy-free geometry for an embedded solar tracker.
//!
//! This crate computes a desired Sun direction and the panel's signed
//! incidence cosine. It deliberately does not select actuator targets,
//! enforce travel limits, or operate hardware.

/// A finite, normalized three-dimensional direction.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitVector([f64; 3]);

impl UnitVector {
    /// Validates and normalizes the supplied Cartesian direction.
    pub fn new(components: [f64; 3]) -> Result<Self, VectorError> {
        if !components.iter().all(|component| component.is_finite()) {
            return Err(VectorError::NonFinite);
        }

        let scale = components
            .iter()
            .map(|component| component.abs())
            .fold(0.0_f64, f64::max);
        if scale == 0.0 {
            return Err(VectorError::ZeroLength);
        }

        let scaled = [
            components[0] / scale,
            components[1] / scale,
            components[2] / scale,
        ];
        let magnitude =
            square_root(scaled[0] * scaled[0] + scaled[1] * scaled[1] + scaled[2] * scaled[2]);
        Ok(Self([
            scaled[0] / magnitude,
            scaled[1] / magnitude,
            scaled[2] / magnitude,
        ]))
    }

    /// Returns the normalized Cartesian components.
    pub fn components(self) -> [f64; 3] {
        self.0
    }
}

/// Why a [`UnitVector`] could not be made.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VectorError {
    /// At least one Cartesian component was infinite or NaN.
    NonFinite,
    /// The direction had zero length.
    ZeroLength,
}

fn square_root(value: f64) -> f64 {
    // `new` scales the largest component to one, so this is always in [1, 3].
    // Twelve Newton iterations are comfortably converged over that bounded range.
    let mut estimate = value;
    for _ in 0..12 {
        estimate = (estimate + value / estimate) * 0.5;
    }
    estimate
}

/// Pure geometric input to an actuator or control layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SolarTrackerOutput {
    /// The normalized direction from the tracker toward the Sun.
    pub desired_sun_direction: UnitVector,
    /// The panel-normal dot Sun-direction, clamped to `[-1.0, 1.0]`.
    pub signed_incidence_cosine: f64,
}

/// Computes solar-tracker geometry without choosing how hardware should move.
pub fn solve(sun_direction: UnitVector, panel_normal: UnitVector) -> SolarTrackerOutput {
    let sun = sun_direction.components();
    let panel = panel_normal.components();
    let incidence = sun[0] * panel[0] + sun[1] * panel[1] + sun[2] * panel[2];

    SolarTrackerOutput {
        desired_sun_direction: sun_direction,
        signed_incidence_cosine: incidence.clamp(-1.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{solve, UnitVector, VectorError};

    #[test]
    fn normalizes_a_finite_nonzero_direction() {
        let vector = UnitVector::new([3.0, 4.0, 0.0]).unwrap();
        assert_eq!(vector.components(), [0.6, 0.8, 0.0]);
    }

    #[test]
    fn rejects_non_finite_and_zero_directions() {
        assert_eq!(
            UnitVector::new([f64::NAN, 0.0, 0.0]),
            Err(VectorError::NonFinite)
        );
        assert_eq!(
            UnitVector::new([0.0, 0.0, 0.0]),
            Err(VectorError::ZeroLength)
        );
    }

    #[test]
    fn reports_alignment_orthogonality_and_opposition_without_control_policy() {
        let sun = UnitVector::new([0.0, 0.0, 1.0]).unwrap();
        let front = solve(sun, UnitVector::new([0.0, 0.0, 1.0]).unwrap());
        let side = solve(sun, UnitVector::new([1.0, 0.0, 0.0]).unwrap());
        let back = solve(sun, UnitVector::new([0.0, 0.0, -1.0]).unwrap());

        assert_eq!(front.desired_sun_direction, sun);
        assert_eq!(front.signed_incidence_cosine, 1.0);
        assert_eq!(side.signed_incidence_cosine, 0.0);
        assert_eq!(back.signed_incidence_cosine, -1.0);
    }
}
