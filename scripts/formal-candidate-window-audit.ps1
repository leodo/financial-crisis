param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$MarketScope = "financial_system",
    [string]$ScenarioId = "us_regional_banks_2023"
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Invoke-FormalCompare {
    param(
        [string]$From,
        [string]$To,
        [string]$Scenario
    )

    $args = @(
        "run", "-p", "fc-worker", "--",
        "research", "release", "formal-probability-compare",
        "--market-scope", $MarketScope,
        "--baseline-release-id", $BaselineReleaseId,
        "--candidate-release-id", $CandidateReleaseId,
        "--from", $From,
        "--to", $To
    )

    if ($Scenario) {
        $args += @("--scenario-id", $Scenario)
    }

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "formal-probability-compare failed for range $From -> $To"
    }
}

Write-Host "Running standard candidate window audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  scope    : $MarketScope"
Write-Host ""

Write-Host "[1/3] Regional banks scenario window"
Invoke-FormalCompare -From "2022-12-01" -To "2023-03-15" -Scenario $ScenarioId
Write-Host ""

Write-Host "[2/3] February false-positive window"
Invoke-FormalCompare -From "2023-02-01" -To "2023-02-15" -Scenario ""
Write-Host ""

Write-Host "[3/3] July false-positive window"
Invoke-FormalCompare -From "2023-07-01" -To "2023-07-20" -Scenario ""
