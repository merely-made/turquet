$ErrorActionPreference = 'Stop'

$invariant = [Globalization.CultureInfo]::InvariantCulture
$references = @(
    [PSCustomObject]@{ Name = 'new'; Time = '2024-04-08 18:21' },
    [PSCustomObject]@{ Name = 'first-quarter'; Time = '2024-04-15 19:13' },
    [PSCustomObject]@{ Name = 'full'; Time = '2024-04-23 23:49' }
)
$instants = foreach ($reference in $references) {
    $epoch = [DateTimeOffset]::ParseExact($reference.Time, 'yyyy-MM-dd HH:mm', $invariant,
        [Globalization.DateTimeStyles]::AssumeUniversal).ToUniversalTime()
    foreach ($offset in @(-12, -6, 0, 6, 12)) {
        [PSCustomObject]@{ Case = $reference.Name; Instant = $epoch.AddHours($offset) }
    }
}
$times = @($instants | ForEach-Object {
    (2440587.5 + $_.Instant.ToUnixTimeMilliseconds() / 86400000.0).ToString('F9', $invariant)
})

function Invoke-Horizons([string] $command, [string] $quantities, [DateTimeOffset] $start, [DateTimeOffset] $stop) {
    $parameters = [ordered]@{
        format = 'text'; COMMAND = "'$command'"
        EPHEM_TYPE = "'OBSERVER'"; CENTER = "'500@399'"
        START_TIME = "'$($start.ToString('yyyy-MM-dd HH:mm', $invariant))'"
        STOP_TIME = "'$($stop.ToString('yyyy-MM-dd HH:mm', $invariant))'"; STEP_SIZE = "'6h'"
        QUANTITIES = "'$quantities'"; ANG_FORMAT = "'DEG'"; EXTRA_PREC = "'YES'"
        CSV_FORMAT = "'YES'"
    }
    $query = ($parameters.GetEnumerator() | ForEach-Object {
        '{0}={1}' -f [Uri]::EscapeDataString($_.Key), [Uri]::EscapeDataString($_.Value)
    }) -join '&'
    Invoke-RestMethod -Uri "https://ssd.jpl.nasa.gov/api/horizons.api?$query"
}

function Get-Rows([string] $body, [string] $quantities, [scriptblock] $convert) {
    $row = 0
    foreach ($reference in $references) {
        $epoch = [DateTimeOffset]::ParseExact($reference.Time, 'yyyy-MM-dd HH:mm', $invariant,
            [Globalization.DateTimeStyles]::AssumeUniversal).ToUniversalTime()
        $response = Invoke-Horizons $body $quantities $epoch.AddHours(-12) $epoch.AddHours(12)
    if (-not $script:apiVersion -and $response -match '(?m)^API VERSION:\s*(\S+)') {
        $script:apiVersion = $Matches[1]
    }
    $inside = $false; $caseRow = 0
    foreach ($line in $response -split "`n") {
        if ($line.Trim() -eq '$$SOE') { $inside = $true; continue }
        if ($line.Trim() -eq '$$EOE') { break }
        if (-not $inside -or [string]::IsNullOrWhiteSpace($line)) { continue }
        $fields = @($line.Split(',') | ForEach-Object Trim)
        if ($fields.Count -lt 7) { throw "Unexpected Horizons row: $line" }
        if ($caseRow -ge 5) { throw "Too many Horizons rows for $body/$($reference.Name)" }
        & $convert (($references.IndexOf($reference) * 5) + $caseRow) $fields
        $caseRow++
    }
    if ($caseRow -ne 5) { throw "Expected 5 rows for $body/$($reference.Name), got $caseRow" }
    $row += $caseRow
    }
    if ($row -ne $times.Count) { throw "Expected $($times.Count) rows for $body, got $row" }
}

$script:apiVersion = $null
$rows = [System.Collections.Generic.List[string]]::new()
Get-Rows '301' '2,10,20,23,24,29,31,32' {
    param($i, $f)
    $rows.Add((@($instants[$i].Case, 'moon', $times[$i], $f[3], $f[4], $f[5], $f[6], $f[10], $f[8], $f[12], $f[13]) -join "`t"))
}
Get-Rows '10' '2,20,31' {
    param($i, $f)
    $rows.Add((@($instants[$i].Case, 'sun', $times[$i], $f[3], $f[4], '', $f[5], '', '', $f[7], $f[8]) -join "`t"))
}
if (-not $script:apiVersion) { throw 'Horizons response did not disclose its API version' }

$generated = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd')
@(
    '# Turquet T4k-a lunar illumination vectors',
    "# oracle: NASA/JPL Horizons API $script:apiVersion, DE441, geocenter 500@399, UTC",
    "# generated: $generated; Horizons response EOP file: eop.260825.p261121",
    '# requests: Moon quantities 2,10,20,23,24,29,31,32; Sun quantities 2,20,31; apparent IAU76/80 ecliptic of date',
    '# references: NASA GSFC phase catalog, 2024-04-08 18:21 new, 2024-04-15 19:13 first quarter, 2024-04-23 23:49 full UT',
    '# sampling: each reference at -12h, -6h, event minute, +6h, +12h; regenerate with this script',
    '# columns: case, body, JD UTC, RA deg, DEC deg, illumination percent, range AU, phase angle deg, solar elongation deg, ecliptic longitude deg, ecliptic latitude deg'
)
$rows
