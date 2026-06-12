$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$RunDir = Join-Path $Root ".run"
$LogDir = Join-Path $Root "logs"
$ApiPidFile = Join-Path $RunDir "fc-api.pid"
$WebPidFile = Join-Path $RunDir "web.pid"
$ApiLog = Join-Path $LogDir "api.log"
$WebLog = Join-Path $LogDir "web.log"
$ApiBinary = Join-Path $Root "target\debug\fc-api.exe"

New-Item -ItemType Directory -Force -Path $RunDir, $LogDir | Out-Null

function Test-ProcessAlive {
    param([string]$PidFile)

    if (-not (Test-Path -LiteralPath $PidFile)) {
        return $false
    }

    $ProcessId = (Get-Content -LiteralPath $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if (-not $ProcessId) {
        return $false
    }

    return [bool](Get-Process -Id $ProcessId -ErrorAction SilentlyContinue)
}

function Get-RecordedProcessId {
    param([string]$PidFile)

    if (-not (Test-Path -LiteralPath $PidFile)) {
        return $null
    }

    Get-Content -LiteralPath $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1
}

function Remove-StalePidFile {
    param([string]$PidFile)

    if (-not (Test-Path -LiteralPath $PidFile)) {
        return
    }

    if (Test-ProcessAlive -PidFile $PidFile) {
        return
    }

    Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
}

function Get-ProcessInfo {
    param([int]$ProcessId)

    Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
}

function Test-ProjectProcess {
    param([int]$ProcessId)

    $ProcessInfo = Get-ProcessInfo -ProcessId $ProcessId
    if (-not $ProcessInfo) {
        return $false
    }

    return ($ProcessInfo.CommandLine -like "*$($Root.Path)*") -or ($ProcessInfo.ExecutablePath -like "*$($Root.Path)*")
}

function Stop-ProjectProcessTree {
    param(
        [string]$Name,
        [int]$ProcessId
    )

    $Process = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $Process) {
        return
    }

    if (-not (Test-ProjectProcess -ProcessId $ProcessId)) {
        Write-Host "Refusing to stop PID $ProcessId for $Name because it does not look like this project."
        return
    }

    $Children = Get-CimInstance Win32_Process -Filter "ParentProcessId = $ProcessId" -ErrorAction SilentlyContinue
    foreach ($Child in $Children) {
        Stop-ProjectProcessTree -Name "$Name child" -ProcessId $Child.ProcessId
    }

    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
}

function Get-ListenerPids {
    param([int[]]$Ports)

    $pids = foreach ($Port in $Ports) {
        Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique |
            Where-Object { $_ -and $_ -ne 0 }
    }
    @($pids | Select-Object -Unique)
}

function Get-ActiveListenerPids {
    param([int[]]$Ports)

    @(
        Get-ListenerPids -Ports $Ports |
            Where-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue } |
            Select-Object -Unique
    )
}

function Get-ProjectListenerPids {
    param([int[]]$Ports)

    @(
        Get-ActiveListenerPids -Ports $Ports |
            Where-Object { Test-ProjectProcess -ProcessId $_ } |
            Select-Object -Unique
    )
}

function Get-ForeignListenerPids {
    param([int[]]$Ports)

    @(
        Get-ActiveListenerPids -Ports $Ports |
            Where-Object { -not (Test-ProjectProcess -ProcessId $_) } |
            Select-Object -Unique
    )
}

function Wait-ForListenerPid {
    param(
        [int[]]$Ports,
        [int]$TimeoutSeconds = 20
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        $pids = Get-ActiveListenerPids -Ports $Ports
        if ($pids.Count -gt 0) {
            return $pids[0]
        }
        Start-Sleep -Milliseconds 500
    } while ((Get-Date) -lt $deadline)

    return $null
}

function Wait-ForHttpOk {
    param(
        [string]$Url,
        [int]$TimeoutSeconds = 30,
        [int]$PollMilliseconds = 500
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        try {
            $response = Invoke-WebRequest -Uri $Url -Method Get -TimeoutSec 5 -ErrorAction Stop
            if ($response.StatusCode -ge 200 -and $response.StatusCode -lt 300) {
                return $true
            }
        } catch {
        }

        Start-Sleep -Milliseconds $PollMilliseconds
    } while ((Get-Date) -lt $deadline)

    return $false
}

function Wait-ForPortsReleased {
    param(
        [int[]]$Ports,
        [int]$TimeoutSeconds = 15
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        if (-not (Test-PortBusy -Ports $Ports)) {
            return $true
        }

        Start-Sleep -Milliseconds 500
    } while ((Get-Date) -lt $deadline)

    return $false
}

function Get-ApiRuntimeMetadata {
    param([string]$Url)

    try {
        $assessment = Invoke-RestMethod -Uri $Url -Method Get -TimeoutSec 5 -ErrorAction Stop
        return [PSCustomObject]@{
            DataMode            = [string]$assessment.runtime.data_mode
            AsOfDate            = [string]$assessment.as_of_date
            LatestObservationAt = [string]$assessment.runtime.latest_observation_at
        }
    } catch {
        return $null
    }
}

function Test-PortBusy {
    param([int[]]$Ports)

    return [bool](Get-ActiveListenerPids -Ports $Ports)
}

function Adopt-ProjectListener {
    param(
        [string]$Name,
        [string]$PidFile,
        [int[]]$Ports
    )

    $ProjectListenerPids = Get-ProjectListenerPids -Ports $Ports
    if ($ProjectListenerPids.Count -eq 0) {
        return $false
    }

    $ListenerPid = [int]$ProjectListenerPids[0]
    Set-Content -LiteralPath $PidFile -Value $ListenerPid
    Write-Host "$Name already running, adopted listener PID $ListenerPid"
    return $true
}

function Start-HiddenService {
    param(
        [string]$Name,
        [string]$Command,
        [string]$PidFile,
        [int[]]$Ports
    )

    Remove-StalePidFile -PidFile $PidFile

    if (Test-ProcessAlive -PidFile $PidFile) {
        $ExistingPid = [int](Get-RecordedProcessId -PidFile $PidFile)
        if (Test-PortBusy -Ports $Ports) {
            Write-Host "$Name already running, PID $ExistingPid"
            return
        }

        Write-Host "$Name recorded PID $ExistingPid is alive but not listening; restarting it."
        Stop-ProjectProcessTree -Name "$Name stale process" -ProcessId $ExistingPid
        Remove-Item -LiteralPath $PidFile -Force -ErrorAction SilentlyContinue
    }

    if (Test-PortBusy -Ports $Ports) {
        if (Adopt-ProjectListener -Name $Name -PidFile $PidFile -Ports $Ports) {
            return
        }

        $ListenerPids = (Get-ActiveListenerPids -Ports $Ports) -join ","
        throw "$Name could not start because port(s) $($Ports -join ',') are already in use by PID(s) $ListenerPids. Run `just stop` first."
    }

    $Process = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList @("-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $Command) `
        -WorkingDirectory $Root `
        -WindowStyle Hidden `
        -PassThru

    $ListenerPid = Wait-ForListenerPid -Ports $Ports
    if ($ListenerPid) {
        Set-Content -LiteralPath $PidFile -Value $ListenerPid
        Write-Host "Started $Name, listener PID $ListenerPid"
    } else {
        Set-Content -LiteralPath $PidFile -Value $Process.Id
        Write-Host "Started $Name, bootstrap PID $($Process.Id) but listener PID was not detected in time"
    }
}

if (-not (Test-Path -LiteralPath (Join-Path $Root "apps/web/node_modules"))) {
    Write-Host "Front-end dependencies not found; running npm install first..."
    Push-Location (Join-Path $Root "apps/web")
    npm install
    Pop-Location
}

$ApiPorts = @(18080)
$DefaultSqlitePath = Join-Path $Root "data\fc-local.sqlite"
$ApiDataMode = $env:FC_DATA_MODE
$ApiSqlitePath = $env:FC_SQLITE_PATH

if (-not $ApiDataMode) {
    if (Test-Path -LiteralPath $DefaultSqlitePath) {
        $ApiDataMode = "sqlite"
        if (-not $ApiSqlitePath) {
            $ApiSqlitePath = "data/fc-local.sqlite"
        }
        Write-Host "FC_DATA_MODE is not set; using local SQLite data at $ApiSqlitePath."
        Write-Host "Run just db-check if the panel reports stale or missing key indicators."
    } else {
        $ApiDataMode = "demo"
        Write-Host "FC_DATA_MODE is not set and no local SQLite database was found; using demo data."
    }
} elseif ($ApiDataMode -eq "sqlite" -and -not $ApiSqlitePath) {
    $ApiSqlitePath = "data/fc-local.sqlite"
}

$ForeignApiListeners = Get-ForeignListenerPids -Ports $ApiPorts
if ($ForeignApiListeners.Count -gt 0) {
    $ForeignApiListenerText = $ForeignApiListeners -join ","
    throw "fc-api could not start because port(s) $($ApiPorts -join ',') are already in use by non-project PID(s) $ForeignApiListenerText. Stop those processes or change the bind port first."
}

Remove-StalePidFile -PidFile $ApiPidFile
if (Test-ProcessAlive -PidFile $ApiPidFile) {
    $ExistingApiPid = [int](Get-RecordedProcessId -PidFile $ApiPidFile)
    if (-not (Test-PortBusy -Ports $ApiPorts)) {
        Write-Host "fc-api recorded PID $ExistingApiPid is alive but not listening; stopping stale process before build."
        Stop-ProjectProcessTree -Name "fc-api stale process" -ProcessId $ExistingApiPid
        Remove-Item -LiteralPath $ApiPidFile -Force -ErrorAction SilentlyContinue
    }
}

$ApiAlreadyRunning = Adopt-ProjectListener -Name "fc-api" -PidFile $ApiPidFile -Ports $ApiPorts
if ($ApiAlreadyRunning) {
    $ExistingApiPid = [int](Get-RecordedProcessId -PidFile $ApiPidFile)
    $ExistingRuntime = Get-ApiRuntimeMetadata -Url "http://127.0.0.1:18080/api/assessment/current"
    $RestartReason = $null

    if (-not $ExistingRuntime) {
        $RestartReason = "current runtime could not be inspected"
    } elseif ($ExistingRuntime.DataMode -ne $ApiDataMode) {
        $RestartReason = "current mode is $($ExistingRuntime.DataMode), expected $ApiDataMode"
    }

    if ($RestartReason) {
        Write-Host "Restarting adopted fc-api because $RestartReason."
        Stop-ProjectProcessTree -Name "fc-api mode mismatch" -ProcessId $ExistingApiPid
        Remove-Item -LiteralPath $ApiPidFile -Force -ErrorAction SilentlyContinue
        if (-not (Wait-ForPortsReleased -Ports $ApiPorts -TimeoutSeconds 20)) {
            throw "fc-api was stopped for a mode mismatch, but port $($ApiPorts -join ',') did not release in time."
        }
        $ApiAlreadyRunning = $false
    }
}

if (-not $ApiAlreadyRunning) {
    Write-Host "Building API binary..."
    Push-Location $Root
    cargo build -p fc-api
    Pop-Location
} else {
    Write-Host "fc-api is already running; skipping API rebuild."
}

if (-not (Test-Path -LiteralPath $ApiBinary)) {
    throw "API binary was not produced at $ApiBinary"
}

$EscapedRoot = $Root.Path.Replace("'", "''")
$EscapedApiLog = $ApiLog.Replace("'", "''")
$EscapedWebLog = $WebLog.Replace("'", "''")
$EscapedApiBinary = $ApiBinary.Replace("'", "''")

$EscapedApiDataMode = $ApiDataMode.Replace("'", "''")
$EscapedApiSqlitePath = ""
if ($ApiSqlitePath) {
    $EscapedApiSqlitePath = $ApiSqlitePath.Replace("'", "''")
}

$ApiCommand = @"
`$PSNativeCommandUseErrorActionPreference = `$false
Set-Location -LiteralPath '$EscapedRoot'
`$env:FC_API_BIND='127.0.0.1:18080'
`$env:FC_DATA_MODE='$EscapedApiDataMode'
if ('$EscapedApiSqlitePath' -ne '') { `$env:FC_SQLITE_PATH='$EscapedApiSqlitePath' }
& '$EscapedApiBinary' *>> '$EscapedApiLog'
"@

$WebCommand = @"
`$PSNativeCommandUseErrorActionPreference = `$false
Set-Location -LiteralPath '$EscapedRoot\apps\web'
npm run dev *>> '$EscapedWebLog'
"@

Start-HiddenService -Name "fc-api" -Command $ApiCommand -PidFile $ApiPidFile -Ports $ApiPorts
if (-not (Wait-ForHttpOk -Url "http://127.0.0.1:18080/health" -TimeoutSeconds 30)) {
    throw "fc-api started but did not become healthy within 30 seconds. Check logs/api.log."
}

Start-HiddenService -Name "web" -Command $WebCommand -PidFile $WebPidFile -Ports @(5173, 5174)
if (-not (Wait-ForHttpOk -Url "http://127.0.0.1:5173" -TimeoutSeconds 30)) {
    throw "web started but did not serve http://127.0.0.1:5173 within 30 seconds. Check logs/web.log."
}

Start-Sleep -Seconds 2

Write-Host ""
Write-Host "Local services are starting:"
Write-Host "  API health: http://127.0.0.1:18080/health"
Write-Host "  Web panel : http://127.0.0.1:5173"
Write-Host ""
Write-Host "Useful commands:"
Write-Host "  just status    # check service status"
Write-Host "  just stop      # stop background services"
Write-Host ""
Write-Host "Logs:"
Write-Host "  $ApiLog"
Write-Host "  $WebLog"
