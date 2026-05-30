$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$RunDir = Join-Path $Root ".run"
$LogDir = Join-Path $Root "logs"
$ApiPidFile = Join-Path $RunDir "fc-api.pid"
$WebPidFile = Join-Path $RunDir "web.pid"
$ApiLog = Join-Path $LogDir "api.log"
$WebLog = Join-Path $LogDir "web.log"

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

function Start-HiddenService {
    param(
        [string]$Name,
        [string]$Command,
        [string]$PidFile
    )

    if (Test-ProcessAlive -PidFile $PidFile) {
        $ExistingPid = Get-Content -LiteralPath $PidFile | Select-Object -First 1
        Write-Host "$Name already running, PID $ExistingPid"
        return
    }

    $Process = Start-Process `
        -FilePath "powershell.exe" `
        -ArgumentList @("-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", $Command) `
        -WorkingDirectory $Root `
        -WindowStyle Hidden `
        -PassThru

    Set-Content -LiteralPath $PidFile -Value $Process.Id
    Write-Host "Started $Name, PID $($Process.Id)"
}

if (-not (Test-Path -LiteralPath (Join-Path $Root "apps/web/node_modules"))) {
    Write-Host "Front-end dependencies not found; running npm install first..."
    Push-Location (Join-Path $Root "apps/web")
    npm install
    Pop-Location
}

$EscapedRoot = $Root.Path.Replace("'", "''")
$EscapedApiLog = $ApiLog.Replace("'", "''")
$EscapedWebLog = $WebLog.Replace("'", "''")

$ApiCommand = @"
Set-Location -LiteralPath '$EscapedRoot'
`$env:FC_API_BIND='127.0.0.1:18080'
if (-not `$env:FC_DATA_MODE) { `$env:FC_DATA_MODE='demo' }
cargo run -p fc-api *>> '$EscapedApiLog'
"@

$WebCommand = @"
Set-Location -LiteralPath '$EscapedRoot\apps\web'
npm run dev *>> '$EscapedWebLog'
"@

Start-HiddenService -Name "fc-api" -Command $ApiCommand -PidFile $ApiPidFile
Start-HiddenService -Name "web" -Command $WebCommand -PidFile $WebPidFile

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

