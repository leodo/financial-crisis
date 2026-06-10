param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [int]$HorizonDays = 20,
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

$CurveFeatureNames = @(
    "us_curve_10y2y_level",
    "interaction__us_curve_10y2y_level__us_fed_funds_level",
    "tail_neg__us_curve_10y2y_level__0",
    "us_baa_10y_spread_level",
    "tail_pos__us_baa_10y_spread_level__2",
    "interaction__us_baa_10y_spread_level__us_vix_level"
)

$UsdJpyFeatureNames = @(
    "us_usdjpy_level",
    "us_usdjpy_change_20d",
    "interaction__external_dimension_score__us_usdjpy_level",
    "interaction__trigger_score__us_usdjpy_change_20d",
    "tail_pos__us_usdjpy_level__145",
    "tail_abs_pos__us_usdjpy_change_20d__4"
)

$FamilyFeatureNames = @(
    "family_proxy__rate_shock",
    "family_context__rate_shock__external_dimension_score",
    "family_proxy__jpy_carry",
    "family_context__jpy_carry__external_dimension_score"
)

$BroadScoreFeatureNames = @(
    "trigger_score",
    "external_dimension_score",
    "tail_pos__trigger_score__50",
    "tail_pos__external_dimension_score__50"
)

function Resolve-ArtifactPath {
    param(
        [string]$ReleaseId,
        [string]$Suffix
    )

    $candidates = @(
        "artifacts/research/model-bundles/generated/$ReleaseId$Suffix",
        "config/model-bundles/generated/$ReleaseId$Suffix"
    )

    foreach ($relative in $candidates) {
        $path = Join-Path $Root $relative
        if (Test-Path -LiteralPath $path) {
            return (Resolve-Path -LiteralPath $path).Path
        }
    }

    throw "Artifact for release $ReleaseId with suffix $Suffix was not found in generated bundle directories."
}

function Load-HorizonRecord {
    param(
        [string]$ReleaseId,
        [string]$Suffix,
        [int]$TargetHorizonDays
    )

    $path = Resolve-ArtifactPath -ReleaseId $ReleaseId -Suffix $Suffix
    $doc = Get-Content -LiteralPath $path | ConvertFrom-Json
    $record = $doc.horizons | Where-Object { $_.horizon_days -eq $TargetHorizonDays } | Select-Object -First 1
    if (-not $record) {
        throw "Release $ReleaseId does not contain horizon $TargetHorizonDays in $Suffix."
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

function New-WeightRow {
    param(
        [hashtable]$BaselineMap,
        [hashtable]$CandidateMap,
        [string]$FeatureName
    )

    $baselineWeight = Get-CoefficientWeight -Map $BaselineMap -FeatureName $FeatureName
    $candidateWeight = Get-CoefficientWeight -Map $CandidateMap -FeatureName $FeatureName
    [pscustomobject]@{
        feature = $FeatureName
        baseline_weight = [math]::Round($baselineWeight, 6)
        candidate_weight = [math]::Round($candidateWeight, 6)
        delta_weight = [math]::Round($candidateWeight - $baselineWeight, 6)
    }
}

function New-ThresholdSummaryRows {
    param(
        $BaselineEval,
        $CandidateEval
    )

    $baselineRegime = $BaselineEval.record.evaluation.regime_separation
    $candidateRegime = $CandidateEval.record.evaluation.regime_separation
    $baselineDiag = $BaselineEval.record.threshold_diagnostics
    $candidateDiag = $CandidateEval.record.threshold_diagnostics

    @(
        [pscustomobject]@{
            metric = "decision_threshold"
            baseline = [math]::Round([double]$BaselineEval.record.decision_threshold, 6)
            candidate = [math]::Round([double]$CandidateEval.record.decision_threshold, 6)
            delta = [math]::Round(([double]$CandidateEval.record.decision_threshold - [double]$BaselineEval.record.decision_threshold), 6)
        }
        [pscustomobject]@{
            metric = "threshold_base"
            baseline = [math]::Round([double]$baselineDiag.base_threshold, 6)
            candidate = [math]::Round([double]$candidateDiag.base_threshold, 6)
            delta = [math]::Round(([double]$candidateDiag.base_threshold - [double]$baselineDiag.base_threshold), 6)
        }
        [pscustomobject]@{
            metric = "threshold_final"
            baseline = [math]::Round([double]$baselineDiag.final_threshold, 6)
            candidate = [math]::Round([double]$candidateDiag.final_threshold, 6)
            delta = [math]::Round(([double]$candidateDiag.final_threshold - [double]$baselineDiag.final_threshold), 6)
        }
        [pscustomobject]@{
            metric = "positive_window_avg_probability"
            baseline = [math]::Round([double]$baselineRegime.positive_window_avg_probability, 6)
            candidate = [math]::Round([double]$candidateRegime.positive_window_avg_probability, 6)
            delta = [math]::Round(([double]$candidateRegime.positive_window_avg_probability - [double]$baselineRegime.positive_window_avg_probability), 6)
        }
        [pscustomobject]@{
            metric = "normal_avg_probability"
            baseline = [math]::Round([double]$baselineRegime.normal_avg_probability, 6)
            candidate = [math]::Round([double]$candidateRegime.normal_avg_probability, 6)
            delta = [math]::Round(([double]$candidateRegime.normal_avg_probability - [double]$baselineRegime.normal_avg_probability), 6)
        }
        [pscustomobject]@{
            metric = "positive_minus_threshold_gap"
            baseline = [math]::Round(([double]$baselineRegime.positive_window_avg_probability - [double]$baselineDiag.final_threshold), 6)
            candidate = [math]::Round(([double]$candidateRegime.positive_window_avg_probability - [double]$candidateDiag.final_threshold), 6)
            delta = [math]::Round((([double]$candidateRegime.positive_window_avg_probability - [double]$candidateDiag.final_threshold) - ([double]$baselineRegime.positive_window_avg_probability - [double]$baselineDiag.final_threshold)), 6)
        }
        [pscustomobject]@{
            metric = "positive_minus_normal_gap"
            baseline = [math]::Round(([double]$baselineRegime.positive_window_avg_probability - [double]$baselineRegime.normal_avg_probability), 6)
            candidate = [math]::Round(([double]$candidateRegime.positive_window_avg_probability - [double]$candidateRegime.normal_avg_probability), 6)
            delta = [math]::Round((([double]$candidateRegime.positive_window_avg_probability - [double]$candidateRegime.normal_avg_probability) - ([double]$baselineRegime.positive_window_avg_probability - [double]$baselineRegime.normal_avg_probability)), 6)
        }
    )
}

function Get-ThresholdRoleTakeaway {
    param(
        $BaselineEval,
        $CandidateEval
    )

    $baselineRegime = $BaselineEval.record.evaluation.regime_separation
    $candidateRegime = $CandidateEval.record.evaluation.regime_separation
    $baselineDiag = $BaselineEval.record.threshold_diagnostics
    $candidateDiag = $CandidateEval.record.threshold_diagnostics

    $baselinePositive = [double]$baselineRegime.positive_window_avg_probability
    $candidatePositive = [double]$candidateRegime.positive_window_avg_probability
    $baselineFinalThreshold = [double]$baselineDiag.final_threshold
    $candidateFinalThreshold = [double]$candidateDiag.final_threshold
    $candidateGap = $candidatePositive - $candidateFinalThreshold
    $baselineGap = $baselinePositive - $baselineFinalThreshold

    if ($candidateGap -ge 0.0) {
        return "candidate positive-window average already clears the 20d final threshold; threshold policy is not the primary blocker on average."
    }

    if ($candidatePositive -lt ($baselinePositive - 0.01)) {
        return "candidate positive-window raw probability fell before threshold policy could help; fix feature semantics or raw head strength first."
    }

    if (
        $candidatePositive -ge ($baselinePositive - 0.01) -and
        $candidateFinalThreshold -gt ($baselineFinalThreshold + 0.01)
    ) {
        return "candidate preserved raw positive-window probability reasonably well, but the 20d final threshold still sits above it; threshold policy remains a binding factor."
    }

    if ($candidateGap -lt $baselineGap) {
        return "candidate still sits further below the 20d final threshold than baseline; raw separation and threshold policy both need attention."
    }

    return "candidate improved part of the 20d picture, but positive-window average still does not clear the final threshold; keep threshold as a soft policy check rather than the main repair lever."
}

function New-GuardrailStatusRow {
    param(
        [string]$Item,
        [string]$Coverage,
        [string]$EntryPoint,
        [string]$Rule,
        [double]$BaselineValue,
        [double]$CandidateValue,
        [nullable[double]]$MinAllowed = $null,
        [nullable[double]]$MaxAllowed = $null
    )

    $status = "doc_only"
    $notes = "not automatically enforced in training"

    if ($Coverage -eq "training_guardrail") {
        $hasMin = $null -ne $MinAllowed
        $hasMax = $null -ne $MaxAllowed
        $withinMin = -not $hasMin -or $CandidateValue -ge ([double]$MinAllowed)
        $withinMax = -not $hasMax -or $CandidateValue -le ([double]$MaxAllowed)
        if ($withinMin -and $withinMax) {
            $status = "ok"
            $notes = "candidate stays within the current training guardrail"
        } else {
            $status = "violated"
            $notes = "candidate would violate the current training guardrail"
        }
    } elseif ($Coverage -eq "partial_policy") {
        $status = "partial"
        $notes = "partly enforced through threshold selection/repair logic, but still needs human interpretation"
    }

    [pscustomobject]@{
        item = $Item
        coverage = $Coverage
        entry_point = $EntryPoint
        rule = $Rule
        baseline = [math]::Round($BaselineValue, 6)
        candidate = [math]::Round($CandidateValue, 6)
        status = $status
        notes = $notes
    }
}

function New-OverlayAuditRow {
    param(
        $BaselineAudit,
        $CandidateAudit,
        [string]$FamilyId
    )

    [pscustomobject]@{
        family_id = $FamilyId
        baseline_gate_feature = if ($BaselineAudit) { $BaselineAudit.gate_feature } else { "-" }
        candidate_gate_feature = if ($CandidateAudit) { $CandidateAudit.gate_feature } else { "-" }
        baseline_scenarios = if ($BaselineAudit) { $BaselineAudit.scenario_count } else { 0 }
        candidate_scenarios = if ($CandidateAudit) { $CandidateAudit.scenario_count } else { 0 }
        baseline_gate_active_eval = if ($BaselineAudit) { $BaselineAudit.evaluation_gate_active_row_count } else { 0 }
        candidate_gate_active_eval = if ($CandidateAudit) { $CandidateAudit.evaluation_gate_active_row_count } else { 0 }
        baseline_positives = if ($BaselineAudit) { $BaselineAudit.positive_label_count } else { 0 }
        candidate_positives = if ($CandidateAudit) { $CandidateAudit.positive_label_count } else { 0 }
        baseline_note = if ($BaselineAudit) { $BaselineAudit.note } else { "-" }
        candidate_note = if ($CandidateAudit) { $CandidateAudit.note } else { "-" }
    }
}

$baselineEval = Load-HorizonRecord -ReleaseId $BaselineReleaseId -Suffix "-evaluation.json" -TargetHorizonDays $HorizonDays
$candidateEval = Load-HorizonRecord -ReleaseId $CandidateReleaseId -Suffix "-evaluation.json" -TargetHorizonDays $HorizonDays
$baselineBundle = Load-HorizonRecord -ReleaseId $BaselineReleaseId -Suffix ".json" -TargetHorizonDays $HorizonDays
$candidateBundle = Load-HorizonRecord -ReleaseId $CandidateReleaseId -Suffix ".json" -TargetHorizonDays $HorizonDays

$baselineMap = Build-CoefficientMap -Coefficients $baselineBundle.record.raw_model.coefficients
$candidateMap = Build-CoefficientMap -Coefficients $candidateBundle.record.raw_model.coefficients

$curveRows = foreach ($featureName in $CurveFeatureNames) {
    New-WeightRow -BaselineMap $baselineMap -CandidateMap $candidateMap -FeatureName $featureName
}
$usdJpyRows = foreach ($featureName in $UsdJpyFeatureNames) {
    New-WeightRow -BaselineMap $baselineMap -CandidateMap $candidateMap -FeatureName $featureName
}
$familyRows = foreach ($featureName in $FamilyFeatureNames) {
    New-WeightRow -BaselineMap $baselineMap -CandidateMap $candidateMap -FeatureName $featureName
}
$broadScoreRows = foreach ($featureName in $BroadScoreFeatureNames) {
    New-WeightRow -BaselineMap $baselineMap -CandidateMap $candidateMap -FeatureName $featureName
}

$thresholdRows = New-ThresholdSummaryRows -BaselineEval $baselineEval -CandidateEval $candidateEval
$thresholdTakeaway = Get-ThresholdRoleTakeaway -BaselineEval $baselineEval -CandidateEval $candidateEval

$guardrailRows = @(
    New-GuardrailStatusRow `
        -Item "curve tail nonnegative" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d tail_neg__us_curve_10y2y_level__0 must stay >= 0 (current guardrail pins it to 0)." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "tail_neg__us_curve_10y2y_level__0") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_neg__us_curve_10y2y_level__0") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.0
    New-GuardrailStatusRow `
        -Item "curve/fed-funds interaction stabilizer band" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d interaction__us_curve_10y2y_level__us_fed_funds_level should stay in 0.18..0.46 so high-rate inversion keeps a stabilizing offset without becoming too broad." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "interaction__us_curve_10y2y_level__us_fed_funds_level") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "interaction__us_curve_10y2y_level__us_fed_funds_level") `
        -MinAllowed 0.18 `
        -MaxAllowed 0.46
    New-GuardrailStatusRow `
        -Item "USDJPY base-level band" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d us_usdjpy_level should stay inside the positive 0.30..0.40 band." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "us_usdjpy_level") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "us_usdjpy_level") `
        -MinAllowed 0.30 `
        -MaxAllowed 0.40
    New-GuardrailStatusRow `
        -Item "USDJPY high-level tail band" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d tail_pos__us_usdjpy_level__145 must stay nonnegative and <= 0.18 so high USDJPY cannot become a large crisis-probability suppressor." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "tail_pos__us_usdjpy_level__145") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_pos__us_usdjpy_level__145") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.18
    New-GuardrailStatusRow `
        -Item "USDJPY external interaction cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d interaction__external_dimension_score__us_usdjpy_level should stay <= 0.58." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "interaction__external_dimension_score__us_usdjpy_level") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "interaction__external_dimension_score__us_usdjpy_level") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.58
    New-GuardrailStatusRow `
        -Item "rate_shock proxy cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d family_proxy__rate_shock should stay <= 0.06." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "family_proxy__rate_shock") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "family_proxy__rate_shock") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.06
    New-GuardrailStatusRow `
        -Item "rate_shock context cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d family_context__rate_shock__external_dimension_score should stay <= 0.12." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "family_context__rate_shock__external_dimension_score") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "family_context__rate_shock__external_dimension_score") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.12
    New-GuardrailStatusRow `
        -Item "jpy_carry proxy cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d family_proxy__jpy_carry should stay <= 0.06." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "family_proxy__jpy_carry") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "family_proxy__jpy_carry") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.06
    New-GuardrailStatusRow `
        -Item "jpy_carry context cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d family_context__jpy_carry__external_dimension_score should stay <= 0.10." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "family_context__jpy_carry__external_dimension_score") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "family_context__jpy_carry__external_dimension_score") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.10
    New-GuardrailStatusRow `
        -Item "trigger score broad-lift cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d trigger_score should stay <= 0.65 in family-context heads so broad trigger pressure cannot dominate false-positive windows." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "trigger_score") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "trigger_score") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.65
    New-GuardrailStatusRow `
        -Item "external-dimension broad-lift cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d external_dimension_score should stay <= 0.42 in family-context heads so external pressure stays contextual rather than a generic 20d driver." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "external_dimension_score") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "external_dimension_score") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.42
    New-GuardrailStatusRow `
        -Item "trigger high-tail broad-lift cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d tail_pos__trigger_score__50 should stay <= 0.35 in family-context heads so high trigger pressure cannot bypass the base broad-score cap." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "tail_pos__trigger_score__50") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_pos__trigger_score__50") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.35
    New-GuardrailStatusRow `
        -Item "external-dimension high-tail broad-lift cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "20d tail_pos__external_dimension_score__50 should stay <= 0.25 in family-context heads so external pressure remains contextual even through high-tail features." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "tail_pos__external_dimension_score__50") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_pos__external_dimension_score__50") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.25
    New-GuardrailStatusRow `
        -Item "USDJPY signed 20d change neutralization" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "Signed us_usdjpy_change_20d must stay pinned at 0; carry-speed risk should flow through absolute-change tail and jpy_carry family proxy." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "us_usdjpy_change_20d") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "us_usdjpy_change_20d") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.0
    New-GuardrailStatusRow `
        -Item "USDJPY signed trigger-change interaction neutralization" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "Signed interaction__trigger_score__us_usdjpy_change_20d must stay pinned at 0 to avoid turning carry direction into a suppressor." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "interaction__trigger_score__us_usdjpy_change_20d") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "interaction__trigger_score__us_usdjpy_change_20d") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.0
    New-GuardrailStatusRow `
        -Item "USDJPY absolute 20d change tail cap" `
        -Coverage "training_guardrail" `
        -EntryPoint "apps/worker/src/model/constraints.rs" `
        -Rule "tail_abs_pos__us_usdjpy_change_20d__4 should stay nonnegative and auxiliary, capped at 0.22." `
        -BaselineValue (Get-CoefficientWeight -Map $baselineMap -FeatureName "tail_abs_pos__us_usdjpy_change_20d__4") `
        -CandidateValue (Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_abs_pos__us_usdjpy_change_20d__4") `
        -MinAllowed 0.0 `
        -MaxAllowed 0.22
    [pscustomobject]@{
        item = "bond-spread suppressor prohibition"
        coverage = "doc_only"
        entry_point = "apps/worker/src/model/constraints.rs"
        rule = "tail_pos__us_baa_10y_spread_level__2 should not be turned into a new negative suppressor; currently no hard training guardrail exists."
        baseline = [math]::Round((Get-CoefficientWeight -Map $baselineMap -FeatureName "tail_pos__us_baa_10y_spread_level__2"), 6)
        candidate = [math]::Round((Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_pos__us_baa_10y_spread_level__2"), 6)
        status = "doc_only"
        notes = "currently a documented constraint, not an enforced training bound"
    }
    [pscustomobject]@{
        item = "20d threshold role"
        coverage = "partial_policy"
        entry_point = "apps/worker/src/probability/threshold/decision/{selection,regime}.rs"
        rule = "Threshold is a soft policy/repair layer; raw positive-window probability must not be crushed and then rescued by threshold lowering."
        baseline = [math]::Round(([double]$baselineEval.record.evaluation.regime_separation.positive_window_avg_probability - [double]$baselineEval.record.threshold_diagnostics.final_threshold), 6)
        candidate = [math]::Round(([double]$candidateEval.record.evaluation.regime_separation.positive_window_avg_probability - [double]$candidateEval.record.threshold_diagnostics.final_threshold), 6)
        status = "partial"
        notes = $thresholdTakeaway
    }
)

$baselineAudits = @{}
foreach ($audit in $baselineBundle.record.family_overlay_audits) {
    $baselineAudits[$audit.family_id] = $audit
}
$candidateAudits = @{}
foreach ($audit in $candidateBundle.record.family_overlay_audits) {
    $candidateAudits[$audit.family_id] = $audit
}

$overlayRows = @(
    New-OverlayAuditRow -BaselineAudit $baselineAudits["rate_shock"] -CandidateAudit $candidateAudits["rate_shock"] -FamilyId "rate_shock"
    New-OverlayAuditRow -BaselineAudit $baselineAudits["jpy_carry"] -CandidateAudit $candidateAudits["jpy_carry"] -FamilyId "jpy_carry"
)

$takeaways = @()
if ((Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_neg__us_curve_10y2y_level__0") -lt 0.0) {
    $takeaways += "candidate is still using a negative curve tail suppressor on 20d; current training guardrail should reject this branch."
}
if ((Get-CoefficientWeight -Map $candidateMap -FeatureName "us_usdjpy_level") -lt 0.30) {
    $takeaways += "candidate still pushes USDJPY base level below the current positive band; this is the blunt-suppression branch the audit is meant to avoid."
}
if ((Get-CoefficientWeight -Map $candidateMap -FeatureName "tail_pos__us_usdjpy_level__145") -lt 0.0) {
    $takeaways += "candidate still lets high USDJPY tail act as a negative suppressor on 20d; this can hide carry-unwind risk and should be rejected by the training guardrail."
}
if ((Get-CoefficientWeight -Map $candidateMap -FeatureName "interaction__external_dimension_score__us_usdjpy_level") -gt 0.58) {
    $takeaways += "candidate still over-expands the USDJPY external interaction; keep this semantics in constrained context space instead."
}
if ($candidateAudits.ContainsKey("jpy_carry") -and $candidateAudits["jpy_carry"].positive_label_count -eq 0) {
    $takeaways += "jpy_carry remains proxy-only in current bundle audits; do not treat it as a labeled primary family yet."
}
$takeaways += $thresholdTakeaway

$summary = [pscustomobject]@{
    baseline_release_id = $BaselineReleaseId
    candidate_release_id = $CandidateReleaseId
    horizon_days = $HorizonDays
    threshold_rows = $thresholdRows
    threshold_takeaway = $thresholdTakeaway
    curve_rows = $curveRows
    usdjpy_rows = $usdJpyRows
    family_rows = $familyRows
    broad_score_rows = $broadScoreRows
    guardrail_rows = $guardrailRows
    overlay_rows = $overlayRows
    takeaways = $takeaways | Select-Object -Unique
}

Write-Host "Formal candidate semantics audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  horizon  : ${HorizonDays}d"
Write-Host ""

Write-Host "20d threshold role"
$thresholdRows | Format-Table -AutoSize
Write-Host ("Takeaway: {0}" -f $thresholdTakeaway)
Write-Host ""

Write-Host "Guardrail coverage and status"
$guardrailRows | Format-Table item, coverage, status, baseline, candidate, entry_point -AutoSize
Write-Host ""

Write-Host "Curve / bond-spread pair weights"
$curveRows | Format-Table -AutoSize
Write-Host ""

Write-Host "USDJPY semantics weights"
$usdJpyRows | Format-Table -AutoSize
Write-Host ""

Write-Host "JPY carry / rate_shock context weights"
$familyRows | Format-Table -AutoSize
Write-Host ""

Write-Host "Broad score weights"
$broadScoreRows | Format-Table -AutoSize
Write-Host ""

Write-Host "Family overlay audit status"
$overlayRows | Format-Table -AutoSize
Write-Host ""

Write-Host "Key takeaways"
foreach ($takeaway in ($takeaways | Select-Object -Unique)) {
    Write-Host ("  - {0}" -f $takeaway)
}

if ($OutputPath) {
    $outputFullPath = Join-Path $Root $OutputPath
    $outputDirectory = Split-Path -Parent $outputFullPath
    if ($outputDirectory) {
        New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    }
    $summary | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $outputFullPath
    Write-Host ""
    Write-Host ("JSON summary written to {0}" -f $outputFullPath)
}
