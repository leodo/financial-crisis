param(
    [string]$CoreStart = "2006-01-01",
    [string]$End = $(Get-Date -Format "yyyy-MM-dd"),
    [string]$SecEdgarStart = "2022-01-01",
    [switch]$IncludeGdelt
)

$ErrorActionPreference = "Stop"

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")

function Invoke-Worker {
    param([string[]]$CommandArgs)

    Write-Host ""
    Write-Host ("==> cargo run -p fc-worker -- {0}" -f ($CommandArgs -join " "))
    Push-Location $Root
    try {
        cargo run -p fc-worker -- $CommandArgs
    } finally {
        Pop-Location
    }
}

Write-Host "Preparing long-range free history for real backtests."
Write-Host ("  CoreStart    : {0}" -f $CoreStart)
Write-Host ("  End          : {0}" -f $End)
Write-Host ("  SecEdgarStart: {0}" -f $SecEdgarStart)
Write-Host ("  IncludeGdelt : {0}" -f $IncludeGdelt.IsPresent)

Invoke-Worker @("db", "init")
Invoke-Worker @("db", "seed")
Invoke-Worker @("backfill", "fred", "--start", $CoreStart, "--end", $End, "--chunk-days", "45")
Invoke-Worker @("backfill", "treasury-yield", "--start", $CoreStart, "--end", $End, "--chunk-days", "120")
Invoke-Worker @("backfill", "boj", "--dataset", "fx-daily", "--start", $CoreStart, "--end", $End, "--chunk-days", "180")
Invoke-Worker @("backfill", "boj", "--dataset", "money-market", "--start", $CoreStart, "--end", $End, "--chunk-days", "180")
Invoke-Worker @("backfill", "world-bank", "--start", "1960-01-01", "--end", $End)
Invoke-Worker @("backfill", "sec-edgar", "--start", $SecEdgarStart, "--end", $End)

if ($IncludeGdelt) {
    $gdeltStart = ([datetime]::ParseExact($End, "yyyy-MM-dd", $null)).AddDays(-89).ToString("yyyy-MM-dd")
    Invoke-Worker @("backfill", "gdelt", "--start", $gdeltStart, "--end", $End, "--watermark-overlap-days", "7")
}

Invoke-Worker @("db", "check")

Write-Host ""
Write-Host "Long-range backfill completed."
Write-Host "You can now restart or reload the API and inspect /api/backtests for real-history coverage."
