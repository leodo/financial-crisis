param(
    [string]$MarketScope = "financial_system",
    [string]$OutputDir = "artifacts/research/dataset-summary-check"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$targets = @(
    "formal_v1_main_1990_daily",
    "formal_v1_ext_stress_1990_daily",
    "formal_v1_ext_acute_pre1990"
)

Write-Host "Listing formal datasets for market scope $MarketScope ..."
$listOutput = & cargo run -q -p fc-worker -- research dataset list-main --market-scope $MarketScope 2>$null
if ($LASTEXITCODE -ne 0) {
    $listOutput | ForEach-Object { Write-Host $_ }
    throw "failed to list formal datasets"
}

$latestKeys = @{}
foreach ($line in $listOutput) {
    if ($line -match "^\[(?<key>[^\]]+)\]") {
        $datasetKey = $Matches["key"]
        $parts = $datasetKey.Split(":", 2)
        if ($parts.Length -ne 2) {
            continue
        }
        $datasetId = $parts[0]
        if (-not $latestKeys.ContainsKey($datasetId)) {
            $latestKeys[$datasetId] = $datasetKey
        }
    }
}

foreach ($datasetId in $targets) {
    if (-not $latestKeys.ContainsKey($datasetId)) {
        throw "missing dataset key for $datasetId; run dataset build first"
    }
    $datasetKey = $latestKeys[$datasetId]
    Write-Host "Exporting summary for $datasetKey ..."
    & cargo run -q -p fc-worker -- research dataset summarize-main --market-scope $MarketScope --dataset-key $datasetKey --output-dir $OutputDir 2>$null
    if ($LASTEXITCODE -ne 0) {
        throw "failed to export summary for $datasetKey"
    }
}

Write-Host "Formal dataset summary pack exported to $OutputDir."
