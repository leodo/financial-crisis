param(
    [switch]$Tracked,
    [string]$MarketScope = "financial_system",
    [string]$ModelShape = "family_conditional_v1",
    [string]$PrimaryDatasetId = "formal_v1_main_1990_daily",
    [string[]]$AuxDatasetIds = @(
        "formal_v1_ext_stress_1990_daily",
        "formal_v1_ext_acute_pre1990"
    )
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Resolve-FormalDatasetKey {
    param(
        [string]$DatasetId,
        [string]$Scope
    )

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & cargo run --quiet -p fc-worker -- research dataset list-main `
            --market-scope $Scope `
            --dataset-id $DatasetId `
            --limit 1 2>&1 | Out-String
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if ($LASTEXITCODE -ne 0) {
        throw "Unable to resolve latest dataset key for $DatasetId.`n$output"
    }

    $line = ($output -split "\r?\n" | Where-Object { $_ -match '^\[(.+?)\]' } | Select-Object -First 1)
    if (-not $line) {
        throw "Dataset list output for $DatasetId did not contain a [dataset_key] line.`n$output"
    }

    $match = [regex]::Match($line, '^\[(.+?)\]')
    if (-not $match.Success) {
        throw "Failed to parse dataset key from line: $line"
    }

    $match.Groups[1].Value
}

$primaryKey = Resolve-FormalDatasetKey -DatasetId $PrimaryDatasetId -Scope $MarketScope
$auxKeys = foreach ($datasetId in $AuxDatasetIds) {
    Resolve-FormalDatasetKey -DatasetId $datasetId -Scope $MarketScope
}

Write-Host "Resolved dataset keys:"
Write-Host "  primary: $primaryKey"
foreach ($key in $auxKeys) {
    Write-Host "  aux    : $key"
}
Write-Host ""

$args = @(
    "run", "-p", "fc-worker", "--",
    "research", "pipeline", "train-probability",
    "--market-scope", $MarketScope,
    "--model-shape", $ModelShape,
    "--dataset-key", $primaryKey
)

foreach ($key in $auxKeys) {
    $args += @("--aux-dataset-key", $key)
}

if ($Tracked) {
    $args += @(
        "--output-dir", "config/model-bundles/generated",
        "--manifest-dir", "config/model-releases/generated"
    )
}

& cargo @args
exit $LASTEXITCODE
