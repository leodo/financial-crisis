$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
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
