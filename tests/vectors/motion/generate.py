#!/usr/bin/env python3
"""Regenerate retained independent motion facts with pyswisseph; no Turquet input."""
import hashlib
import importlib.metadata
from pathlib import Path
import swisseph as swe

HALF_INTERVAL = 0.01
THRESHOLD = 0.001
FLAGS = swe.FLG_MOSEPH | swe.FLG_SPEED


def state(jd, body):
    values, flags = swe.calc(jd, body, FLAGS)  # ET/TT, not UT
    assert flags == FLAGS, (jd, body, flags)
    return values


def jd(year, month, day, hour=0):
    # Calendar fields here are TT, deliberately avoiding a Delta-T/UTC conversion.
    return swe.julday(year, month, day, hour, swe.GREG_CAL)


def root(function, left, right):
    assert function(left) * function(right) < 0
    for _ in range(50):
        mid = (left + right) / 2
        if mid == left or mid == right:
            break
        if function(left) * function(mid) <= 0:
            right = mid
        else:
            left = mid
    return (left + right) / 2


def classification(speed):
    return "NearStation" if abs(speed) <= THRESHOLD else "Direct" if speed > 0 else "Retrograde"


rows = []


def retain(label, body, at):
    before = state(at - HALF_INTERVAL, body)[0]
    center = state(at, body)
    after = state(at + HALF_INTERVAL, body)[0]
    rate = ((after - before + 180) % 360 - 180) / (2 * HALF_INTERVAL)
    # Both the independent Swiss speed and the independently sampled rate must
    # agree on the class of every retained row.
    assert classification(rate) == classification(center[3]), label
    rows.append((label, swe.get_planet_name(body).lower(), at, before, center[0], after,
                 rate, center[3], classification(center[3]), FLAGS))


# All ten bodies at two independent epochs: direct Mercury in August 2026,
# retrograde Mercury during the April 2024 eclipse, and outer retrogrades.
for label, at in [("eclipse_2024", jd(2024, 4, 8, 18)), ("august_2026", jd(2026, 8, 13, 12))]:
    for body in range(swe.SUN, swe.PLUTO + 1):
        retain(label, body, at)

# Independently solve Swiss speed zeros only to select the fixture epochs.
# These are oracle station neighborhoods, not asserted exact Turquet stations.
for label, body, left, right, flank in [
    ("mercury_april_retro_station", swe.MERCURY, jd(2024, 4, 1), jd(2024, 4, 3), 0.25),
    ("mercury_april_direct_station", swe.MERCURY, jd(2024, 4, 25), jd(2024, 4, 26), 0.25),
    ("saturn_station", swe.SATURN, jd(2024, 6, 28), jd(2024, 7, 1), 2.0),
]:
    at = root(lambda t: state(t, body)[3], left, right)
    for suffix, offset in [("before", -flank), ("near", 0), ("after", flank)]:
        retain(label + "_" + suffix, body, at + offset)

# Locate real direct and retrograde Mercury crossings of 0 degrees during
# 2025, then sample within a 0.02-day window straddling the crossing.
start = jd(2025, 1, 1)
wraps = set()
for day in range(365):
    left, right = start + day, start + day + 1
    a, b = state(left, swe.MERCURY)[0], state(right, swe.MERCURY)[0]
    if abs(b - a) <= 300:
        continue
    direction = "direct" if b < a else "retrograde"
    if direction in wraps:
        continue
    at = root(lambda t: (state(t, swe.MERCURY)[0] + 180) % 360 - 180, left, right)
    retain("mercury_" + direction + "_wrap", swe.MERCURY, at)
    wraps.add(direction)
assert wraps == {"direct", "retrograde"}

out = Path(__file__).with_name("swiss.tsv")
lines = [
    "# Swiss Ephemeris " + swe.version + "; pyswisseph " + importlib.metadata.version("pyswisseph"),
    "# Explicit Moshier backend, geocentric apparent tropical true ecliptic/equinox of date; no sidereal/topocentric/J2000/truepos flags.",
    "# swe.calc ET/TT JD; flags requested=returned=260 (FLG_MOSEPH | FLG_SPEED); no ephemeris-file fallback.",
    "# half_interval_tt_days=0.01; near_station_abs_degrees_per_day_inclusive=0.001",
    "# extension_sha256=" + hashlib.sha256(Path(swe.__file__).read_bytes()).hexdigest(),
    "label\tbody\tjd_tt\tbefore_longitude_deg\tlongitude_deg\tafter_longitude_deg\tsampled_deg_per_day\tswiss_speed_deg_per_day\tclass\treturned_flags",
]
for row in rows:
    lines.append("\t".join(format(x, ".15f") if isinstance(x, float) else str(x) for x in row))
out.write_text("\n".join(lines) + "\n")
print(str(out) + ": " + str(len(rows)) + " retained rows; sha256=" + hashlib.sha256(out.read_bytes()).hexdigest())
