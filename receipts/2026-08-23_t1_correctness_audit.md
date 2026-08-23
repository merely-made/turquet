# T1 receipt: inherited correctness and disclosure audit

**Date:** 2026-08-23

## Reproduced defects

The inherited code failed three direct assertions:

- `coords::hr_angl_frm_observer_long` subtracted east-positive longitude,
  reproducing [`astro-rust` issue 18](https://github.com/saurvs/astro-rust/issues/18).
- `coords::dec_frm_hz` used `cos(azimuth)` twice and omitted
  `cos(altitude)`, reproducing
  [issue 19](https://github.com/saurvs/astro-rust/issues/19) and the earlier
  [ERFA comparison in issue 13](https://github.com/saurvs/astro-rust/issues/13).
- `lunar::time_of_phase` ended the quarter-phase `W` expression early, so
  the final three cosine terms were evaluated and discarded. The compiler's
  `unused_must_use` warning was a correctness report.

Auditing the complete lunar phase formula exposed two adjacent defects:

- the lunation index was truncated toward zero instead of rounded to the
  nearest integer, selecting the following lunation for part of every year
  before 2000;
- the `-0.00034 E sin(2M' - M)` quarter correction omitted `E`.

The sign repair also exposed west-positive longitude fixtures in the inherited
parallax and transit examples. Their geographical values are now expressed
east-positive, and transit's private Meeus relation translates the original
west-positive term at that boundary.

## Independent evidence

`tests/coords.rs` carries the IAU SOFA 2023-10-11 `iauHd2ae` and `iauAe2hd`
validation vectors. Turquet retains the inherited south-zero, east-positive
azimuth convention, so the test translates only the azimuth origin by pi;
the numerical transform is otherwise compared directly. Both affected tests
failed before the repairs.

`tests/lunar.rs` carries three distinct receipts:

- PyMeeus 0.5.12's published 2044 last-quarter example, in TT, catches the
  discarded `W` term at subsecond precision. The inherited expression missed
  the vector by 0.276321 seconds; the repaired expression is within 0.1 second
  of the independently rounded result.
- NASA GSFC's minute-resolution 2000 first-quarter entry is converted from its
  published UT label through Turquet's `hifitime` UTC-to-TT boundary, then
  compared in JDE TT.
- NASA GSFC's 1999 January New Moon proves nearest-lunation selection. The
  inherited truncation selected the February event, while rounding selects
  the published January event.

The NASA catalog states that its times are Universal Time and that Fred
Espenak calculated them from Meeus's algorithms. The calendar label therefore
is converted before comparison; treating it directly as TT creates a false
roughly one-minute discrepancy.

## Disclosure boundary

`AUDIT.md` names every exported calculation and records its current source,
units, frame or convention, time scale, range, and evidence. An omitted legacy
contract is written as unverified rather than inferred. The measured consumer
path is `apparent`; inherited modules remain the 0.1.x compatibility surface.

This meets T1's disclosure gate. It does not make the anonymous legacy API
typed, add IAU 2006/2000A, or validate every inherited calculation. Those are
the T2 boundary.
