/*
Copyright (c) 2015, 2016 Saurav Sachidanand

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
*/

use super::*;

/**
Computes the times of passage of the Moon through the ascending and
descending nodes, close to a given date

# Returns

`(time_of_ascend_node, time_of_descend_node)`

* `time_of_ascend_node` : Time of passage through the ascending node
* `time_of_descend_node`: Time of passage through the descending node

# Arguments

`date`: The Date
**/
#[inline]
pub fn time_of_passage_through_nodes(date: &time::Date) -> (f64, f64) {
    let k = 13.4223 * (time::decimal_year(&date) - 2000.05);
    let T = k / 1342.23;
    let k1 = (k as i32) as f64;
    let k2 = (k1 as f64) + 0.5;

    (
        time_of_passage_through_node(k1, T),
        time_of_passage_through_node(k2, T),
    )
}

pub(super) fn time_of_passage_through_node(k: f64, T: f64) -> f64 {
    let D = (183.638 + 331.73735682 * k + T * T * (0.0014852 + T * (0.00000209 - T * 0.00000001)))
        .to_radians();
    let D_times_2 = 2.0 * D;
    let M = (17.4006 + 26.8203725 * k + T * T * (0.0001186 + T * 0.00000006)).to_radians();
    let M1 =
        (38.3776 + 355.52747313 * k + T * T * (0.0123499 + T * (0.000014627 - T * 0.000000069)))
            .to_radians();
    let sigma = (123.9767 - 1.44098956 * k
        + T * T * (0.0020608 + T * (0.00000214 - T * 0.000000016)))
        .to_radians();
    let P = sigma + (272.75 - T * 2.3).to_radians();
    let V = (299.75 + T * (132.85 - T * 0.009173)).to_radians();

    (2451565.1619 + 27.212220817 * k + T * T * (0.0002762 + T * (0.000000021 - T * 0.000000000088))
        - 0.4721 * M1.sin()
        - 0.1649 * D_times_2.sin()
        - 0.0868 * (D_times_2 - M1).sin()
        + 0.0084 * (D_times_2 + M1).sin()
        - 0.0083 * (D_times_2 - M).sin()
        - 0.0039 * (D_times_2 - M - M1).sin()
        + 0.0034 * (2.0 * M1).sin()
        - 0.0031 * (2.0 * (D - M1)).sin()
        + 0.0030 * (D_times_2 + M).sin()
        + 0.0028 * (M - M1).sin()
        + 0.0026 * M.sin()
        + 0.0025 * (2.0 * D_times_2).sin()
        + 0.0024 * D.sin()
        + 0.0022 * (M + M1).sin()
        + 0.0017 * sigma.sin()
        + 0.0014 * (2.0 * D_times_2 - M1).sin()
        + 0.0005 * (D_times_2 + M - M1).sin()
        + 0.0004 * (D_times_2 - M + M1).sin()
        + 0.0003 * ((2.0 * (M - D)).sin() + (2.0 * D_times_2 - M).sin() + V.sin() + P.sin()))
}

/// Represents a phase of the Moon
pub enum Phase {
    /// New Moon
    New,
    /// First Quarter
    First,
    /// Full Moon
    Full,
    /// Last Quarter
    Last,
}

/**
Computes the Julian day corresponding to one of the four phases
of the Moon

# Returns

* `JDE TT`: Julian ephemeris day in Terrestrial Time corresponding to the
            exact time of the phase that is closest to `date`

*Meeus* says the mean error in `JD` for phases between 1980 AD
and mid-2020 AD is 3.8 seconds.

# Arguments

* `date` : Civil date used only to select the nearest lunation. Its time of
           day and time-zone fields do not define the returned time scale.
* `phase`: The [Phase](./enum.Phase.html)
**/
pub fn time_of_phase(date: &time::Date, phase: &Phase) -> f64 {
    // Meeus chapter 49 selects the integer lunation nearest the requested
    // date. Casting to i64 truncated toward zero, choosing the following
    // lunation for part of every year before 2000.
    let K = (12.3685 * (time::decimal_year(&date) - 2000.0)).round();

    let k = match phase {
        &Phase::New => K,
        &Phase::First => K + 0.25,
        &Phase::Full => K + 0.5,
        &Phase::Last => K + 0.75,
    };

    let T = k / 1236.85;

    let mut JD = 2451550.09766
        + k * 29.530588861
        + T * Horner_eval!(T, 0.0, 0.00015437, -0.00000015, 0.00000000073);

    let E = E(T);

    let M =
        (2.5534 + k * 29.1053567 + T * Horner_eval!(T, 0.0, -0.0000014, -0.00000011)).to_radians();

    let M1 = (201.5643
        + k * 385.81693528
        + T * Horner_eval!(T, 0.0, 0.0107582, 0.00001238, -0.000000058))
    .to_radians();

    let F = (160.7108
        + k * 390.67050284
        + T * Horner_eval!(T, 0.0, -0.0016118, -0.00000227, 0.000000011))
    .to_radians();

    let omega =
        (124.7746 - k * 1.56375588 + T * Horner_eval!(T, 0.0, 0.0020672, 0.00000215)).to_radians();

    let A1 = (299.77 + 0.107408 * k - 0.009173 * T * T).to_radians();
    let A2 = (251.88 + 0.016321 * k).to_radians();
    let A3 = (251.83 + 26.651886 * k).to_radians();
    let A4 = (349.42 + 36.412478 * k).to_radians();
    let A5 = (84.66 + 18.206239 * k).to_radians();
    let A6 = (141.74 + 53.303771 * k).to_radians();
    let A7 = (207.14 + 2.453732 * k).to_radians();
    let A8 = (154.84 + 7.306860 * k).to_radians();
    let A9 = (34.52 + 27.261239 * k).to_radians();
    let A10 = (207.19 + 0.121824 * k).to_radians();
    let A11 = (291.34 + 1.844379 * k).to_radians();
    let A12 = (161.72 + 24.198154 * k).to_radians();
    let A13 = (239.56 + 25.513099 * k).to_radians();
    let A14 = (331.55 + 3.592518 * k).to_radians();

    let is_quarter = match phase {
        &Phase::Last | &Phase::First => true,
        &_ => false,
    };

    if is_quarter {
        let W = 0.00306 - 0.00038 * E * M.cos() + 0.00026 * M1.cos()
            - 0.00002 * ((M1 - M).cos() - (M1 + M).cos() - (2.0 * F).cos());

        JD += match phase {
            &Phase::Last => -W,
            &Phase::First => W,
            &_ => 0.0,
        };

        let corrections = [
            [-0.62801, M1],
            [0.17172 * E, M],
            [-0.01183 * E, M1 + M],
            [0.00862, 2.0 * M1],
            [0.00804, 2.0 * F],
            [0.00454 * E, M1 - M],
            [0.00204 * E * E, 2.0 * M],
            [-0.0018, M1 - 2.0 * F],
            [-0.0007, M1 + 2.0 * F],
            [-0.0004, 3.0 * M1],
            [-0.00034 * E, 2.0 * M1 - M],
            [0.00032 * E, M + 2.0 * F],
            [0.00032 * E, M - 2.0 * F],
            [-0.00028 * E * E, M1 + 2.0 * M],
            [0.00027 * E, 2.0 * M1 + M],
            [-0.00017, omega],
            [-0.00005, M1 - M - 2.0 * F],
            [0.00004, 2.0 * (M1 + F)],
            [-0.00004, M1 + M + 2.0 * F],
            [0.00004, M1 - 2.0 * M],
            [0.00003, M1 + M - 2.0 * F],
            [0.00003, 3.0 * M],
            [0.00002, 2.0 * (M1 - F)],
            [0.00002, M1 - M + 2.0 * F],
            [-0.00002, 3.0 * M1 + M],
        ];

        for &x in corrections.iter() {
            JD += x[0] * x[1].sin();
        }
    } else {
        let is_new = match phase {
            &Phase::New => true,
            &_ => false,
        };

        let sine_arguments = [
            M1,
            M,
            2.0 * M1,
            2.0 * F,
            M1 - M,
            M1 + M,
            2.0 * M,
            M1 - 2.0 * F,
            M1 + 2.0 * F,
            2.0 * M1 + M,
            3.0 * M1,
            M + 2.0 * F,
            M - 2.0 * F,
            2.0 * M1 - M,
            omega,
            M1 + 2.0 * M,
            2.0 * (M1 - F),
            3.0 * M,
            M1 + M - 2.0 * F,
            2.0 * (M1 + F),
            M1 + M + 2.0 * F,
            M1 - M + 2.0 * F,
            M1 - M - 2.0 * F,
            3.0 * M1 + M,
            4.0 * M1,
        ];

        let multipliers = if is_new {
            [
                -0.4072,
                0.17241 * E,
                0.01608,
                0.01039,
                0.00739 * E,
                -0.00514 * E,
                0.00208 * E * E,
                -0.00111,
                -0.00057,
                0.00056 * E,
                -0.00042,
                0.00042 * E,
                0.00038 * E,
                -0.00024 * E,
                -0.00017,
                -0.00007,
                0.00004,
                0.00004,
                0.00003,
                0.00003,
                -0.00003,
                0.00003,
                -0.00002,
                -0.00002,
                0.00002,
            ]
        } else {
            [
                -0.40614,
                0.17302 * E,
                0.01614,
                0.01043,
                0.00734 * E,
                -0.00515 * E,
                0.00209 * E * E,
                -0.00111,
                -0.00057,
                0.00056 * E,
                -0.00042,
                0.00042 * E,
                0.00038 * E,
                -0.00024 * E,
                -0.00017,
                -0.00007,
                0.00004,
                0.00004,
                0.00003,
                0.00003,
                -0.00003,
                0.00003,
                -0.00002,
                -0.00002,
                0.00002,
            ]
        };

        for mx in multipliers.iter().zip(sine_arguments.iter()) {
            JD += mx.0 * (mx.1).sin();
        }
    }

    let additional_corrections = [
        [0.000_325, A1],
        [0.000_165, A2],
        [0.000_164, A3],
        [0.000_126, A4],
        [0.000_110, A5],
        [0.000_062, A6],
        [0.000_06, A7],
        [0.000_056, A8],
        [0.000_047, A9],
        [0.000_042, A10],
        [0.000_04, A11],
        [0.000_037, A12],
        [0.000_035, A13],
        [0.000_023, A14],
    ];
    for &x in additional_corrections.iter() {
        JD += x[0] * x[1].sin();
    }

    JD
}
