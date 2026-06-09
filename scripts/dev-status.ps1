$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$DefaultSqlitePath = Join-Path $Root "data\fc-local.sqlite"
$RunDir = Join-Path $Root ".run"
$ApiPidFile = Join-Path $RunDir "fc-api.pid"
$WebPidFile = Join-Path $RunDir "web.pid"

function Get-ServiceStatus {
    param(
        [string]$Name,
        [string]$PidFile,
        [int]$Port,
        [string]$Url
    )

    $ProcessId = $null
    $Alive = $false

    if (Test-Path -LiteralPath $PidFile) {
        $ProcessId = (Get-Content -LiteralPath $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
        if ($ProcessId) {
            $Alive = [bool](Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)
        }
    }

    $ListenerPids = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty OwningProcess -Unique |
        Where-Object { $_ -and $_ -ne 0 }
    $Listening = [bool]$ListenerPids

    if ($Listening -and $ListenerPids.Count -eq 1) {
        $ProcessId = $ListenerPids[0]
        $Alive = [bool](Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)
        if ($Alive) {
            Set-Content -LiteralPath $PidFile -Value $ProcessId
        }
    }

    [PSCustomObject]@{
        Name = $Name
        RecordedPid = if ($ProcessId) { $ProcessId } else { "-" }
        RecordedAlive = $Alive
        Port = $Port
        Listening = $Listening
        ListenerPid = if ($ListenerPids) { $ListenerPids -join "," } else { "-" }
        Url = $Url
    }
}

Get-ServiceStatus -Name "fc-api" -PidFile $ApiPidFile -Port 18080 -Url "http://127.0.0.1:18080/health"
Get-ServiceStatus -Name "web" -PidFile $WebPidFile -Port 5173 -Url "http://127.0.0.1:5173"

if ((Get-NetTCPConnection -LocalPort 18080 -State Listen -ErrorAction SilentlyContinue)) {
    try {
        $assessment = Invoke-RestMethod -Uri "http://127.0.0.1:18080/api/assessment/current" -Method Get
        $usdJpy = $assessment.key_indicators |
            Where-Object { $_.indicator_id -eq "us_external_usdjpy_level" } |
            Select-Object -First 1
        $latestObservationAt = $assessment.runtime.latest_observation_at
        if (-not $latestObservationAt) {
            $latestObservationAt = "-"
        }
        $latestKeyObservationAt = $assessment.runtime.latest_key_indicator_at
        if (-not $latestKeyObservationAt) {
            $latestKeyObservationAt = $latestObservationAt
        }

        Write-Host ""
        Write-Host "API runtime summary:"
        Write-Host ("  Data mode : {0}" -f $assessment.runtime.data_mode)
        Write-Host ("  As of     : {0}" -f $assessment.as_of_date)
        Write-Host ("  Latest    : {0}" -f $latestObservationAt)
        Write-Host ("  Key latest: {0}" -f $latestKeyObservationAt)
        Write-Host ("  Generated : {0}" -f $assessment.runtime.generated_at)

        if ($usdJpy) {
            Write-Host ("  USDJPY    : {0} @ {1} ({2}/{3}, {4})" -f `
                $usdJpy.latest_value,
                $usdJpy.latest_as_of_date,
                $usdJpy.source_id,
                $usdJpy.dataset_id,
                $usdJpy.status)
        }

        if ($assessment.runtime.data_mode -eq "demo" -and (Test-Path -LiteralPath $DefaultSqlitePath)) {
            Write-Warning "Local SQLite exists at data/fc-local.sqlite, but the API is still serving demo data. Run `just stop` and then `just dev` or `just dev-sqlite`."
        }
    } catch {
        Write-Warning "API is listening, but runtime summary could not be loaded from /api/assessment/current."
    }
}
