$ErrorActionPreference = 'Stop'

$cases = @(
    [PSCustomObject]@{
        Name = 'boston_sun'
        Body = 'sun'
        BodyId = '10'
        Coordinates = '-71.0589,42.3601,0.043'
        Start = '2024-04-08 00:00'
        Stop = '2024-04-09 00:00'
    },
    [PSCustomObject]@{
        Name = 'sydney_moon'
        Body = 'moon'
        BodyId = '301'
        Coordinates = '151.2093,-33.8688,0.058'
        Start = '2024-04-08 00:00'
        Stop = '2024-04-09 00:00'
    },
    [PSCustomObject]@{
        Name = 'tromso_sun_empty'
        Body = 'sun'
        BodyId = '10'
        Coordinates = '18.9553,69.6492,0.013'
        Start = '2024-06-21 00:00'
        Stop = '2024-06-22 00:00'
    }
)

function Invoke-Horizons([System.Collections.Specialized.OrderedDictionary]$Parameters) {
    $query = ($Parameters.GetEnumerator() | ForEach-Object {
        '{0}={1}' -f [Uri]::EscapeDataString($_.Key), [Uri]::EscapeDataString($_.Value)
    }) -join '&'
    Invoke-RestMethod -Uri "https://ssd.jpl.nasa.gov/api/horizons.api?$query"
}

function Get-HorizonsRows([string]$Response) {
    $rows = [System.Collections.Generic.List[object]]::new()
    $inside = $false
    foreach ($line in $Response -split "`n") {
        if ($line.Trim() -eq '$$SOE') {
            $inside = $true
            continue
        }
        if ($line.Trim() -eq '$$EOE') {
            break
        }
        if ($inside -and -not [string]::IsNullOrWhiteSpace($line)) {
            $rows.Add(@($line.Split(',') | ForEach-Object Trim))
        }
    }
    $rows
}

$rows = [System.Collections.Generic.List[string]]::new()
$apiVersion = $null
$eopSnapshot = $null

foreach ($case in $cases) {
    $common = [ordered]@{
        format = 'text'
        COMMAND = "'$($case.BodyId)'"
        OBJ_DATA = "'NO'"
        MAKE_EPHEM = "'YES'"
        EPHEM_TYPE = "'OBSERVER'"
        START_TIME = "'$($case.Start)'"
        STOP_TIME = "'$($case.Stop)'"
        STEP_SIZE = "'5 m'"
        ANG_FORMAT = "'DEG'"
        CAL_FORMAT = "'JD'"
        EXTRA_PREC = "'YES'"
        CSV_FORMAT = "'YES'"
        TIME_TYPE = "'UT'"
        APPARENT = "'AIRLESS'"
    }
    $geocentricParameters = [ordered]@{}
    foreach ($entry in $common.GetEnumerator()) {
        $geocentricParameters[$entry.Key] = $entry.Value
    }
    $geocentricParameters.CENTER = "'500@399'"
    $geocentricParameters.QUANTITIES = "'20,31'"
    $geocentricResponse = Invoke-Horizons $geocentricParameters

    $topocentricParameters = [ordered]@{}
    foreach ($entry in $common.GetEnumerator()) {
        $topocentricParameters[$entry.Key] = $entry.Value
    }
    $topocentricParameters.CENTER = "'coord@399'"
    $topocentricParameters.COORD_TYPE = "'GEODETIC'"
    $topocentricParameters.SITE_COORD = "'$($case.Coordinates)'"
    $topocentricParameters.QUANTITIES = "'4,42,49'"
    $topocentricResponse = Invoke-Horizons $topocentricParameters

    foreach ($response in @($geocentricResponse, $topocentricResponse)) {
        if (-not $apiVersion -and $response -match '(?m)^API VERSION:\s*(\S+)') {
            $apiVersion = $Matches[1]
        }
        if (-not $eopSnapshot -and $response -match '(?m)^EOP file\s*:\s*(\S+)') {
            $eopSnapshot = $Matches[1]
        }
    }

    $geocentricRows = @(Get-HorizonsRows $geocentricResponse)
    $topocentricRows = @(Get-HorizonsRows $topocentricResponse)
    if ($geocentricRows.Count -ne 289 -or $topocentricRows.Count -ne 289) {
        throw "Expected 289 rows for $($case.Name), got $($geocentricRows.Count) geocentric and $($topocentricRows.Count) topocentric"
    }

    for ($index = 0; $index -lt $geocentricRows.Count; $index += 1) {
        $geocentric = $geocentricRows[$index]
        $topocentric = $topocentricRows[$index]
        if ($geocentric[0] -ne $topocentric[0]) {
            throw "Mismatched Horizons epochs for $($case.Name) at row $index"
        }
        $rows.Add((@(
            $case.Name,
            $case.Coordinates,
            $geocentric[0],
            $case.Body,
            $geocentric[5],
            $geocentric[6],
            $geocentric[3],
            $topocentric[4],
            $topocentric[5],
            $topocentric[6]
        ) -join "`t"))
    }
}

if (-not $apiVersion -or -not $eopSnapshot) {
    throw 'Horizons response did not disclose its API version and EOP snapshot'
}

$generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
@(
    '# Turquet airless altitude-crossing vectors',
    "# oracle: NASA/JPL Horizons API $apiVersion, DE441, quantities 4,20,31,42,49, APPARENT=AIRLESS",
    "# generated: $generated; EOP $eopSnapshot",
    '# sampling: five-minute UTC grid including both endpoints of each 24-hour case',
    '# sites: user-defined WGS84 geodetic longitude degrees east, latitude degrees north, height km',
    '# cases: Boston Sun ordinary and twilight pairs; Sydney Moon ordinary pair; Tromso Sun midsummer empty control',
    '# polar motion: Horizons applies its EOP pole; Turquet test explicitly approximates xp=yp=0',
    '# regenerate: pwsh -File scripts/fetch_horizons_altitude_crossing_vectors.ps1 > tests/vectors/altitude_crossings_horizons.tsv',
    '# columns: case, site lon/lat/height, JD UTC, body, geocentric apparent longitude degrees, latitude degrees, range AU, direct topocentric altitude degrees, direct local apparent hour angle decimal hours, DUT1 seconds'
)
$rows
