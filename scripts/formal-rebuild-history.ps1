param(
    [string]$StartDate = "1990-01-01",
    [string]$EndDate = "2026-05-31",
    [int]$ChunkYears = 1,
    [string]$PointInTimeMode = "best_effort",
    [string]$MarketScope = "financial_system",
    [string]$FeatureSetVersion = "",
    [switch]$ForceRebuild,
    [string]$DatasetVersion = ""
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

$start = [datetime]::Parse($StartDate)
$end = [datetime]::Parse($EndDate)
if ($start -gt $end) {
    throw "StartDate must be earlier than or equal to EndDate."
}
if ($ChunkYears -lt 1) {
    throw "ChunkYears must be at least 1."
}

$cursor = $start
$chunkIndex = 0

while ($cursor -le $end) {
    $chunkIndex += 1
    $chunkEnd = $cursor.AddYears($ChunkYears).AddDays(-1)
    if ($chunkEnd -gt $end) {
        $chunkEnd = $end
    }

    $fromText = $cursor.ToString("yyyy-MM-dd")
    $toText = $chunkEnd.ToString("yyyy-MM-dd")
    Write-Host ""
    Write-Host ("[{0}] feature build {1} -> {2} pit={3}" -f $chunkIndex, $fromText, $toText, $PointInTimeMode)

    $featureArgs = @(
        "run", "-p", "fc-worker", "--",
        "research", "feature", "build",
        "--market-scope", $MarketScope,
        "--from", $fromText,
        "--to", $toText,
        "--point-in-time-mode", $PointInTimeMode
    )

    if ($FeatureSetVersion) {
        $featureArgs += @("--feature-set-version", $FeatureSetVersion)
    }
    if ($ForceRebuild) {
        $featureArgs += "--force-rebuild"
    }

    & cargo @featureArgs

    $cursor = $chunkEnd.AddDays(1)
}

Write-Host ""
Write-Host ("[final] dataset build {0} -> {1} pit={2}" -f $start.ToString("yyyy-MM-dd"), $end.ToString("yyyy-MM-dd"), $PointInTimeMode)

$datasetArgs = @(
    "run", "-p", "fc-worker", "--",
    "research", "dataset", "build-main",
    "--market-scope", $MarketScope,
    "--from", $start.ToString("yyyy-MM-dd"),
    "--to", $end.ToString("yyyy-MM-dd"),
    "--point-in-time-mode", $PointInTimeMode
)

if ($FeatureSetVersion) {
    $datasetArgs += @("--feature-set-version", $FeatureSetVersion)
}

if ($DatasetVersion) {
    $datasetArgs += @("--dataset-version", $DatasetVersion)
}

& cargo @datasetArgs
