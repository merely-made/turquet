$ErrorActionPreference = 'Stop'

$bodies = @(
    [PSCustomObject]@{ Name = 'sun'; Id = '10' },
    [PSCustomObject]@{ Name = 'moon'; Id = '301' }
)
$times = @(
    '2460409.256944444',
    '2460409.263888889',
    '2460409.270833333'
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
    '# Turquet T4a conjunction vectors',
    "# oracle: NASA/JPL Horizons API $apiVersion, DE441, geocenter 500@399, quantities 20 and 31",
    "# generated: $generated; apparent IAU76/80 ecliptic of date",
    '# reference: NASA GSFC 2024-04-08 ecliptic conjunction 18:20:46.8 UT',
    '# regenerate: pwsh -File scripts/fetch_horizons_conjunction_vectors.ps1 > tests/vectors/eclipse_conjunction_horizons.tsv',
    '# columns: body, JD UTC, apparent longitude degrees, apparent latitude degrees, range AU'
)
$rows
