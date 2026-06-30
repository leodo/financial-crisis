param(
    [string]$Interval = "PT1H",
    [string]$TaskName = "FinancialCrisis-DataRefresh",
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
$TaskFile = Join-Path $Root ".run\refresh-task.xml"
$LogDir = Join-Path $Root "logs"
$RunDir = Join-Path $Root ".run"

New-Item -ItemType Directory -Force -Path $LogDir, $RunDir | Out-Null

if ($Uninstall) {
    Write-Host "Uninstalling scheduled task '$TaskName'..."
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    Write-Host "Done."
    exit 0
}

# 检查现有任务
$Existing = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
if ($Existing) {
    Write-Host "Task '$TaskName' already exists. Updating..."
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
}

$RefreshCommand = @"
cd /d "$Root" && just refresh-latest && just daily-health-report-save && just deploy-check-save
"@

$Action = New-ScheduledTaskAction -Execute "powershell.exe" -Argument "-NoLogo -NoProfile -ExecutionPolicy Bypass -Command `"$RefreshCommand`" -WindowStyle Hidden"

$Trigger = New-ScheduledTaskTrigger -RepetitionInterval $Interval -At (Get-Date).AddMinutes(1) -RepetitionDuration ([System.TimeSpan]::MaxValue)

$Principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest

$Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Minutes 30)

try {
    Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Principal $Principal -Settings $Settings -Force
    Write-Host ""
    Write-Host "Scheduled task '$TaskName' created successfully."
    Write-Host "  Interval : Every $Interval"
    Write-Host "  Command  : $RefreshCommand"
    Write-Host ""
    Write-Host "To verify: Get-ScheduledTask -TaskName '$TaskName' | Get-ScheduledTaskInfo"
    Write-Host "To remove: .\scripts\schedule-refresh.ps1 -Uninstall"
    Write-Host ""
    Write-Host "IMPORTANT: The task runs as SYSTEM. Make sure credentials and network are available."
} catch {
    Write-Host "ERROR: Failed to register scheduled task. $_"
    Write-Host ""
    Write-Host "Try running as Administrator, or register manually:"
    Write-Host "  1. Open 'Task Scheduler' as Administrator"
    Write-Host "  2. Create a new task with trigger=$Interval"
    Write-Host "  3. Action: start powershell.exe with arguments:"
    Write-Host "     -NoLogo -NoProfile -ExecutionPolicy Bypass -Command `"$RefreshCommand`""
    exit 1
}
