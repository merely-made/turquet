param(
    [ValidateSet('all', 'moon_perigee', 'moon_apogee', 'mars_close')]
    [string]$CaseName = 'all'
)

$ErrorActionPreference = 'Stop'
$invariant = [Globalization.CultureInfo]::InvariantCulture

# These are deliberately broad windows around catalogued candidate extrema. The
# consumer and test independently fit a parabola through the sampled ranges.
$cases = @(
    [PSCustomObject]@{ Name = 'moon_perigee'; Body = 'moon'; Id = '301'; Start = '2024-04-07 00:00'; Stop = '2024-04-09 12:00'; Description = 'April 2024 lunar perigee' },
    [PSCustomObject]@{ Name = 'moon_apogee'; Body = 'moon'; Id = '301'; Start = '2024-04-19 00:00'; Stop = '2024-04-21 12:00'; Description = 'April 2024 lunar apogee' },
    [PSCustomObject]@{ Name = 'mars_close'; Body = 'mars'; Id = '499'; Start = '2022-11-25 00:00'; Stop = '2022-12-08 12:00'; Description = 'Mars opposition distance minimum' }
)
if ($CaseName -ne 'all') { $cases = @($cases | Where-Object Name -eq $CaseName) }
if ($cases.Count -eq 0) { throw "Unknown distance-extrema fixture case: $CaseName" }

function Invoke-Horizons([hashtable]$Parameters) {
    $query = ($Parameters.GetEnumerator() | ForEach-Object {
        '{0}={1}' -f [Uri]::EscapeDataString($_.Key), [Uri]::EscapeDataString($_.Value)
    }) -join '&'
    Invoke-RestMethod -Uri "https://ssd.jpl.nasa.gov/api/horizons.api?$query"
}

function Get-Rows([string]$Response) {
    $inside = $false
    foreach ($line in $Response -split "`n") {
        if ($line.Trim() -eq '$$SOE') { $inside = $true; continue }
        if ($line.Trim() -eq '$$EOE') { break }
        if ($inside -and -not [string]::IsNullOrWhiteSpace($line)) {
            $fields = @($line.Split(',') | ForEach-Object Trim)
            Write-Output (, $fields)
        }
    }
}

$rows = [System.Collections.Generic.List[string]]::new()
$apiVersion = $null
$eopSnapshot = $null
foreach ($case in $cases) {
    $parameters = [ordered]@{
        format = 'text'; COMMAND = "'$($case.Id)'"; OBJ_DATA = "'NO'"
        MAKE_EPHEM = "'YES'"; EPHEM_TYPE = "'OBSERVER'"; CENTER = "'500@399'"
        START_TIME = "'$($case.Start)'"; STOP_TIME = "'$($case.Stop)'"; STEP_SIZE = "'6 h'"
        QUANTITIES = "'20,31,49'"; ANG_FORMAT = "'DEG'"; CAL_FORMAT = "'JD'"
        EXTRA_PREC = "'YES'"; APPARENT = "'AIRLESS'"; CSV_FORMAT = "'YES'"
    }
    $response = Invoke-Horizons $parameters
    if (-not $apiVersion -and $response -match '(?m)^API VERSION:\s*(\S+)') { $apiVersion = $Matches[1] }
    if (-not $eopSnapshot -and $response -match '(?m)^EOP file\s*:\s*(\S+)') { $eopSnapshot = $Matches[1] }
    $sample = 0
    foreach ($fields in Get-Rows $response) {
        if ($fields.Count -lt 8) { throw "Unexpected Horizons row for $($case.Name): $($fields -join ',')" }
        # Quantity 20 emits observer range in AU and range rate in km/s.
        $rows.Add((@($case.Name, $case.Body, $fields[0], $fields[3], $fields[4], $fields[5], $fields[6], $fields[7]) -join "`t"))
        $sample++
    }
    if ($sample -lt 5) { throw "Expected a useful sample window for $($case.Name), got $sample rows" }
}
if (-not $apiVersion -or -not $eopSnapshot) { throw 'Horizons response did not disclose API version and EOP snapshot' }

$generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
@(
    '# Turquet T4k-b distance-extrema vectors',
    "# oracle: NASA/JPL Horizons API $apiVersion, DE441, geocenter 500@399, UTC",
    "# generated: $generated; Horizons response EOP file: $eopSnapshot",
    "# requests: apparent AIRLESS geocentric observer quantities 20,31,49; 6 h cadence",
    '# candidates: Moon perigee 2024-04-07/09, Moon apogee 2024-04-19/21, Mars close approach 2022-11-25/12-08',
    '# method: retain UTC/JD and range; independent consumers may fit a three-point parabola around each sampled minimum or maximum',
    '# columns: case, body, JD UTC, range AU, range rate km/s, observer ecliptic longitude deg, observer ecliptic latitude deg, DUT1 seconds'
)
$rows
