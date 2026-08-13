# Adoption roadmap

Turquet's first objective is a trustworthy celestial fact engine. Completing
the inherited catalogue of algorithms follows that foundation.

## T0: Founding baseline

- Preserve upstream history and MIT attribution.
- Establish the Turquet package and repository identity.
- Run the inherited suite on stable Rust in continuous integration.
- Record inherited capabilities, known defects, and unmeasured claims.

Done when the renamed crate builds and its inherited tests pass from a clean
checkout.

## T1: Correctness audit

- Reproduce and repair the open upstream coordinate and lunar defects.
- Inventory every public function by source, units, frame, time scale, range,
  and existing evidence.
- Add independent vectors for corrected behavior.

Done when no public function presents an unstated frame or unit as a verified
calculation.

## T2: Typed foundations

- Introduce typed epochs, time scales, angles, distances, observers, frames,
  states, and accuracy metadata.
- Keep the inherited API in an explicitly named compatibility module until its
  consumers have migrated.
- Add IAU 2006 precession with IAU 2000A nutation and SOFA conformance vectors.

Done when invalid frame and time-scale combinations cannot enter the primary
API as interchangeable floating-point values.

## T3: Analytical ephemeris

- Complete the apparent geocentric and topocentric Sun, Moon, Mercury through
  Pluto pipeline.
- Apply light-time, aberration, deflection, precession, and nutation through
  explicit stages.
- Compare a broad date and observer cohort against JPL output.

Done when the Rust-only provider meets documented tolerances and every result
records its model and supported range.

## T4: Celestial events

- Complete the useful missing Meeus algorithms.
- Express conjunctions, stations, phases, eclipses, rise and set, visibility,
  and extrema as searches over a position provider.
- Report event intervals and uncertainty rather than isolated magic numbers.

Done when the same event algorithms operate over both analytical and external
verification providers.

## T5: Consumers and embedding

- Prove one legitimate astronomy consumer and one interpretive consumer.
- Add a bounded embedded profile for position and orientation calculations.
- Keep control policy, secrets, interpretation, and social authority outside
  the engine.

Done when two materially different consumers reuse the same celestial state
and derivation receipt without duplicating the calculation.
