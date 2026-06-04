param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [int]$HorizonDays = 20,
    [int]$TopCount = 12
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

$TrackedFeatures = @(
    "tail_neg__us_curve_10y2y_level__0",
    "tail_pos__us_baa_10y_spread_level__2",
    "us_usdjpy_level",
    "interaction__external_dimension_score__us_usdjpy_level",
    "family_context__jpy_carry__external_dimension_score",
    "family_context__rate_shock__external_dimension_score",
    "us_fed_funds_level",
    "interaction__structural_score__trigger_score"
)

function Resolve-EvaluationPath {
    param([string]$ReleaseId)

    $candidates = @(
        "artifacts/research/model-bundles/generated/$ReleaseId-evaluation.json",
        "config/model-bundles/generated/$ReleaseId-evaluation.json"
    )

    foreach ($relative in $candidates) {
        $path = Join-Path $Root $relative
        if (Test-Path -LiteralPath $path) {
            return (Resolve-Path -LiteralPath $path).Path
        }
    }

    throw "Evaluation artifact for release $ReleaseId was not found in generated bundle directories."
}

function Load-HorizonRecord {
    param(
        [string]$ReleaseId,
        [int]$TargetHorizonDays
    )

    $path = Resolve-EvaluationPath -ReleaseId $ReleaseId
    $doc = Get-Content -LiteralPath $path | ConvertFrom-Json
    $record = $doc.horizons | Where-Object { $_.horizon_days -eq $TargetHorizonDays } | Select-Object -First 1
    if (-not $record) {
        throw "Release $ReleaseId does not contain horizon $TargetHorizonDays."
    }

    [pscustomobject]@{
        release_id = $ReleaseId
        path = $path
        record = $record
    }
}

function Build-CoefficientMap {
    param($Coefficients)

    $map = @{}
    foreach ($coefficient in $Coefficients) {
        $map[$coefficient.name] = [double]$coefficient.weight
    }
    $map
}

function Get-CoefficientWeight {
    param(
        [hashtable]$Map,
        [string]$FeatureName
    )

    if ($Map.ContainsKey($FeatureName)) {
        return [double]$Map[$FeatureName]
    }

    return 0.0
}

$baseline = Load-HorizonRecord -ReleaseId $BaselineReleaseId -TargetHorizonDays $HorizonDays
$candidate = Load-HorizonRecord -ReleaseId $CandidateReleaseId -TargetHorizonDays $HorizonDays

$baselineMap = Build-CoefficientMap -Coefficients $baseline.record.raw_model.coefficients
$candidateMap = Build-CoefficientMap -Coefficients $candidate.record.raw_model.coefficients

$allFeatureNames = @($baselineMap.Keys + $candidateMap.Keys | Sort-Object -Unique)
$coefficientRows = foreach ($featureName in $allFeatureNames) {
    $baselineWeight = Get-CoefficientWeight -Map $baselineMap -FeatureName $featureName
    $candidateWeight = Get-CoefficientWeight -Map $candidateMap -FeatureName $featureName
    [pscustomobject]@{
        feature = $featureName
        baseline_weight = [math]::Round($baselineWeight, 6)
        candidate_weight = [math]::Round($candidateWeight, 6)
        delta_weight = [math]::Round($candidateWeight - $baselineWeight, 6)
    }
}
$topRows = $coefficientRows |
    Sort-Object { [math]::Abs($_.delta_weight) } -Descending |
    Select-Object -First $TopCount

$trackedRows = foreach ($featureName in $TrackedFeatures) {
    $baselineWeight = Get-CoefficientWeight -Map $baselineMap -FeatureName $featureName
    $candidateWeight = Get-CoefficientWeight -Map $candidateMap -FeatureName $featureName
    [pscustomobject]@{
        feature = $featureName
        baseline_weight = [math]::Round($baselineWeight, 6)
        candidate_weight = [math]::Round($candidateWeight, 6)
        delta_weight = [math]::Round($candidateWeight - $baselineWeight, 6)
    }
}

$baselineRegime = $baseline.record.evaluation.regime_separation
$candidateRegime = $candidate.record.evaluation.regime_separation
$regimeRows = @(
    [pscustomobject]@{ metric = "decision_threshold"; baseline = [math]::Round($baseline.record.decision_threshold, 6); candidate = [math]::Round($candidate.record.decision_threshold, 6); delta = [math]::Round($candidate.record.decision_threshold - $baseline.record.decision_threshold, 6) }
    [pscustomobject]@{ metric = "normal_avg_probability"; baseline = [math]::Round($baselineRegime.normal_avg_probability, 6); candidate = [math]::Round($candidateRegime.normal_avg_probability, 6); delta = [math]::Round($candidateRegime.normal_avg_probability - $baselineRegime.normal_avg_probability, 6) }
    [pscustomobject]@{ metric = "pre_warning_buffer_avg_probability"; baseline = [math]::Round($baselineRegime.pre_warning_buffer_avg_probability, 6); candidate = [math]::Round($candidateRegime.pre_warning_buffer_avg_probability, 6); delta = [math]::Round($candidateRegime.pre_warning_buffer_avg_probability - $baselineRegime.pre_warning_buffer_avg_probability, 6) }
    [pscustomobject]@{ metric = "positive_window_avg_probability"; baseline = [math]::Round($baselineRegime.positive_window_avg_probability, 6); candidate = [math]::Round($candidateRegime.positive_window_avg_probability, 6); delta = [math]::Round($candidateRegime.positive_window_avg_probability - $baselineRegime.positive_window_avg_probability, 6) }
    [pscustomobject]@{ metric = "post_crisis_cooldown_avg_probability"; baseline = [math]::Round($baselineRegime.post_crisis_cooldown_avg_probability, 6); candidate = [math]::Round($candidateRegime.post_crisis_cooldown_avg_probability, 6); delta = [math]::Round($candidateRegime.post_crisis_cooldown_avg_probability - $baselineRegime.post_crisis_cooldown_avg_probability, 6) }
)

Write-Host "Formal candidate feature audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  horizon  : ${HorizonDays}d"
Write-Host ""

Write-Host "Regime separation summary"
$regimeRows | Format-Table -AutoSize
Write-Host ""

Write-Host "Tracked feature weights"
$trackedRows | Format-Table -AutoSize
Write-Host ""

Write-Host "Top absolute coefficient deltas"
$topRows | Format-Table -AutoSize
