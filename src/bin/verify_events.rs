// Copyright 2026 Turquet contributors
// SPDX-License-Identifier: MIT

//! Execute every landed provider-neutral event family through a live JPL SPK
//! verifier and compare the results with Turquet's analytical provider.

extern crate turquet;

use std::process;

use turquet::apparent::ApparentBody;
use turquet::events::{
    eclipse_candidates, ecliptic_longitude_conjunctions, ecliptic_longitude_lunar_phases,
    ecliptic_longitude_stations, lunar_eclipse_circumstances, EclipseCandidateKind, EventInterval,
    LongitudeMotion, LunarEclipseContactKind, LunarEclipseKind, LunarEclipseSearch, LunarPhase,
    SearchWindow, StationSearch,
};
use turquet::foundation::{JulianDate, ScaleAwareEpoch, TerrestrialTime};
use turquet::provider::{AnalyticalEphemeris, GeocentricPositionProvider};
use turquet::verify::JplVerifier;

const ROOT_LIMIT_SECONDS: f64 = 15.0;
const STATION_LIMIT_SECONDS: f64 = 2.0;
const GREATEST_LIMIT_SECONDS: f64 = 10.0;
const CONTACT_LIMIT_SECONDS: f64 = 25.0;
const INTERVAL_LIMIT_SECONDS: f64 = 1.001;

#[derive(Default)]
struct Metrics {
    comparisons: usize,
    worst_root_seconds: f64,
    worst_station_seconds: f64,
    worst_greatest_seconds: f64,
    worst_contact_seconds: f64,
}

fn main() {
    std::thread::Builder::new()
        .name("turquet-event-verifier".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(run)
        .expect("could not start verifier worker")
        .join()
        .expect("verifier worker panicked");
}

fn run() {
    let kernel = match std::env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: verify_events <kernel.bsp>");
            process::exit(2);
        }
    };
    let verifier = match JplVerifier::open(&kernel) {
        Ok(verifier) => verifier,
        Err(error) => {
            eprintln!("could not open {}: {}", kernel, error);
            process::exit(1);
        }
    };
    println!(
        "provider: {} {}",
        verifier.model().name(),
        verifier.model().revision()
    );
    println!(
        "kernel: {}",
        verifier
            .data_snapshot()
            .expect("the live verifier retains its kernel digest")
    );

    let mut metrics = Metrics::default();
    let result = verify_conjunction(&verifier, &mut metrics)
        .and_then(|_| verify_station(&verifier, &mut metrics))
        .and_then(|_| verify_phases(&verifier, &mut metrics))
        .and_then(|_| verify_candidates(&verifier, &mut metrics))
        .and_then(|_| verify_lunar_circumstances(&verifier, &mut metrics));
    if let Err(detail) = result {
        eprintln!("verification failed: {}", detail);
        process::exit(1);
    }

    println!(
        "verified {} paired event results; worst root {:.3}s; station {:.3}s; greatest {:.3}s; contact {:.3}s",
        metrics.comparisons,
        metrics.worst_root_seconds,
        metrics.worst_station_seconds,
        metrics.worst_greatest_seconds,
        metrics.worst_contact_seconds,
    );
}

fn verify_conjunction(verifier: &JplVerifier, metrics: &mut Metrics) -> Result<(), String> {
    let reference = tt_from_utc(2024, 4, 8, 18, 20, 46.8);
    let window = centered_window(reference, 10.0, 10.0)?;
    let analytical = single(
        ecliptic_longitude_conjunctions(
            &AnalyticalEphemeris,
            ApparentBody::Sun,
            ApparentBody::Moon,
            window,
        )
        .map_err(|error| format!("analytical conjunction: {}", error))?,
        "analytical conjunction",
    )?;
    let live = single(
        ecliptic_longitude_conjunctions(verifier, ApparentBody::Sun, ApparentBody::Moon, window)
            .map_err(|error| format!("live conjunction: {}", error))?,
        "live conjunction",
    )?;
    require_interval(analytical.interval(), "analytical conjunction")?;
    require_interval(live.interval(), "live conjunction")?;
    require_live_snapshot(live.provider_snapshot(), verifier, "conjunction")?;
    let delta = seconds_apart(analytical.interval().midpoint(), live.interval().midpoint());
    require_limit(delta, ROOT_LIMIT_SECONDS, "conjunction provider difference")?;
    metrics.comparisons += 1;
    metrics.worst_root_seconds = metrics.worst_root_seconds.max(delta);
    println!("conjunction 2024-04-08: provider difference {:.3}s", delta);
    Ok(())
}

fn verify_station(verifier: &JplVerifier, metrics: &mut Metrics) -> Result<(), String> {
    let start = tt_from_utc(2024, 4, 25, 0, 0, 0.0);
    let end = tt_from_utc(2024, 4, 26, 0, 0, 0.0);
    let window = SearchWindow::new(start, end, 3.0 / 24.0, 1.0 / 86_400.0)
        .map_err(|error| error.to_string())?;
    let search = StationSearch::new(window, 6.0 / 24.0).map_err(|error| error.to_string())?;
    let analytical = single(
        ecliptic_longitude_stations(&AnalyticalEphemeris, ApparentBody::Mercury, search)
            .map_err(|error| format!("analytical station: {}", error))?,
        "analytical station",
    )?;
    let live = single(
        ecliptic_longitude_stations(verifier, ApparentBody::Mercury, search)
            .map_err(|error| format!("live station: {}", error))?,
        "live station",
    )?;
    if analytical.motion_before() != LongitudeMotion::Retrograde
        || analytical.motion_after() != LongitudeMotion::Direct
        || live.motion_before() != LongitudeMotion::Retrograde
        || live.motion_after() != LongitudeMotion::Direct
    {
        return Err("Mercury station direction disagrees".to_string());
    }
    require_interval(analytical.interval(), "analytical station")?;
    require_interval(live.interval(), "live station")?;
    require_live_snapshot(live.provider_snapshot(), verifier, "station")?;
    let delta = seconds_apart(analytical.interval().midpoint(), live.interval().midpoint());
    require_limit(delta, STATION_LIMIT_SECONDS, "station provider difference")?;
    metrics.comparisons += 1;
    metrics.worst_station_seconds = delta;
    println!(
        "Mercury station 2024-04-25: provider difference {:.3}s",
        delta
    );
    Ok(())
}

fn verify_phases(verifier: &JplVerifier, metrics: &mut Metrics) -> Result<(), String> {
    let cases = [
        (LunarPhase::LastQuarter, (2024, 4, 2, 3, 15, 0.0)),
        (LunarPhase::NewMoon, (2024, 4, 8, 18, 21, 0.0)),
        (LunarPhase::FirstQuarter, (2024, 4, 15, 19, 13, 0.0)),
        (LunarPhase::FullMoon, (2024, 4, 23, 23, 49, 0.0)),
    ];
    for &(expected, parts) in &cases {
        let window = centered_window(tt_from_parts(parts), 10.0, 10.0)?;
        let analytical = single(
            ecliptic_longitude_lunar_phases(&AnalyticalEphemeris, window)
                .map_err(|error| format!("analytical {}: {}", expected.name(), error))?,
            expected.name(),
        )?;
        let live = single(
            ecliptic_longitude_lunar_phases(verifier, window)
                .map_err(|error| format!("live {}: {}", expected.name(), error))?,
            expected.name(),
        )?;
        if analytical.phase() != expected || live.phase() != expected {
            return Err(format!("{} classification disagrees", expected.name()));
        }
        require_interval(analytical.interval(), "analytical lunar phase")?;
        require_interval(live.interval(), "live lunar phase")?;
        require_live_snapshot(live.provider_snapshot(), verifier, expected.name())?;
        let delta = seconds_apart(analytical.interval().midpoint(), live.interval().midpoint());
        require_limit(delta, ROOT_LIMIT_SECONDS, "lunar phase provider difference")?;
        metrics.comparisons += 1;
        metrics.worst_root_seconds = metrics.worst_root_seconds.max(delta);
        println!("{}: provider difference {:.3}s", expected.name(), delta);
    }
    Ok(())
}

fn verify_candidates(verifier: &JplVerifier, metrics: &mut Metrics) -> Result<(), String> {
    let cases = [
        (
            (2024, 3, 25, 7, 0, 0.0),
            Some(EclipseCandidateKind::PenumbralLunar),
        ),
        ((2024, 4, 8, 18, 21, 0.0), Some(EclipseCandidateKind::Solar)),
        ((2024, 4, 23, 23, 49, 0.0), None),
        ((2024, 5, 8, 3, 22, 0.0), None),
        (
            (2024, 9, 18, 2, 34, 0.0),
            Some(EclipseCandidateKind::PartialLunar),
        ),
        (
            (2025, 3, 14, 6, 55, 0.0),
            Some(EclipseCandidateKind::TotalLunar),
        ),
        (
            (2025, 3, 29, 10, 58, 0.0),
            Some(EclipseCandidateKind::Solar),
        ),
    ];
    for &(parts, expected) in &cases {
        let window = centered_window(tt_from_parts(parts), 10.0, 10.0)?;
        let analytical = eclipse_candidates(&AnalyticalEphemeris, window)
            .map_err(|error| format!("analytical eclipse candidate: {}", error))?;
        let live = eclipse_candidates(verifier, window)
            .map_err(|error| format!("live eclipse candidate: {}", error))?;
        match expected {
            None => {
                if !analytical.is_empty() || !live.is_empty() {
                    return Err(format!("ordinary phase {:?} became an eclipse", parts));
                }
                println!("ordinary phase {:?}: rejected by both providers", parts);
            }
            Some(kind) => {
                let analytical = single(analytical, "analytical eclipse candidate")?;
                let live = single(live, "live eclipse candidate")?;
                if analytical.kind() != kind || live.kind() != kind {
                    return Err(format!(
                        "eclipse candidate {:?} classification disagrees",
                        parts
                    ));
                }
                require_interval(analytical.interval(), "analytical eclipse candidate")?;
                require_interval(live.interval(), "live eclipse candidate")?;
                require_live_snapshot(live.provider_snapshot(), verifier, "eclipse candidate")?;
                let delta =
                    seconds_apart(analytical.interval().midpoint(), live.interval().midpoint());
                require_limit(
                    delta,
                    ROOT_LIMIT_SECONDS,
                    "candidate root provider difference",
                )?;
                metrics.comparisons += 1;
                metrics.worst_root_seconds = metrics.worst_root_seconds.max(delta);
                println!(
                    "eclipse candidate {:?}: {:?}, provider difference {:.3}s",
                    parts, kind, delta
                );
            }
        }
    }
    Ok(())
}

fn verify_lunar_circumstances(verifier: &JplVerifier, metrics: &mut Metrics) -> Result<(), String> {
    verify_lunar_case(
        verifier,
        metrics,
        (2024, 3, 25, 7, 0, 14.6),
        0.25,
        LunarEclipseKind::Penumbral,
        &[
            LunarEclipseContactKind::PenumbralIngress,
            LunarEclipseContactKind::PenumbralEgress,
        ],
    )?;
    verify_lunar_case(
        verifier,
        metrics,
        (2024, 9, 18, 2, 34, 22.9),
        0.2,
        LunarEclipseKind::Partial,
        &[
            LunarEclipseContactKind::PenumbralIngress,
            LunarEclipseContactKind::UmbralIngress,
            LunarEclipseContactKind::UmbralEgress,
            LunarEclipseContactKind::PenumbralEgress,
        ],
    )?;
    verify_lunar_case(
        verifier,
        metrics,
        (2025, 3, 14, 6, 54, 33.5),
        0.3,
        LunarEclipseKind::Total,
        &[
            LunarEclipseContactKind::PenumbralIngress,
            LunarEclipseContactKind::UmbralIngress,
            LunarEclipseContactKind::TotalityBegins,
            LunarEclipseContactKind::TotalityEnds,
            LunarEclipseContactKind::UmbralEgress,
            LunarEclipseContactKind::PenumbralEgress,
        ],
    )
}

fn verify_lunar_case(
    verifier: &JplVerifier,
    metrics: &mut Metrics,
    parts: (i32, u8, u8, u8, u8, f64),
    span_days: f64,
    expected_kind: LunarEclipseKind,
    expected_contacts: &[LunarEclipseContactKind],
) -> Result<(), String> {
    let phase_window = centered_window(tt_from_parts(parts), 10.0, 10.0)?;
    let search =
        LunarEclipseSearch::new(phase_window, span_days).map_err(|error| error.to_string())?;
    let analytical = single(
        lunar_eclipse_circumstances(&AnalyticalEphemeris, search)
            .map_err(|error| format!("analytical lunar circumstances: {}", error))?,
        "analytical lunar circumstances",
    )?;
    let live = single(
        lunar_eclipse_circumstances(verifier, search)
            .map_err(|error| format!("live lunar circumstances: {}", error))?,
        "live lunar circumstances",
    )?;
    if analytical.kind() != expected_kind || live.kind() != expected_kind {
        return Err(format!("lunar circumstance {:?} class disagrees", parts));
    }
    let analytical_kinds: Vec<_> = analytical
        .contacts()
        .iter()
        .map(|contact| contact.kind())
        .collect();
    let live_kinds: Vec<_> = live
        .contacts()
        .iter()
        .map(|contact| contact.kind())
        .collect();
    if analytical_kinds != expected_contacts || live_kinds != expected_contacts {
        return Err(format!("lunar circumstance {:?} contacts disagree", parts));
    }
    require_interval(
        analytical.greatest_interval(),
        "analytical greatest eclipse",
    )?;
    require_interval(live.greatest_interval(), "live greatest eclipse")?;
    require_live_snapshot(live.provider_snapshot(), verifier, "lunar circumstances")?;
    let greatest_delta = seconds_apart(
        analytical.greatest_interval().midpoint(),
        live.greatest_interval().midpoint(),
    );
    require_limit(
        greatest_delta,
        GREATEST_LIMIT_SECONDS,
        "greatest-eclipse provider difference",
    )?;
    metrics.comparisons += 1;
    metrics.worst_greatest_seconds = metrics.worst_greatest_seconds.max(greatest_delta);

    let mut worst_contact: f64 = 0.0;
    for (analytical_contact, live_contact) in analytical.contacts().iter().zip(live.contacts()) {
        require_interval(analytical_contact.interval(), "analytical lunar contact")?;
        require_interval(live_contact.interval(), "live lunar contact")?;
        let delta = seconds_apart(
            analytical_contact.interval().midpoint(),
            live_contact.interval().midpoint(),
        );
        require_limit(
            delta,
            CONTACT_LIMIT_SECONDS,
            "lunar-contact provider difference",
        )?;
        worst_contact = worst_contact.max(delta);
        metrics.comparisons += 1;
        metrics.worst_contact_seconds = metrics.worst_contact_seconds.max(delta);
    }
    println!(
        "lunar {:?} {:?}: greatest {:.3}s; worst contact {:.3}s",
        expected_kind, parts, greatest_delta, worst_contact
    );
    Ok(())
}

fn centered_window(
    reference: JulianDate<TerrestrialTime>,
    half_width_minutes: f64,
    step_minutes: f64,
) -> Result<SearchWindow, String> {
    SearchWindow::new(
        reference
            .offset_days(-half_width_minutes / 1_440.0)
            .map_err(|error| format!("window start: {:?}", error))?,
        reference
            .offset_days(half_width_minutes / 1_440.0)
            .map_err(|error| format!("window end: {:?}", error))?,
        step_minutes / 1_440.0,
        1.0 / 86_400.0,
    )
    .map_err(|error| error.to_string())
}

fn single<T>(mut values: Vec<T>, label: &str) -> Result<T, String> {
    if values.len() != 1 {
        return Err(format!("{} returned {} results", label, values.len()));
    }
    Ok(values.remove(0))
}

fn require_interval(interval: EventInterval, label: &str) -> Result<(), String> {
    let seconds = interval.width_days() * 86_400.0;
    require_limit(
        seconds,
        INTERVAL_LIMIT_SECONDS,
        &format!("{} interval", label),
    )
}

fn require_live_snapshot(
    snapshot: Option<&str>,
    verifier: &JplVerifier,
    label: &str,
) -> Result<(), String> {
    if snapshot != verifier.data_snapshot() {
        return Err(format!("{} lost the live kernel snapshot", label));
    }
    Ok(())
}

fn require_limit(value: f64, limit: f64, label: &str) -> Result<(), String> {
    if value > limit {
        Err(format!("{} {:.3}s exceeds {:.3}s", label, value, limit))
    } else {
        Ok(())
    }
}

fn seconds_apart(first: JulianDate<TerrestrialTime>, second: JulianDate<TerrestrialTime>) -> f64 {
    (first.day() - second.day()).abs() * 86_400.0
}

fn tt_from_parts(parts: (i32, u8, u8, u8, u8, f64)) -> JulianDate<TerrestrialTime> {
    tt_from_utc(parts.0, parts.1, parts.2, parts.3, parts.4, parts.5)
}

fn tt_from_utc(
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: f64,
) -> JulianDate<TerrestrialTime> {
    let whole = second.trunc() as u8;
    let nanos = ((second - f64::from(whole)) * 1e9).round() as u32;
    JulianDate::from_epoch(ScaleAwareEpoch::from_gregorian_utc(
        year, month, day, hour, minute, whole, nanos,
    ))
}
