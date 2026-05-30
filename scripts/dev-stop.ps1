$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$RootPath = $Root.Path
$RunDir = Join-Path $Root ".run"
$PidFiles = @(
    @{ Name = "fc-api"; Path = Join-Path $RunDir "fc-api.pid"; Ports = @(18080) },
    @{ Name = "web"; Path = Join-Path $RunDir "web.pid"; Ports = @(5173, 5174) }
)

function Get-ProcessInfo {
    param([int]$ProcessId)

    return Get-CimInstance Win32_Process -Filter "ProcessId = $ProcessId" -ErrorAction SilentlyContinue
}

function Test-ProjectProcess {
    param([int]$ProcessId)

    $ProcessInfo = Get-ProcessInfo -ProcessId $ProcessId
    if (-not $ProcessInfo) {
        return $false
    }

    return ($ProcessInfo.CommandLine -like "*$RootPath*") -or ($ProcessInfo.ExecutablePath -like "*$RootPath*")
}

function Stop-ProjectProcessTree {
    param(
        [string]$Name,
        [int]$ProcessId
    )

    $Process = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $Process) {
        Write-Host "$Name was already stopped."
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

    Stop-Process -Id $ProcessId -Force
    Write-Host "Stopped $Name, PID $ProcessId"
}

function Stop-RecordedProcess {
    param(
        [string]$Name,
        [string]$PidFile
    )

    if (-not (Test-Path -LiteralPath $PidFile)) {
        Write-Host "$Name is not recorded as running."
        return
    }

    $ProcessId = (Get-Content -LiteralPath $PidFile -ErrorAction SilentlyContinue | Select-Object -First 1)
    if (-not $ProcessId) {
        Remove-Item -LiteralPath $PidFile -Force
        Write-Host "$Name pid file was empty; removed it."
        return
    }

    $Process = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $Process) {
        Remove-Item -LiteralPath $PidFile -Force
        Write-Host "$Name was already stopped."
        return
    }

    Stop-ProjectProcessTree -Name $Name -ProcessId $ProcessId
    Remove-Item -LiteralPath $PidFile -Force
}

function Stop-PortListeners {
    param(
        [string]$Name,
        [int]$Port
    )

    $Listeners = Get-NetTCPConnection -LocalPort $Port -State Listen -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty OwningProcess -Unique |
        Where-Object { $_ -and $_ -ne 0 }

    foreach ($ListenerPid in $Listeners) {
        Stop-ProjectProcessTree -Name "$Name listener on port $Port" -ProcessId $ListenerPid
    }
}

foreach ($Item in $PidFiles) {
    Stop-RecordedProcess -Name $Item.Name -PidFile $Item.Path
    foreach ($Port in $Item.Ports) {
        Stop-PortListeners -Name $Item.Name -Port $Port
    }
}
