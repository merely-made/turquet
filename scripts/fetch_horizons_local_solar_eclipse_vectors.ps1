param(
    [ValidateSet('all', 'boston_partial', 'dallas_total', 'albuquerque_annular', 'galway_partial', 'cape_town_control')]
    [string]$CaseName = 'all'
)

$ErrorActionPreference = 'Stop'

$cases = @(
    [PSCustomObject]@{
        Name = 'boston_partial'
        Coordinates = '-71.0589,42.3601,0.043'
        Start = '2024-04-08 00:00'
        Stop = '2024-04-09 00:30'
        Description = '2024 total solar eclipse, local partial view'
    },
    [PSCustomObject]@{
        Name = 'dallas_total'
        Coordinates = '-96.7970,32.7767,0.000'
        Start = '2024-04-08 00:00'
        Stop = '2024-04-09 00:30'
        Description = '2024 total solar eclipse, local totality'
    },
    [PSCustomObject]@{
        Name = 'albuquerque_annular'
        Coordinates = '-106.6504,35.0844,0.000'
        Start = '2023-10-14 00:00'
        Stop = '2023-10-15 00:30'
        Description = '2023 annular solar eclipse, local annularity'
    },
    [PSCustomObject]@{
        Name = 'galway_partial'
        Coordinates = '-9.0568,53.2707,0.000'
        Start = '2024-04-08 00:00'
        Stop = '2024-04-09 00:30'
        Description = '2024 total solar eclipse, low-altitude local partial view'
    },
    [PSCustomObject]@{
        Name = 'cape_town_control'
        Coordinates = '18.4241,-33.9249,0.000'
        Start = '2024-04-08 00:00'
        Stop = '2024-04-09 00:30'
        Description = '2024 eclipse date, outside the local eclipse footprint'
    }
)

if ($CaseName -ne 'all') {
    $cases = @($cases | Where-Object Name -eq $CaseName)
}
if ($cases.Count -eq 0) {
    throw "Unknown eclipse fixture case: $CaseName"
}

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

$bodyDefinitions = @(
    [PSCustomObject]@{ Name = 'sun'; Id = '10' },
    [PSCustomObject]@{ Name = 'moon'; Id = '301' }
)
$rows = [System.Collections.Generic.List[string]]::new()
$apiVersions = [System.Collections.Generic.HashSet[string]]::new()
$eopSnapshots = [System.Collections.Generic.HashSet[string]]::new()

foreach ($case in $cases) {
    foreach ($body in $bodyDefinitions) {
        $common = [ordered]@{
            format = 'text'
            COMMAND = "'$($body.Id)'"
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
            if ($response -match '(?m)^API VERSION:\s*(\S+)') {
                [void]$apiVersions.Add($Matches[1])
            }
            if ($response -match '(?m)^EOP file\s*:\s*(\S+)') {
                [void]$eopSnapshots.Add($Matches[1])
            }
        }

        $geocentricRows = @(Get-HorizonsRows $geocentricResponse)
        $topocentricRows = @(Get-HorizonsRows $topocentricResponse)
        if ($geocentricRows.Count -ne 295 -or $topocentricRows.Count -ne 295) {
            throw "Expected 295 five-minute rows for $($case.Name)/$($body.Name), got $($geocentricRows.Count) geocentric and $($topocentricRows.Count) topocentric"
        }

        for ($index = 0; $index -lt $geocentricRows.Count; $index += 1) {
            $geocentric = $geocentricRows[$index]
            $topocentric = $topocentricRows[$index]
            if ($geocentric.Count -lt 7 -or $topocentric.Count -lt 7) {
                throw "Unexpected Horizons row for $($case.Name)/$($body.Name) at index $index"
            }
            if ($geocentric[0] -ne $topocentric[0]) {
                throw "Mismatched Horizons epochs for $($case.Name)/$($body.Name) at row $index"
            }
            $rows.Add((@(
                $case.Name,
                $case.Coordinates,
                $geocentric[0],
                $body.Name,
                $geocentric[5],
                $geocentric[6],
                $geocentric[3],
                $topocentric[3],
                $topocentric[4],
                $topocentric[5],
                $topocentric[6]
            ) -join "`t"))
        }
    }
}

if ($apiVersions.Count -eq 0 -or $eopSnapshots.Count -eq 0) {
    throw 'Horizons response did not disclose its API version and EOP snapshot'
}

$generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
@(
    '# Turquet T4j local solar-eclipse observer vectors',
    '# source: NASA/JPL Horizons API https://ssd.jpl.nasa.gov/api/horizons.api',
    "# oracle: Horizons API $($apiVersions -join ', '), DE441, AIRLESS apparent observer ephemerides",
    "# EOP: Horizons EOP file $($eopSnapshots -join ', '); DUT1 is emitted per row in the final column",
    "# generated: $generated; regeneration date is informational, request parameters and source are fixed below",
    '# requests: geocenter 500@399 with quantities 20,31; site center coord@399 with GEODETIC coordinates and quantities 4,42,49',
    '# sampling: five-minute UTC grid including both endpoints of each 24.5-hour case; CAL_FORMAT=JD, TIME_TYPE=UT',
    '# sites: WGS84 geodetic longitude degrees east, latitude degrees north, height km',
    '# cases: Boston partial, Dallas total, Albuquerque annular, Galway low-altitude partial, Cape Town outside-footprint control',
    '# model note: Horizons applies its EOP pole; consumers approximating polar motion as zero must state that assumption',
    '# regenerate: pwsh -NoProfile -File scripts/fetch_horizons_local_solar_eclipse_vectors.ps1 > tests/vectors/local_solar_eclipse_horizons.tsv',
    '# select one case: pwsh -NoProfile -File scripts/fetch_horizons_local_solar_eclipse_vectors.ps1 -CaseName dallas_total',
    '# columns: case, site lon/lat/height, JD UTC, body, geocentric apparent longitude degrees, latitude degrees, range AU, direct topocentric azimuth degrees, altitude degrees, local apparent hour angle decimal hours, DUT1 seconds'
)
$rows
