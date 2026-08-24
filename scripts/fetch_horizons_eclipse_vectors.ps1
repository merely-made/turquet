$ErrorActionPreference = 'Stop'

$invariant = [Globalization.CultureInfo]::InvariantCulture
$references = @(
    '2024-03-25 07:00',
    '2024-04-08 18:21',
    '2024-04-23 23:49',
    '2024-05-08 03:22',
    '2024-09-18 02:34',
    '2025-03-14 06:55',
    '2025-03-29 10:58'
) | ForEach-Object {
    [DateTimeOffset]::ParseExact(
        $_,
        'yyyy-MM-dd HH:mm',
        $invariant,
        [Globalization.DateTimeStyles]::AssumeUniversal
    ).ToUniversalTime()
}
$instants = foreach ($reference in $references) {
    $reference.AddMinutes(-10)
    $reference
    $reference.AddMinutes(10)
}
$times = @($instants | ForEach-Object {
    $julianDay = 2440587.5 + $_.ToUnixTimeMilliseconds() / 86400000.0
    $julianDay.ToString('F9', $invariant)
})
$bodies = @(
    [PSCustomObject]@{ Name = 'sun'; Id = '10' },
    [PSCustomObject]@{ Name = 'moon'; Id = '301' }
)
$rows = [System.Collections.Generic.List[string]]::new()
$apiVersion = $null

foreach ($body in $bodies) {
    $parameters = [ordered]@{
        format = 'text'
        COMMAND = "'$($body.Id)'"
        OBJ_DATA = "'NO'"
        MAKE_EPHEM = "'YES'"
        EPHEM_TYPE = "'OBSERVER'"
        CENTER = "'500@399'"
        TLIST = "'$($times -join ',')'"
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
    if (-not $apiVersion -and $response -match '(?m)^API VERSION:\s*(\S+)') {
        $apiVersion = $Matches[1]
    }

    $inside = $false
    $row = 0
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
            $body.Name,
            $times[$row],
            $fields[5],
            $fields[6],
            $fields[3]
        ) -join "`t"))
        $row += 1
    }
    if ($row -ne $times.Count) {
        throw "Expected $($times.Count) rows for $($body.Name), got $row"
    }
}

if (-not $apiVersion) {
    throw 'Horizons response did not disclose its API version'
}

$generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
@(
    '# Turquet T4d eclipse candidate geometry vectors',
    "# oracle: NASA/JPL Horizons API $apiVersion, DE441, geocenter 500@399, quantities 20 and 31",
    "# generated: $generated; apparent IAU76/80 ecliptic of date",
    '# sampling: ten minutes before, at, and after each NASA GSFC phase-catalog minute',
    '# reference: NASA GSFC 2024-03-25 penumbral lunar, 04-08 total solar, 04-23 ordinary full moon, 05-08 ordinary new moon, 09-18 partial lunar; 2025-03-14 total lunar, 03-29 partial solar',
    '# regenerate: pwsh -File scripts/fetch_horizons_eclipse_vectors.ps1 > tests/vectors/eclipse_geometry_horizons.tsv',
    '# columns: body, JD UTC, apparent longitude degrees, apparent latitude degrees, range AU'
)
$rows
