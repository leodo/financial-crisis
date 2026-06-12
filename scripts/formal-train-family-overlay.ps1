param(
    [switch]$Tracked,
    [switch]$DryRun,
    [string]$MarketScope = "financial_system",
    [string]$ModelShape = "family_conditional_v1",
    [string]$ReleasePrefix = "",
    [string]$OutputDir = "",
    [string]$ManifestDir = "",
    [string]$LogDir = "artifacts/research/formal-training-logs",
    [switch]$NoLog,
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

function Resolve-RepoPath {
    param([string]$Path)

    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return $Path
    }

    return Join-Path $Root $Path
}

function Format-CommandLine {
    param([string[]]$CommandArgs)

    $quotedArgs = foreach ($arg in $CommandArgs) {
        if ($arg -match '[\s"`$]') {
            '"' + ($arg -replace '"', '\"') + '"'
        } else {
            $arg
        }
    }

    "cargo " + ($quotedArgs -join " ")
}

function Get-NewTrainingArtifacts {
    param(
        [string[]]$Directories,
        [datetime]$StartedAt
    )

    foreach ($directory in $Directories) {
        $resolved = Resolve-RepoPath -Path $directory
        if (-not $resolved -or -not (Test-Path -LiteralPath $resolved)) {
            continue
        }

        Get-ChildItem -LiteralPath $resolved -File -Filter "*.json" |
            Where-Object { $_.LastWriteTime -ge $StartedAt.AddSeconds(-2) } |
            Sort-Object LastWriteTime, Name
    }
}

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

$effectiveOutputDir = if (-not [string]::IsNullOrWhiteSpace($OutputDir)) {
    $OutputDir
} elseif ($Tracked) {
    "config/model-bundles/generated"
} else {
    "artifacts/research/model-bundles/generated"
}

$effectiveManifestDir = if (-not [string]::IsNullOrWhiteSpace($ManifestDir)) {
    $ManifestDir
} elseif ($Tracked) {
    "config/model-releases/generated"
} else {
    "artifacts/research/model-releases/generated"
}

$cargoArgs = @(
    "run", "-p", "fc-worker", "--",
    "research", "pipeline", "train-probability",
    "--market-scope", $MarketScope,
    "--model-shape", $ModelShape,
    "--dataset-key", $primaryKey
)

foreach ($key in $auxKeys) {
    $cargoArgs += @("--aux-dataset-key", $key)
}

if ($DryRun) {
    $cargoArgs += @("--dry-run")
}

if ($Tracked -or -not [string]::IsNullOrWhiteSpace($OutputDir)) {
    $cargoArgs += @("--output-dir", $effectiveOutputDir)
}

if ($Tracked -or -not [string]::IsNullOrWhiteSpace($ManifestDir)) {
    $cargoArgs += @("--manifest-dir", $effectiveManifestDir)
}

if (-not [string]::IsNullOrWhiteSpace($ReleasePrefix)) {
    $cargoArgs += @("--release-prefix", $ReleasePrefix)
}

$startedAt = Get-Date
$commandLine = Format-CommandLine -CommandArgs $cargoArgs
Write-Host "Training command:"
Write-Host "  $commandLine"
Write-Host "Expected artifact directories:"
Write-Host "  bundles  : $effectiveOutputDir"
Write-Host "  manifests: $effectiveManifestDir"

$logPath = $null
if (-not $NoLog) {
    $resolvedLogDir = Resolve-RepoPath -Path $LogDir
    New-Item -ItemType Directory -Path $resolvedLogDir -Force | Out-Null
    $mode = if ($DryRun) { "dry-run" } else { "train" }
    $safeModelShape = $ModelShape -replace '[^A-Za-z0-9_.-]', '_'
    $timestamp = (Get-Date).ToUniversalTime().ToString("yyyyMMddTHHmmssZ")
    $logPath = Join-Path $resolvedLogDir "$timestamp-$safeModelShape-$mode.log"
    Write-Host "Log file:"
    Write-Host "  $logPath"
    Write-Host ""
    "Started: $($startedAt.ToUniversalTime().ToString("o"))" | Set-Content -LiteralPath $logPath -Encoding utf8
    "Command: $commandLine" | Add-Content -LiteralPath $logPath -Encoding utf8
    "" | Add-Content -LiteralPath $logPath -Encoding utf8
    & cargo @cargoArgs 2>&1 | Tee-Object -FilePath $logPath -Append
} else {
    Write-Host ""
    & cargo @cargoArgs
}

$exitCode = $LASTEXITCODE
$finishedAt = Get-Date
$elapsed = New-TimeSpan -Start $startedAt -End $finishedAt

Write-Host ""
Write-Host ("Training command finished with exit code {0} after {1:n1}s." -f $exitCode, $elapsed.TotalSeconds)
if ($logPath) {
    "Finished: $($finishedAt.ToUniversalTime().ToString("o"))" | Add-Content -LiteralPath $logPath -Encoding utf8
    "ExitCode: $exitCode" | Add-Content -LiteralPath $logPath -Encoding utf8
    "ElapsedSeconds: $([Math]::Round($elapsed.TotalSeconds, 1))" | Add-Content -LiteralPath $logPath -Encoding utf8
}

if (-not $DryRun) {
    $newArtifacts = @(
        Get-NewTrainingArtifacts -Directories @($effectiveOutputDir, $effectiveManifestDir) -StartedAt $startedAt
    )
    if ($newArtifacts.Count -gt 0) {
        Write-Host "New or updated training artifacts:"
        foreach ($artifact in $newArtifacts) {
            $relative = Resolve-Path -LiteralPath $artifact.FullName -Relative
            Write-Host ("  {0} ({1:n0} bytes)" -f $relative, $artifact.Length)
        }
    } else {
        Write-Host "No new JSON artifacts were detected in expected output directories."
    }
}

exit $exitCode
