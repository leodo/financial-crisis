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

function Invoke-ReleaseReview {
    $args = @(
        "run", "-p", "fc-worker", "--",
        "research", "release", "review",
        "--market-scope", $MarketScope,
        "--baseline-release-id", $BaselineReleaseId,
        "--candidate-release-id", $CandidateReleaseId,
        "--history-mode", "default",
        "--history-limit", "5000"
    )

    & cargo @args
    if ($LASTEXITCODE -ne 0) {
        throw "release review failed for baseline=$BaselineReleaseId candidate=$CandidateReleaseId"
    }
}

function Resolve-ReleaseReviewPath {
    param(
        [string]$Baseline,
        [string]$Candidate,
        [string]$Mode = "default"
    )

    $reportDirectory = Join-Path $Root "artifacts/research/release-review"
    $pattern = "*$Baseline-vs-$Candidate-$Mode-release-review.json"
    $match = Get-ChildItem -LiteralPath $reportDirectory -Filter $pattern -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if ($match) {
        return $match.FullName
    }

    return $null
}

function Read-ReleaseReviewSummary {
    param([string]$Path)

    $extractor = @'
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as handle:
    doc = json.load(handle)

def runtime_summary(review):
    review = review or {}
    return {
        "regime_separation_summaries": review.get("regime_separation_summaries", []),
    }

summary = {
    "comparison": doc.get("comparison", {}),
    "baseline_runtime_review": runtime_summary(doc.get("baseline_runtime_review")),
    "candidate_runtime_review": runtime_summary(doc.get("candidate_runtime_review")),
    "overall_guard_passed": doc.get("overall_guard_passed"),
}
print(json.dumps(summary, ensure_ascii=False))
'@

    $json = $extractor | python - $Path
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to extract release-review summary from $Path"
    }

    $json | ConvertFrom-Json
}

function Load-ReleaseReviewJson {
    $path = Resolve-ReleaseReviewPath -Baseline $BaselineReleaseId -Candidate $CandidateReleaseId
    if (-not $path) {
        Write-Host "No default release-review artifact found; running release review first."
        Invoke-ReleaseReview
        $path = Resolve-ReleaseReviewPath -Baseline $BaselineReleaseId -Candidate $CandidateReleaseId
    }

    if (-not $path) {
        throw "Expected release-review artifact was not found after review run."
    }

    $doc = Read-ReleaseReviewSummary -Path $path

    [pscustomobject]@{
        path = $path
        doc = $doc
    }
}

function Format-DeltaPct {
    param([double]$Value)
    "{0:+0.0%;-0.0%;0.0%}" -f $Value
}

function Select-RegimeSummary {
    param(
        $RuntimeReview,
        [int]$HorizonDays
    )

    $RuntimeReview.regime_separation_summaries |
        Where-Object { [int]$_.horizon_days -eq $HorizonDays } |
        Select-Object -First 1
}

function Add-NoGoReason {
    param([string]$Reason)

    if ($script:recommendation -ne "no_go_offline") {
        $script:reasons.Clear()
    }
    $script:recommendation = "no_go_offline"
    $script:reasons.Add($Reason)
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

Write-Host "[1/6] Regional banks scenario window"
Invoke-FormalCompare -From "2022-12-01" -To "2023-03-15" -Scenario $ScenarioId
Write-Host ""

Write-Host "[2/6] February false-positive window"
Invoke-FormalCompare -From "2023-02-01" -To "2023-02-15" -Scenario ""
Write-Host ""

Write-Host "[3/6] July false-positive window"
Invoke-FormalCompare -From "2023-07-01" -To "2023-07-20" -Scenario ""
Write-Host ""

Write-Host "[4/6] Release-review cooldown / false-positive governance"
$releaseReviewEnvelope = Load-ReleaseReviewJson
$releaseReview = $releaseReviewEnvelope.doc
Write-Host ("  release review artifact: {0}" -f $releaseReviewEnvelope.path)
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
$reviewPrecision = $releaseReview.comparison.actionable_precision
$reviewLongestFalsePositive = $releaseReview.comparison.longest_false_positive_episode_days
$reviewRuntimeFloorHits = $releaseReview.comparison.runtime_floor_hit_count
$baselineRuntime20 = Select-RegimeSummary -RuntimeReview $releaseReview.baseline_runtime_review -HorizonDays 20
$candidateRuntime20 = Select-RegimeSummary -RuntimeReview $releaseReview.candidate_runtime_review -HorizonDays 20
$baselineRuntime60 = Select-RegimeSummary -RuntimeReview $releaseReview.baseline_runtime_review -HorizonDays 60
$candidateRuntime60 = Select-RegimeSummary -RuntimeReview $releaseReview.candidate_runtime_review -HorizonDays 60
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

$cooldownGovernanceRows = foreach ($item in @(
        @{ horizon = 20; baseline = $baselineRuntime20; candidate = $candidateRuntime20 },
        @{ horizon = 60; baseline = $baselineRuntime60; candidate = $candidateRuntime60 }
    )) {
    if (-not $item.baseline -or -not $item.candidate) {
        continue
    }

    $candidateCooldownMinusPositive = [double]$item.candidate.post_crisis_cooldown_avg_probability -
        [double]$item.candidate.positive_window_avg_probability
    $candidateCooldownMinusNormal = [double]$item.candidate.post_crisis_cooldown_avg_probability -
        [double]$item.candidate.normal_avg_probability

    [pscustomobject]@{
        horizon = "$($item.horizon)d"
        baseline_diagnosis = $item.baseline.diagnosis
        candidate_diagnosis = $item.candidate.diagnosis
        baseline_positive = [math]::Round([double]$item.baseline.positive_window_avg_probability, 6)
        candidate_positive = [math]::Round([double]$item.candidate.positive_window_avg_probability, 6)
        baseline_cooldown = [math]::Round([double]$item.baseline.post_crisis_cooldown_avg_probability, 6)
        candidate_cooldown = [math]::Round([double]$item.candidate.post_crisis_cooldown_avg_probability, 6)
        candidate_cooldown_minus_positive = [math]::Round($candidateCooldownMinusPositive, 6)
        candidate_cooldown_minus_normal = [math]::Round($candidateCooldownMinusNormal, 6)
    }
}

if ($reviewPrecision) {
    $candidatePrecision = [double]$reviewPrecision.candidate
    $precisionDelta = [double]$reviewPrecision.delta
    if ($candidatePrecision -lt 0.70 -or $precisionDelta -le -0.05) {
        Add-NoGoReason -Reason ("release review actionable precision is too weak: {0:P1} -> {1:P1}" -f [double]$reviewPrecision.baseline, $candidatePrecision)
    }
}

if ($reviewLongestFalsePositive) {
    $longestFalsePositiveDelta = [double]$reviewLongestFalsePositive.delta
    $candidateLongestFalsePositive = [double]$reviewLongestFalsePositive.candidate
    if ($longestFalsePositiveDelta -ge 7 -or $candidateLongestFalsePositive -gt 30) {
        Add-NoGoReason -Reason ("release review longest false-positive episode worsened: {0}d -> {1}d" -f [int]$reviewLongestFalsePositive.baseline, [int]$candidateLongestFalsePositive)
    }
}

if ($reviewRuntimeFloorHits -and [double]$reviewRuntimeFloorHits.delta -le -5) {
    Add-NoGoReason -Reason ("runtime floor hit count fell materially in release review: {0} -> {1}" -f [int]$reviewRuntimeFloorHits.baseline, [int]$reviewRuntimeFloorHits.candidate)
}

if ($candidateRuntime20) {
    $candidate20CooldownMinusPositive = [double]$candidateRuntime20.post_crisis_cooldown_avg_probability -
        [double]$candidateRuntime20.positive_window_avg_probability
    if ($candidateRuntime20.diagnosis -eq "cooldown_bleed") {
        Add-NoGoReason -Reason ("candidate shows 20d cooldown_bleed in release review runtime audit")
    }
    if ($candidate20CooldownMinusPositive -ge 0.0) {
        Add-NoGoReason -Reason ("candidate 20d cooldown avg is not below positive-window avg")
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
if ($reviewPrecision) {
    Write-Host ("  release-review precision         : {0:P1} -> {1:P1} ({2})" -f [double]$reviewPrecision.baseline, [double]$reviewPrecision.candidate, (Format-DeltaPct -Value ([double]$reviewPrecision.delta)))
}
if ($reviewLongestFalsePositive) {
    Write-Host ("  longest false-positive episode   : {0}d -> {1}d (delta {2})" -f [int]$reviewLongestFalsePositive.baseline, [int]$reviewLongestFalsePositive.candidate, [int]$reviewLongestFalsePositive.delta)
}
if ($reviewRuntimeFloorHits) {
    Write-Host ("  runtime floor hit count          : {0} -> {1} (delta {2})" -f [int]$reviewRuntimeFloorHits.baseline, [int]$reviewRuntimeFloorHits.candidate, [int]$reviewRuntimeFloorHits.delta)
}
Write-Host ("  recommendation                    : {0}" -f $recommendation)
foreach ($reason in $reasons) {
    Write-Host ("    - {0}" -f $reason)
}
Write-Host ""
Write-Host "Release-review cooldown governance"
$cooldownGovernanceRows | Format-Table -AutoSize
Write-Host ""
Write-Host "20d regime summary"
$regimeRows | Format-Table -AutoSize
Write-Host ""
Write-Host "Tracked 20d weight deltas"
$trackedRows | Format-Table -AutoSize
Write-Host ""
Write-Host "[5/6] Curve / USDJPY / threshold semantics audit"
& (Join-Path $PSScriptRoot "formal-candidate-semantics-audit.ps1") `
    -BaselineReleaseId $BaselineReleaseId `
    -CandidateReleaseId $CandidateReleaseId `
    -HorizonDays 20
if ($LASTEXITCODE -ne 0) {
    throw "formal-candidate-semantics-audit failed"
}
Write-Host ""
Write-Host "[6/6] US history scenario-pack audit"
& (Join-Path $PSScriptRoot "formal-candidate-scenario-pack-audit.ps1") `
    -BaselineReleaseId $BaselineReleaseId `
    -CandidateReleaseId $CandidateReleaseId
if ($LASTEXITCODE -ne 0) {
    throw "formal-candidate-scenario-pack-audit failed"
}
