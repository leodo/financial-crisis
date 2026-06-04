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

function Load-CompareJson {
    param(
        [string]$Baseline,
        [string]$Candidate,
        [string]$From,
        [string]$To,
        [string]$Scenario
    )

    $slug = if ($Scenario) {
        "$Baseline-vs-$Candidate-$From-$To-formal-probability-compare-$Scenario.json"
    } else {
        "$Baseline-vs-$Candidate-$From-$To-formal-probability-compare.json"
    }
    $path = Join-Path $Root "artifacts/research/formal-probability-compares/$slug"
    if (-not (Test-Path -LiteralPath $path)) {
        throw "Expected compare artifact was not found: $path"
    }
    Get-Content -LiteralPath $path | ConvertFrom-Json
}

function Format-DeltaPct {
    param([double]$Value)
    "{0:+0.0%;-0.0%;0.0%}" -f $Value
}

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

$TrackedFeatures = @(
    "tail_neg__us_curve_10y2y_level__0",
    "tail_pos__us_baa_10y_spread_level__2",
    "us_usdjpy_level",
    "interaction__external_dimension_score__us_usdjpy_level",
    "family_context__jpy_carry__external_dimension_score",
    "family_context__rate_shock__external_dimension_score",
    "us_fed_funds_level"
)

Write-Host "Offline candidate screen"
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
Write-Host ""

$regional = Load-CompareJson -Baseline $BaselineReleaseId -Candidate $CandidateReleaseId -From "2022-12-01" -To "2023-03-15" -Scenario $ScenarioId
$february = Load-CompareJson -Baseline $BaselineReleaseId -Candidate $CandidateReleaseId -From "2023-02-01" -To "2023-02-15" -Scenario ""
$july = Load-CompareJson -Baseline $BaselineReleaseId -Candidate $CandidateReleaseId -From "2023-07-01" -To "2023-07-20" -Scenario ""

$baselineH20 = Load-HorizonRecord -ReleaseId $BaselineReleaseId -TargetHorizonDays 20
$candidateH20 = Load-HorizonRecord -ReleaseId $CandidateReleaseId -TargetHorizonDays 20
$baselineMap = Build-CoefficientMap -Coefficients $baselineH20.record.raw_model.coefficients
$candidateMap = Build-CoefficientMap -Coefficients $candidateH20.record.raw_model.coefficients

$regionalSummary = $regional.summary
$regionalPositive = $regionalSummary.positive_window_20d
$febSummary = $february.summary.overall_window
$julySummary = $july.summary.overall_window
$baselinePositiveAvgProbability = [double]$baselineH20.record.evaluation.regime_separation.positive_window_avg_probability
$candidatePositiveAvgProbability = [double]$candidateH20.record.evaluation.regime_separation.positive_window_avg_probability
$positiveAvgProbabilityRetention = if ($baselinePositiveAvgProbability -gt 0.0) {
    $candidatePositiveAvgProbability / $baselinePositiveAvgProbability
} else {
    1.0
}
$positiveAvgProbabilityDelta = $candidatePositiveAvgProbability - $baselinePositiveAvgProbability
$curveTailDelta = (Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_neg__us_curve_10y2y_level__0") - (Get-CoefficientWeight -Map $baselineMap -FeatureName "tail_neg__us_curve_10y2y_level__0")
$usdjpyLevelDelta = (Get-CoefficientWeight -Map $candidateMap -FeatureName "us_usdjpy_level") - (Get-CoefficientWeight -Map $baselineMap -FeatureName "us_usdjpy_level")
$usdjpyInteractionDelta = (Get-CoefficientWeight -Map $candidateMap -FeatureName "interaction__external_dimension_score__us_usdjpy_level") - (Get-CoefficientWeight -Map $baselineMap -FeatureName "interaction__external_dimension_score__us_usdjpy_level")

$retainedPositiveHitRate = if ($regionalPositive.baseline_hit_rate_20d -gt 0) {
    [double]$regionalPositive.candidate_hit_rate_20d / [double]$regionalPositive.baseline_hit_rate_20d
} else {
    1.0
}
$regionalHitLoss = [int]$regionalSummary.baseline_hit_count_20d - [int]$regionalSummary.candidate_hit_count_20d
$falsePositiveReductionFeb = [int]$february.summary.baseline_hit_count_20d - [int]$february.summary.candidate_hit_count_20d
$falsePositiveReductionJuly = [int]$july.summary.baseline_hit_count_20d - [int]$july.summary.candidate_hit_count_20d
$retainedPositiveHitRateForDecision = $retainedPositiveHitRate + 1e-9

$recommendation = "manual_review"
$reasons = New-Object System.Collections.Generic.List[string]

if ($regionalPositive.candidate_hit_rate_20d -lt 0.5 -or $retainedPositiveHitRate -lt 0.70 -or $regionalHitLoss -ge 12) {
    $recommendation = "no_go_offline"
    $reasons.Add("regional_banks positive-window continuity fell too far before runtime review")
}

if (
    $recommendation -ne "no_go_offline" -and
    ($positiveAvgProbabilityRetention -lt 0.75 -or $positiveAvgProbabilityDelta -le -0.06)
) {
    $recommendation = "no_go_offline"
    $reasons.Add("candidate crushed 20d raw positive-window probability before threshold policy could help")
}

if (
    $recommendation -ne "no_go_offline" -and
    $curveTailDelta -le -0.08 -and
    $usdjpyLevelDelta -le -0.12 -and
    $usdjpyInteractionDelta -ge 0.07
) {
    $recommendation = "no_go_offline"
    $reasons.Add("candidate deepened curve-tail suppression while simultaneously shifting USDJPY into a harsher interaction mix")
}

if ($recommendation -ne "no_go_offline") {
    if ($falsePositiveReductionFeb -le 0 -and $falsePositiveReductionJuly -le 0) {
        $recommendation = "no_go_offline"
        $reasons.Add("candidate did not materially reduce either false-positive window")
    } elseif (
        $retainedPositiveHitRateForDecision -ge 0.80 -and
        $positiveAvgProbabilityRetention -ge 0.80 -and
        $falsePositiveReductionFeb -ge 2 -and
        $falsePositiveReductionJuly -ge 4
    ) {
        $recommendation = "worth_fast_review"
        $reasons.Add("candidate keeps most positive-window continuity while materially shrinking both false-positive windows")
    } else {
        $recommendation = "manual_review"
        $reasons.Add("candidate shows mixed trade-offs that need human inspection before runtime review")
    }
}

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

$regimeRows = @(
    [pscustomobject]@{ metric = "threshold20"; baseline = [math]::Round($baselineH20.record.decision_threshold, 6); candidate = [math]::Round($candidateH20.record.decision_threshold, 6); delta = [math]::Round($candidateH20.record.decision_threshold - $baselineH20.record.decision_threshold, 6) }
    [pscustomobject]@{ metric = "normal_avg_p20d"; baseline = [math]::Round($baselineH20.record.evaluation.regime_separation.normal_avg_probability, 6); candidate = [math]::Round($candidateH20.record.evaluation.regime_separation.normal_avg_probability, 6); delta = [math]::Round($candidateH20.record.evaluation.regime_separation.normal_avg_probability - $baselineH20.record.evaluation.regime_separation.normal_avg_probability, 6) }
    [pscustomobject]@{ metric = "buffer_avg_p20d"; baseline = [math]::Round($baselineH20.record.evaluation.regime_separation.pre_warning_buffer_avg_probability, 6); candidate = [math]::Round($candidateH20.record.evaluation.regime_separation.pre_warning_buffer_avg_probability, 6); delta = [math]::Round($candidateH20.record.evaluation.regime_separation.pre_warning_buffer_avg_probability - $baselineH20.record.evaluation.regime_separation.pre_warning_buffer_avg_probability, 6) }
    [pscustomobject]@{ metric = "positive_avg_p20d"; baseline = [math]::Round($baselineH20.record.evaluation.regime_separation.positive_window_avg_probability, 6); candidate = [math]::Round($candidateH20.record.evaluation.regime_separation.positive_window_avg_probability, 6); delta = [math]::Round($candidateH20.record.evaluation.regime_separation.positive_window_avg_probability - $baselineH20.record.evaluation.regime_separation.positive_window_avg_probability, 6) }
)

Write-Host "Offline screen summary"
Write-Host ("  regional positive-window hit rate : {0:P1} -> {1:P1} (retained {2:P1})" -f $regionalPositive.baseline_hit_rate_20d, $regionalPositive.candidate_hit_rate_20d, $retainedPositiveHitRate)
Write-Host ("  regional 20d hits                 : {0} -> {1} (delta {2})" -f $regionalSummary.baseline_hit_count_20d, $regionalSummary.candidate_hit_count_20d, ($regionalHitLoss * -1))
Write-Host ("  positive-window avg p20d          : {0:0.000} -> {1:0.000} (retained {2:P1})" -f $baselinePositiveAvgProbability, $candidatePositiveAvgProbability, $positiveAvgProbabilityRetention)
Write-Host ("  feb false-positive hits           : {0} -> {1}" -f $february.summary.baseline_hit_count_20d, $february.summary.candidate_hit_count_20d)
Write-Host ("  july false-positive hits          : {0} -> {1}" -f $july.summary.baseline_hit_count_20d, $july.summary.candidate_hit_count_20d)
Write-Host ("  feb avg delta p20d               : {0}" -f (Format-DeltaPct -Value ([double]$febSummary.avg_delta_p_20d)))
Write-Host ("  july avg delta p20d              : {0}" -f (Format-DeltaPct -Value ([double]$julySummary.avg_delta_p_20d)))
Write-Host ("  recommendation                    : {0}" -f $recommendation)
foreach ($reason in $reasons) {
    Write-Host ("    - {0}" -f $reason)
}
Write-Host ""
Write-Host "20d regime summary"
$regimeRows | Format-Table -AutoSize
Write-Host ""
Write-Host "Tracked 20d weight deltas"
$trackedRows | Format-Table -AutoSize
