param(
    [ValidateSet('all', 'boston', 'sydney', 'tromso')]
    [string]$SiteName = 'all'
)

$ErrorActionPreference = 'Stop'

$sites = @(
    [PSCustomObject]@{ Name = 'boston'; Coordinates = '-71.0589,42.3601,0.043' },
    [PSCustomObject]@{ Name = 'sydney'; Coordinates = '151.2093,-33.8688,0.058' },
    [PSCustomObject]@{ Name = 'tromso'; Coordinates = '18.9553,69.6492,0.013' }
)
$bodies = @(
    [PSCustomObject]@{ Name = 'sun'; Id = '10' },
    [PSCustomObject]@{ Name = 'moon'; Id = '301' },
    [PSCustomObject]@{ Name = 'mercury'; Id = '199' },
    [PSCustomObject]@{ Name = 'venus'; Id = '299' },
    [PSCustomObject]@{ Name = 'mars'; Id = '499' },
    [PSCustomObject]@{ Name = 'jupiter'; Id = '599' },
    [PSCustomObject]@{ Name = 'saturn'; Id = '699' },
    [PSCustomObject]@{ Name = 'uranus'; Id = '799' },
    [PSCustomObject]@{ Name = 'neptune'; Id = '899' },
    [PSCustomObject]@{ Name = 'pluto'; Id = '999' }
)
$times = @(
    [PSCustomObject]@{ Label = '2000-01-01T12:00:00Z'; JulianUtc = '2451545.0' },
    [PSCustomObject]@{ Label = '2024-04-08T18:00:00Z'; JulianUtc = '2460409.25' },
    [PSCustomObject]@{ Label = '2026-08-13T12:00:00Z'; JulianUtc = '2461266.0' }
)

if ($SiteName -ne 'all') {
    $sites = @($sites | Where-Object Name -eq $SiteName)
}

$rows = [System.Collections.Generic.List[string]]::new()
$apiVersion = $null
$eopSnapshot = $null

foreach ($site in $sites) {
    foreach ($body in $bodies) {
        $parameters = [ordered]@{
            format = 'text'
            COMMAND = "'$($body.Id)'"
            OBJ_DATA = "'NO'"
            MAKE_EPHEM = "'YES'"
            EPHEM_TYPE = "'OBSERVER'"
            CENTER = "'coord@399'"
            COORD_TYPE = "'GEODETIC'"
            SITE_COORD = "'$($site.Coordinates)'"
            TLIST = "'$($times.JulianUtc -join ',')'"
            QUANTITIES = "'2,4,20,49'"
            ANG_FORMAT = "'DEG'"
            EXTRA_PREC = "'YES'"
            APPARENT = "'AIRLESS'"
            CSV_FORMAT = "'YES'"
        }
        $query = ($parameters.GetEnumerator() | ForEach-Object {
            '{0}={1}' -f [Uri]::EscapeDataString($_.Key), [Uri]::EscapeDataString($_.Value)
        }) -join '&'
        $response = Invoke-RestMethod -Uri "https://ssd.jpl.nasa.gov/api/horizons.api?$query"
        if (-not $apiVersion -and $response -match '(?m)^API VERSION:\s*(\S+)') {
            $apiVersion = $Matches[1]
        }
        if (-not $eopSnapshot -and $response -match '(?m)^EOP file\s*:\s*(\S+)') {
            $eopSnapshot = $Matches[1]
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
            if ($fields.Count -lt 10) {
                throw "Unexpected Horizons row: $line"
            }
            $time = $times[$row]
            $rows.Add((@(
                $site.Name,
                $site.Coordinates,
                $time.Label,
                $time.JulianUtc,
                $body.Name,
                $fields[3],
                $fields[4],
                $fields[5],
                $fields[6],
                $fields[7],
                $fields[9]
            ) -join "`t"))
            $row += 1
        }
        if ($row -ne $times.Count) {
            throw "Expected $($times.Count) rows for $($site.Name)/$($body.Name), got $row"
        }
    }
}

if (-not $apiVersion -or -not $eopSnapshot) {
    throw 'Horizons response did not disclose its API version and EOP snapshot'
}

$generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
@(
    '# Turquet observer vectors',
    '# oracle: NASA/JPL Horizons API, DE441, quantities 2,4,20,49, AIRLESS',
    "# generated: $generated; API $apiVersion; EOP $eopSnapshot",
    '# sites: user-defined WGS84 geodetic longitude degrees east, latitude degrees north, height km',
    '# polar motion: Horizons applies its EOP pole; Turquet test explicitly approximates xp=yp=0',
    '# regenerate: pwsh -File scripts/fetch_horizons_observer_vectors.ps1 > tests/vectors/observer_horizons.tsv',
    '# columns: site, site lon/lat/height, UTC, JD UTC, body, RA, Dec, azimuth, altitude, range AU, DUT1 seconds'
)
$rows
