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

function Get-ListenerPids {
    param([int[]]$Ports)

    $pids = foreach ($Port in $Ports) {
        Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
            Select-Object -ExpandProperty OwningProcess -Unique |
            Where-Object { $_ -and $_ -ne 0 }
    }
    @($pids | Select-Object -Unique)
}

function Wait-ForListenerPid {
    param(
        [int[]]$Ports,
        [int]$TimeoutSeconds = 20
    )

    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    do {
        $pids = Get-ListenerPids -Ports $Ports
        if ($pids.Count -gt 0) {
            return $pids[0]
        }
        Start-Sleep -Milliseconds 500
    } while ((Get-Date) -lt $deadline)

    return $null
}

function Test-PortBusy {
    param([int[]]$Ports)

    return [bool](Get-ListenerPids -Ports $Ports)
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
        $ExistingPid = Get-Content -LiteralPath $PidFile | Select-Object -First 1
        Write-Host "$Name already running, PID $ExistingPid"
        return
    }

    if (Test-PortBusy -Ports $Ports) {
        $ListenerPids = (Get-ListenerPids -Ports $Ports) -join ","
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

Write-Host "Building API binary..."
Push-Location $Root
cargo build -p fc-api
Pop-Location

if (-not (Test-Path -LiteralPath $ApiBinary)) {
    throw "API binary was not produced at $ApiBinary"
}

$EscapedRoot = $Root.Path.Replace("'", "''")
$EscapedApiLog = $ApiLog.Replace("'", "''")
$EscapedWebLog = $WebLog.Replace("'", "''")
$EscapedApiBinary = $ApiBinary.Replace("'", "''")
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

Start-HiddenService -Name "fc-api" -Command $ApiCommand -PidFile $ApiPidFile -Ports @(18080)
Start-HiddenService -Name "web" -Command $WebCommand -PidFile $WebPidFile -Ports @(5173, 5174)

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
