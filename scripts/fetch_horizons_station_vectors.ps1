$ErrorActionPreference = 'Stop'

$parameters = [ordered]@{
    format = 'text'
    COMMAND = "'199'"
    OBJ_DATA = "'NO'"
    MAKE_EPHEM = "'YES'"
    EPHEM_TYPE = "'OBSERVER'"
    CENTER = "'500@399'"
    START_TIME = "'2024-04-24 18:00'"
    STOP_TIME = "'2024-04-26 06:00'"
    STEP_SIZE = "'1h'"
    CAL_FORMAT = "'JD'"
    QUANTITIES = "'20,31'"
    ANG_FORMAT = "'DEG'"
    EXTRA_PREC = "'YES'"
    CSV_FORMAT = "'YES'"
    TIME_TYPE = "'UT'"
}
$query = ($parameters.GetEnumerator() | ForEach-Object {
    '{0}={1}' -f [Uri]::EscapeDataString($_.Key), [Uri]::EscapeDataString($_.Value)
}) -join '&'
$response = Invoke-RestMethod -Uri "https://ssd.jpl.nasa.gov/api/horizons.api?$query"

if ($response -notmatch '(?m)^API VERSION:\s*(\S+)') {
    throw 'Horizons response did not disclose its API version'
}
$apiVersion = $Matches[1]
$rows = [System.Collections.Generic.List[string]]::new()
$inside = $false
foreach ($line in $response -split "`n") {
    if ($line.Trim() -eq '$$SOE') {
        $inside = $true
        continue
    }
    if ($line.Trim() -eq '$$EOE') {
        break
    }
    if (-not $inside -or [string]::IsNullOrWhiteSpace($line)) {
        continue
    }
    $fields = @($line.Split(',') | ForEach-Object Trim)
    if ($fields.Count -lt 7) {
        throw "Unexpected Horizons row: $line"
    }
    $rows.Add((@(
        'mercury',
        $fields[0],
        $fields[5],
        $fields[6],
        $fields[3]
    ) -join "`t"))
}
if ($rows.Count -ne 37) {
    throw "Expected 37 Mercury rows, got $($rows.Count)"
}

$generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
@(
    '# Turquet T4b station vectors',
    "# oracle: NASA/JPL Horizons API $apiVersion, DE441, geocenter 500@399, quantities 20 and 31",
    "# generated: $generated; apparent IAU76/80 ecliptic of date",
    '# sampling: hourly apparent Mercury positions, 2024-04-24 18:00 through 2024-04-26 06:00 UTC',
    '# regenerate: pwsh -File scripts/fetch_horizons_station_vectors.ps1 > tests/vectors/mercury_station_horizons.tsv',
    '# columns: body, JD UTC, apparent longitude degrees, apparent latitude degrees, range AU'
)
$rows
