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
    "family_proxy__systemic_credit",
    "family_context__systemic_credit__structural_score",
    "family_context__systemic_credit__trigger_score",
    "family_context__systemic_credit__external_dimension_score",
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

function Resolve-ReleaseReviewPath {
    $reportDirectory = Join-Path $Root "artifacts/research/release-review"
    $pattern = "*$BaselineReleaseId-vs-$CandidateReleaseId-default-release-review.json"
    $match = Get-ChildItem -LiteralPath $reportDirectory -Filter $pattern -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1

    if ($match) {
        return $match.FullName
    }

    return $null
}

function Read-ReleaseReviewRuntimeSummary {
    param([string]$Path)

    $extractor = @'
import json
import sys

path, horizon = sys.argv[1], int(sys.argv[2])
with open(path, "r", encoding="utf-8") as handle:
    doc = json.load(handle)

def horizon_row(rows):
    for row in rows or []:
        if int(row.get("horizon_days", -1)) == horizon:
            return row
    return None

def runtime_rows(review):
    return (review or {}).get("regime_separation_summaries") or []

comparison = doc.get("comparison") or {}
summary = {
    "path": path,
    "baseline": horizon_row(runtime_rows(doc.get("baseline_runtime_review"))),
    "candidate": horizon_row(runtime_rows(doc.get("candidate_runtime_review"))),
    "comparison": horizon_row(comparison.get("runtime_separation_summary") or []),
}
print(json.dumps(summary, ensure_ascii=False))
'@

    $json = $extractor | python - $Path $HorizonDays
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to extract runtime regime summary from $Path"
    }

    $json | ConvertFrom-Json
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

function Get-NumberOrNull {
    param($Value)

    if ($null -eq $Value) {
        return $null
    }

    return [double]$Value
}

function New-RegimeMetricRow {
    param(
        [string]$Metric,
        [double]$BaselineValue,
        [double]$CandidateValue
    )

    [pscustomobject]@{
        metric = $Metric
        baseline = [math]::Round($BaselineValue, 6)
        candidate = [math]::Round($CandidateValue, 6)
        delta = [math]::Round($CandidateValue - $BaselineValue, 6)
    }
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
$baselineThreshold = [double]$baseline.record.decision_threshold
$candidateThreshold = [double]$candidate.record.decision_threshold
$baselinePositive = Get-NumberOrNull $baselineRegime.positive_window_avg_probability
$candidatePositive = Get-NumberOrNull $candidateRegime.positive_window_avg_probability
$baselinePreWarning = Get-NumberOrNull $baselineRegime.pre_warning_buffer_avg_probability
$candidatePreWarning = Get-NumberOrNull $candidateRegime.pre_warning_buffer_avg_probability
$baselineCooldown = Get-NumberOrNull $baselineRegime.post_crisis_cooldown_avg_probability
$candidateCooldown = Get-NumberOrNull $candidateRegime.post_crisis_cooldown_avg_probability
$baselineNormal = Get-NumberOrNull $baselineRegime.normal_avg_probability
$candidateNormal = Get-NumberOrNull $candidateRegime.normal_avg_probability
$regimeRows = @(
    New-RegimeMetricRow -Metric "decision_threshold" -BaselineValue $baselineThreshold -CandidateValue $candidateThreshold
    New-RegimeMetricRow -Metric "normal_avg_probability" -BaselineValue $baselineNormal -CandidateValue $candidateNormal
    New-RegimeMetricRow -Metric "pre_warning_buffer_avg_probability" -BaselineValue $baselinePreWarning -CandidateValue $candidatePreWarning
    New-RegimeMetricRow -Metric "positive_window_avg_probability" -BaselineValue $baselinePositive -CandidateValue $candidatePositive
    New-RegimeMetricRow -Metric "post_crisis_cooldown_avg_probability" -BaselineValue $baselineCooldown -CandidateValue $candidateCooldown
    New-RegimeMetricRow -Metric "positive_minus_threshold_gap" -BaselineValue ($baselinePositive - $baselineThreshold) -CandidateValue ($candidatePositive - $candidateThreshold)
    New-RegimeMetricRow -Metric "positive_minus_cooldown_gap" -BaselineValue ($baselinePositive - $baselineCooldown) -CandidateValue ($candidatePositive - $candidateCooldown)
    New-RegimeMetricRow -Metric "positive_minus_normal_gap" -BaselineValue ($baselinePositive - $baselineNormal) -CandidateValue ($candidatePositive - $candidateNormal)
    New-RegimeMetricRow -Metric "prewarning_minus_cooldown_gap" -BaselineValue ($baselinePreWarning - $baselineCooldown) -CandidateValue ($candidatePreWarning - $candidateCooldown)
)

$takeaways = New-Object System.Collections.Generic.List[string]
if (($candidatePositive - $candidateThreshold) -lt 0.0) {
    $takeaways.Add("candidate positive-window average remains below its decision threshold; runtime floor hits can stay sparse even when raw probability improves.")
}
if (($candidatePositive - $candidateCooldown) -le 0.0) {
    $takeaways.Add("candidate positive-window average is not above cooldown; this is the core cooldown-bleed failure mode.")
}
if (($candidatePositive - $candidateNormal) -le 0.0) {
    $takeaways.Add("candidate positive-window average is not above normal; the head is not separating true risk windows from background.")
}
if (($candidatePreWarning - $candidateCooldown) -le 0.0 -and $HorizonDays -in @(20, 60)) {
    $takeaways.Add("candidate pre-warning buffer is not above cooldown; early-warning shape is still contaminated by post-crisis background.")
}
if ($candidateRegime.diagnosis -eq "cooldown_bleed" -or $candidateRegime.diagnosis -eq "cold_across_all_regimes") {
    $takeaways.Add("candidate regime diagnosis is $($candidateRegime.diagnosis); treat this as a training/feature-separation problem, not a runtime threshold tweak.")
}

$runtimeSummary = $null
$runtimeRows = @()
$reviewPath = Resolve-ReleaseReviewPath
if ($reviewPath) {
    $runtimeSummary = Read-ReleaseReviewRuntimeSummary -Path $reviewPath
    if ($runtimeSummary.baseline -and $runtimeSummary.candidate) {
        $runtimeBaselinePositive = Get-NumberOrNull $runtimeSummary.baseline.positive_window_avg_probability
        $runtimeCandidatePositive = Get-NumberOrNull $runtimeSummary.candidate.positive_window_avg_probability
        $runtimeBaselineCooldown = Get-NumberOrNull $runtimeSummary.baseline.post_crisis_cooldown_avg_probability
        $runtimeCandidateCooldown = Get-NumberOrNull $runtimeSummary.candidate.post_crisis_cooldown_avg_probability
        $runtimeBaselineNormal = Get-NumberOrNull $runtimeSummary.baseline.normal_avg_probability
        $runtimeCandidateNormal = Get-NumberOrNull $runtimeSummary.candidate.normal_avg_probability
        $runtimeBaselinePreWarning = Get-NumberOrNull $runtimeSummary.baseline.pre_warning_buffer_avg_probability
        $runtimeCandidatePreWarning = Get-NumberOrNull $runtimeSummary.candidate.pre_warning_buffer_avg_probability

        $runtimeRows = @(
            New-RegimeMetricRow -Metric "runtime_normal_avg_probability" -BaselineValue $runtimeBaselineNormal -CandidateValue $runtimeCandidateNormal
            New-RegimeMetricRow -Metric "runtime_pre_warning_buffer_avg_probability" -BaselineValue $runtimeBaselinePreWarning -CandidateValue $runtimeCandidatePreWarning
            New-RegimeMetricRow -Metric "runtime_positive_window_avg_probability" -BaselineValue $runtimeBaselinePositive -CandidateValue $runtimeCandidatePositive
            New-RegimeMetricRow -Metric "runtime_post_crisis_cooldown_avg_probability" -BaselineValue $runtimeBaselineCooldown -CandidateValue $runtimeCandidateCooldown
            New-RegimeMetricRow -Metric "runtime_positive_minus_cooldown_gap" -BaselineValue ($runtimeBaselinePositive - $runtimeBaselineCooldown) -CandidateValue ($runtimeCandidatePositive - $runtimeCandidateCooldown)
            New-RegimeMetricRow -Metric "runtime_positive_minus_normal_gap" -BaselineValue ($runtimeBaselinePositive - $runtimeBaselineNormal) -CandidateValue ($runtimeCandidatePositive - $runtimeCandidateNormal)
            New-RegimeMetricRow -Metric "bundle_to_runtime_positive_cooldown_gap_drift" -BaselineValue (($baselinePositive - $baselineCooldown) - ($runtimeBaselinePositive - $runtimeBaselineCooldown)) -CandidateValue (($candidatePositive - $candidateCooldown) - ($runtimeCandidatePositive - $runtimeCandidateCooldown))
        )

        if (($runtimeCandidatePositive - $runtimeCandidateCooldown) -le 0.0) {
            $takeaways.Add("candidate runtime positive-window average is not above cooldown; release review still sees cooldown bleed even if bundle evaluation looks separated.")
        }
        if (($candidatePositive - $candidateCooldown) -gt 0.0 -and ($runtimeCandidatePositive - $runtimeCandidateCooldown) -le 0.0) {
            $takeaways.Add("candidate separates positive-window from cooldown in bundle evaluation but loses that separation in runtime replay; prioritize train/runtime distribution drift and feature transfer.")
        }
    }
}

Write-Host "Formal candidate feature audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  horizon  : ${HorizonDays}d"
Write-Host ""

Write-Host "Regime separation summary"
$regimeRows | Format-Table -AutoSize
Write-Host ""

if ($runtimeSummary -and $runtimeRows.Count -gt 0) {
    Write-Host "Runtime regime separation summary"
    Write-Host "  review: $($runtimeSummary.path)"
    Write-Host "  baseline diagnosis: $($runtimeSummary.baseline.diagnosis)"
    Write-Host "  candidate diagnosis: $($runtimeSummary.candidate.diagnosis)"
    $runtimeRows | Format-Table -AutoSize
    Write-Host ""
} elseif ($reviewPath) {
    Write-Host "Runtime regime separation summary"
    Write-Host "  review artifact found but horizon ${HorizonDays}d was missing."
    Write-Host ""
} else {
    Write-Host "Runtime regime separation summary"
    Write-Host "  no default release-review artifact found for this candidate; run formal-candidate-screen or release review to attach runtime evidence."
    Write-Host ""
}

Write-Host "Regime gap takeaways"
if ($takeaways.Count -eq 0) {
    Write-Host "  - candidate regime gaps do not show an obvious threshold/cooldown blocker in this horizon."
} else {
    foreach ($takeaway in $takeaways) {
        Write-Host "  - $takeaway"
    }
}
Write-Host ""

Write-Host "Tracked feature weights"
$trackedRows | Format-Table -AutoSize
Write-Host ""

Write-Host "Top absolute coefficient deltas"
$topRows | Format-Table -AutoSize
