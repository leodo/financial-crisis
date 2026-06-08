param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$HistoryMode = "default",
    [string]$ReportPath = "",
    [string[]]$ScenarioIds = @(
        "us_black_monday_1987",
        "us_early_90s_banking_stress",
        "us_bond_massacre_1994",
        "us_dotcom_unwind_2000",
        "us_gfc_2008",
        "us_funding_stress_2011",
        "us_covid_liquidity_2020",
        "us_rate_shock_2022",
        "us_regional_banks_2023"
    ),
    [string]$MarketScope = "financial_system",
    [string]$DatasetKey = "",
    [string]$DatasetId = "",
    [string]$DatasetVersion = "",
    [string]$OutputDir = "artifacts/research/spa"
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Resolve-ReviewReportPath {
    param(
        [string]$BaselineRelease,
        [string]$CandidateRelease,
        [string]$Mode,
        [string]$ExplicitPath
    )

    if ($ExplicitPath) {
        $resolved = Join-Path $Root $ExplicitPath
        if (-not (Test-Path -LiteralPath $resolved)) {
            throw "Review report was not found: $resolved"
        }
        return (Resolve-Path -LiteralPath $resolved).Path
    }

    $reportDirectory = Join-Path $Root "artifacts/research/release-review"
    $pattern = "*$BaselineRelease-vs-$CandidateRelease-$Mode-release-review.json"
    $report = Get-ChildItem -LiteralPath $reportDirectory -Filter $pattern |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $report) {
        throw "No release-review artifact matched baseline=$BaselineRelease candidate=$CandidateRelease history_mode=$Mode."
    }

    $report.FullName
}

function Sanitize-FileComponent {
    param([string]$Value)

    ($Value.ToCharArray() | ForEach-Object {
            if ($_ -match '[A-Za-z0-9._-]') { $_ } else { '_' }
        }) -join ''
}

function Find-LatestArtifact {
    param(
        [string]$Directory,
        [string]$Pattern
    )

    $artifact = Get-ChildItem -LiteralPath $Directory -Filter $Pattern |
        Sort-Object LastWriteTime -Descending |
        Select-Object -First 1
    if (-not $artifact) {
        throw "No artifact matched pattern '$Pattern' in $Directory"
    }

    (Resolve-Path -LiteralPath $artifact.FullName).Path
}

function Short-ReleaseToken {
    param([string]$ReleaseId)

    if ($ReleaseId -match '([0-9]{8}T[0-9]{6})$') {
        return $Matches[1]
    }

    $sanitized = Sanitize-FileComponent -Value $ReleaseId
    if ($sanitized.Length -le 24) {
        return $sanitized
    }

    $sanitized.Substring($sanitized.Length - 24)
}

function Round-Safe {
    param(
        $Value,
        [int]$Digits = 6
    )

    if ($null -eq $Value) {
        return $null
    }
    if ($Value -isnot [ValueType] -and $Value -isnot [string]) {
        return $null
    }

    $number = 0.0
    if (-not [double]::TryParse($Value.ToString(), [ref]$number)) {
        return $null
    }

    [math]::Round($number, $Digits)
}

function Measure-RatioSafe {
    param(
        $Numerator,
        $Denominator
    )

    $num = Round-Safe -Value $Numerator -Digits 12
    $den = Round-Safe -Value $Denominator -Digits 12
    if ($null -eq $num -or $null -eq $den -or [math]::Abs($den) -lt 1e-12) {
        return $null
    }

    Round-Safe -Value ($num / $den) -Digits 6
}

function Read-JsonFile {
    param([string]$Path)

    Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Add-DatasetSelector {
    param(
        [System.Collections.Generic.List[object]]$Selectors,
        [string]$SelectorKey = "",
        [string]$SelectorId = "",
        [string]$SelectorVersion = "",
        [string]$Reason = ""
    )

    $identity = if ($SelectorKey) {
        "key:{0}" -f $SelectorKey
    } elseif ($SelectorVersion) {
        "id:{0}:{1}" -f $SelectorId, $SelectorVersion
    } else {
        "id:{0}" -f $SelectorId
    }

    if ($Selectors | Where-Object { $_.identity -eq $identity }) {
        return
    }

    $Selectors.Add([pscustomobject]@{
            identity = $identity
            dataset_key = $SelectorKey
            dataset_id = $SelectorId
            dataset_version = $SelectorVersion
            reason = $Reason
        })
}

function Resolve-ScenarioDatasetSelectors {
    param([pscustomobject]$Scenario)

    $selectors = New-Object 'System.Collections.Generic.List[object]'
    if ($DatasetKey) {
        Add-DatasetSelector -Selectors $selectors -SelectorKey $DatasetKey -Reason "explicit dataset key"
        return $selectors.ToArray()
    }

    if ($DatasetId) {
        Add-DatasetSelector -Selectors $selectors -SelectorId $DatasetId -SelectorVersion $DatasetVersion -Reason "explicit dataset id"
        return $selectors.ToArray()
    }

    $trainingRole = [string]$Scenario.training_role
    $scenarioFamily = [string]$Scenario.family
    $preWarningStart = $null
    if ($Scenario.pre_warning_start) {
        try {
            $preWarningStart = [datetime]$Scenario.pre_warning_start
        } catch {
            $preWarningStart = $null
        }
    }

    $isAcuteExtension = $scenarioFamily -eq "acute_market_liquidity_crash" -and (
        $trainingRole -eq "extension_only" -or
        ($preWarningStart -and $preWarningStart -lt [datetime]"1990-01-01")
    )
    $needsStressExtension = (
        $Scenario.protected_window -or
        @("candidate_optional", "extension_only") -contains $trainingRole
    ) -and -not $isAcuteExtension

    if ($isAcuteExtension) {
        Add-DatasetSelector -Selectors $selectors -SelectorId "formal_v1_ext_acute_pre1990" -Reason "acute extension coverage"
    }

    if ($needsStressExtension) {
        Add-DatasetSelector -Selectors $selectors -SelectorId "formal_v1_ext_stress_1990_daily" -Reason "protected or extension coverage"
    }

    Add-DatasetSelector -Selectors $selectors -SelectorId "formal_v1_main_1990_daily" -Reason "main baseline coverage"

    if (-not $isAcuteExtension -and $scenarioFamily -eq "acute_market_liquidity_crash") {
        Add-DatasetSelector -Selectors $selectors -SelectorId "formal_v1_ext_acute_pre1990" -Reason "acute fallback coverage"
    }

    if (-not $needsStressExtension) {
        Add-DatasetSelector -Selectors $selectors -SelectorId "formal_v1_ext_stress_1990_daily" -Reason "stress fallback coverage"
    }

    return $selectors.ToArray()
}

function Load-ScenarioPackMetadata {
    param([string[]]$RequestedScenarioIds)

    $catalogPath = Join-Path $Root "config/research_crisis_scenarios.us.json"
    $coveragePath = Join-Path $Root "config/research_scenario_data_coverage.us.json"
    $catalog = Read-JsonFile -Path $catalogPath
    $coverage = Read-JsonFile -Path $coveragePath

    $resolved = foreach ($scenarioId in $RequestedScenarioIds) {
        $scenario = $catalog.scenarios | Where-Object { $_.scenario_id -eq $scenarioId } | Select-Object -First 1
        if (-not $scenario) {
            throw "Scenario catalog does not contain scenario_id=$scenarioId"
        }

        $coverageRecord = $coverage.records | Where-Object { $_.scenario_id -eq $scenarioId } | Select-Object -First 1
        if (-not $coverageRecord) {
            throw "Scenario coverage catalog does not contain scenario_id=$scenarioId"
        }

        [pscustomobject]@{
            scenario_id = $scenario.scenario_id
            scenario_label = $scenario.label
            family = $scenario.family
            pre_warning_start = $scenario.pre_warning_start
            crisis_start = $scenario.crisis_start
            acute_start = $scenario.acute_start
            crisis_peak = $scenario.crisis_peak
            crisis_end = $scenario.crisis_end
            default_horizon_roles = @($scenario.default_horizon_roles)
            training_role = $scenario.training_role
            protected_window = [bool]$scenario.protected_window
            protected_action_levels = @($scenario.protected_action_levels)
            evidence_basis = $scenario.evidence_basis
            recommended_role = $coverageRecord.recommended_role
            coverage_grade = $coverageRecord.coverage_grade
            point_in_time_mode = $coverageRecord.point_in_time_mode
            usable_for_main_training = [bool]$coverageRecord.usable_for_main_training
            usable_for_extension_training = [bool]$coverageRecord.usable_for_extension_training
            usable_for_protected_stress = [bool]$coverageRecord.usable_for_protected_stress
            usable_for_historical_analog = [bool]$coverageRecord.usable_for_historical_analog
            current_status = $coverageRecord.current_status
            free_sources = @($coverageRecord.free_sources)
            blocking_gaps = @($coverageRecord.blocking_gaps)
        }
    }

    @($resolved)
}

function Resolve-ReviewScenarioContext {
    param(
        $ReviewReport,
        [string]$ScenarioId
    )

    $focus = @($ReviewReport.scenario_focus | Where-Object { $_.scenario_id -eq $ScenarioId }) | Select-Object -First 1
    $backtest = @($ReviewReport.comparison.backtest_scenarios | Where-Object { $_.scenario_id -eq $ScenarioId }) | Select-Object -First 1
    $priority = @($ReviewReport.historical_audit_priorities | Where-Object { $_.scenario_id -eq $ScenarioId }) | Select-Object -First 1

    $dominantBlocks = $focus.dominant_runtime_blocks
    $dominantContinuity = $focus.dominant_runtime_continuity_facets

    [pscustomobject]@{
        review_present = [bool]($focus -or $backtest -or $priority)
        outcome = $backtest.outcome
        signal_source = $backtest.signal_source
        baseline_lead_time_days = $backtest.baseline_lead_time_days
        candidate_lead_time_days = $backtest.candidate_lead_time_days
        baseline_actionable_lead_time_days = $backtest.baseline_actionable_lead_time_days
        candidate_actionable_lead_time_days = $backtest.candidate_actionable_lead_time_days
        actionable_delta_days = $backtest.actionable_delta_days
        primary_workstream = $priority.primary_workstream
        suggested_review = $priority.suggested_review
        baseline_primary_failure_mode = $focus.baseline_primary_failure_mode
        candidate_primary_failure_mode = $focus.candidate_primary_failure_mode
        baseline_dominant_runtime_block = if ($dominantBlocks) { ($dominantBlocks.baseline_categories -join " + ") } else { $null }
        candidate_dominant_runtime_block = if ($dominantBlocks) { ($dominantBlocks.candidate_categories -join " + ") } else { $null }
        baseline_dominant_continuity_facet = if ($dominantContinuity) { ($dominantContinuity.baseline_categories -join " + ") } else { $null }
        candidate_dominant_continuity_facet = if ($dominantContinuity) { ($dominantContinuity.candidate_categories -join " + ") } else { $null }
        baseline_first_runtime_floor_hit_without_l3_reason = $focus.baseline_first_runtime_floor_hit_without_l3_reason
        candidate_first_runtime_floor_hit_without_l3_reason = $focus.candidate_first_runtime_floor_hit_without_l3_reason
    }
}

function Invoke-FormalCompareWithFallback {
    param(
        [pscustomobject]$Scenario,
        [string]$CompareOutputDir
    )

    $attempts = New-Object 'System.Collections.Generic.List[object]'
    foreach ($selector in (Resolve-ScenarioDatasetSelectors -Scenario $Scenario)) {
        $args = @(
            "run", "-p", "fc-worker", "--",
            "research", "release", "formal-probability-compare",
            "--market-scope", $MarketScope,
            "--baseline-release-id", $BaselineReleaseId,
            "--candidate-release-id", $CandidateReleaseId,
            "--scenario-id", $Scenario.scenario_id,
            "--from", $Scenario.pre_warning_start,
            "--to", $Scenario.crisis_end,
            "--output-dir", $CompareOutputDir
        )

        if ($selector.dataset_key) {
            $args += @("--dataset-key", $selector.dataset_key)
        } elseif ($selector.dataset_id) {
            $args += @("--dataset-id", $selector.dataset_id)
            if ($selector.dataset_version) {
                $args += @("--dataset-version", $selector.dataset_version)
            }
        }

        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            $commandOutput = & cargo @args 2>&1
            $exitCode = $LASTEXITCODE
        } finally {
            $ErrorActionPreference = $previousErrorActionPreference
        }

        $commandText = ($commandOutput | ForEach-Object { $_.ToString() }) -join [Environment]::NewLine
        $isEmptyWindow = $commandText -like "*has no rows*" -or $commandText -like "*no overlapping rows found*"

        if ($exitCode -eq 0) {
            $pattern = "{0}-vs-{1}-{2}-{3}-formal-probability-compare-{4}.json" -f `
                (Sanitize-FileComponent $BaselineReleaseId), `
                (Sanitize-FileComponent $CandidateReleaseId), `
                (Sanitize-FileComponent $Scenario.pre_warning_start), `
                (Sanitize-FileComponent $Scenario.crisis_end), `
                (Sanitize-FileComponent $Scenario.scenario_id)
            $artifactPath = Join-Path $CompareOutputDir $pattern
            for ($attempt = 0; $attempt -lt 10 -and -not (Test-Path -LiteralPath $artifactPath); $attempt++) {
                Start-Sleep -Milliseconds 200
            }
            if (-not (Test-Path -LiteralPath $artifactPath)) {
                $artifact = Get-ChildItem -LiteralPath $CompareOutputDir -Filter "*.json" |
                    Where-Object { $_.Name -eq $pattern } |
                    Select-Object -First 1
                if ($artifact) {
                    $artifactPath = $artifact.FullName
                } else {
                    throw "Expected compare artifact was not found: $artifactPath"
                }
            }

            return [pscustomobject]@{
                status = "ok"
                selector_reason = $selector.reason
                selector_identity = $selector.identity
                dataset_key = $selector.dataset_key
                dataset_id = $selector.dataset_id
                dataset_version = $selector.dataset_version
                compare_json_path = $artifactPath
                attempts = $attempts.ToArray()
            }
        }

        $attempts.Add([pscustomobject]@{
                selector_reason = $selector.reason
                selector_identity = $selector.identity
                dataset_key = $selector.dataset_key
                dataset_id = $selector.dataset_id
                dataset_version = $selector.dataset_version
                exit_code = $exitCode
                empty_window = $isEmptyWindow
                output = $commandText
            })

        if ($isEmptyWindow) {
            Write-Host ("Skipped empty compare window for {0} using {1}" -f $Scenario.scenario_id, $selector.identity)
            continue
        }

        throw "formal-probability-compare failed for scenario $($Scenario.scenario_id)`n$commandText"
    }

    return [pscustomobject]@{
        status = "missing"
        selector_reason = $null
        selector_identity = $null
        dataset_key = $null
        dataset_id = $null
        dataset_version = $null
        compare_json_path = $null
        attempts = $attempts.ToArray()
    }
}

function Read-ThresholdValue {
    param(
        [object[]]$Thresholds,
        [int]$HorizonDays
    )

    $row = @($Thresholds | Where-Object { [int]$_.horizon_days -eq $HorizonDays }) | Select-Object -First 1
    if (-not $row) {
        return $null
    }

    $row.decision_threshold
}

function Build-CompareSummary {
    param([pscustomobject]$CompareResult)

    $attemptedDatasets = @(
        $CompareResult.attempts |
            ForEach-Object {
                if ($_.dataset_key) {
                    $_.dataset_key
                } elseif ($_.dataset_version) {
                    "{0}:{1}" -f $_.dataset_id, $_.dataset_version
                } elseif ($_.dataset_id) {
                    $_.dataset_id
                } else {
                    $_.selector_identity
                }
            } |
            Where-Object { $_ } |
            Sort-Object -Unique
    )

    if ($CompareResult.status -ne "ok") {
        return [pscustomobject]@{
            compare_status = "missing"
            compare_selector_reason = $null
            compare_selector_identity = $null
            compare_dataset_key = $null
            compare_json_path = $null
            attempted_datasets = $attemptedDatasets
            row_count = 0
            baseline_threshold_20d = $null
            candidate_threshold_20d = $null
            baseline_threshold_60d = $null
            candidate_threshold_60d = $null
            baseline_hit_count_20d = 0
            candidate_hit_count_20d = 0
            baseline_hit_count_60d = 0
            candidate_hit_count_60d = 0
            overall_avg_delta_p_20d = $null
            overall_avg_delta_p_60d = $null
            overall_baseline_hit_rate_20d = $null
            overall_candidate_hit_rate_20d = $null
            overall_baseline_hit_rate_60d = $null
            overall_candidate_hit_rate_60d = $null
            positive_window_baseline_hit_rate_20d = $null
            positive_window_candidate_hit_rate_20d = $null
            positive_window_retention_20d = $null
            positive_window_baseline_avg_gap_20d = $null
            positive_window_candidate_avg_gap_20d = $null
            hedge_window_candidate_hit_rate_20d = $null
            top_feature_deltas_20d = @()
            top_feature_deltas_60d = @()
            compare_gap_reason = "No dataset rows were available for this scenario window in the candidate dataset selectors."
        }
    }

    $compare = Read-JsonFile -Path $CompareResult.compare_json_path
    $overall = $compare.summary.overall_window
    $positiveWindow = $compare.summary.positive_window_20d
    $hedgeWindow = $compare.summary.hedge_window

    [pscustomobject]@{
        compare_status = "ok"
        compare_selector_reason = $CompareResult.selector_reason
        compare_selector_identity = $CompareResult.selector_identity
        compare_dataset_key = $compare.dataset_key
        compare_json_path = $CompareResult.compare_json_path
        attempted_datasets = if ($compare.dataset_key) {
            @($attemptedDatasets + $compare.dataset_key | Sort-Object -Unique)
        } else {
            $attemptedDatasets
        }
        row_count = [int]$compare.row_count
        baseline_threshold_20d = Round-Safe -Value (Read-ThresholdValue -Thresholds $compare.baseline_thresholds -HorizonDays 20)
        candidate_threshold_20d = Round-Safe -Value (Read-ThresholdValue -Thresholds $compare.candidate_thresholds -HorizonDays 20)
        baseline_threshold_60d = Round-Safe -Value (Read-ThresholdValue -Thresholds $compare.baseline_thresholds -HorizonDays 60)
        candidate_threshold_60d = Round-Safe -Value (Read-ThresholdValue -Thresholds $compare.candidate_thresholds -HorizonDays 60)
        baseline_hit_count_20d = [int]$compare.summary.baseline_hit_count_20d
        candidate_hit_count_20d = [int]$compare.summary.candidate_hit_count_20d
        baseline_hit_count_60d = [int]$compare.summary.baseline_hit_count_60d
        candidate_hit_count_60d = [int]$compare.summary.candidate_hit_count_60d
        overall_avg_delta_p_20d = Round-Safe -Value $overall.avg_delta_p_20d
        overall_avg_delta_p_60d = Round-Safe -Value $overall.avg_delta_p_60d
        overall_baseline_hit_rate_20d = Round-Safe -Value $overall.baseline_hit_rate_20d
        overall_candidate_hit_rate_20d = Round-Safe -Value $overall.candidate_hit_rate_20d
        overall_baseline_hit_rate_60d = Round-Safe -Value $overall.baseline_hit_rate_60d
        overall_candidate_hit_rate_60d = Round-Safe -Value $overall.candidate_hit_rate_60d
        positive_window_baseline_hit_rate_20d = Round-Safe -Value $positiveWindow.baseline_hit_rate_20d
        positive_window_candidate_hit_rate_20d = Round-Safe -Value $positiveWindow.candidate_hit_rate_20d
        positive_window_retention_20d = Measure-RatioSafe `
            -Numerator $positiveWindow.candidate_hit_rate_20d `
            -Denominator $positiveWindow.baseline_hit_rate_20d
        positive_window_baseline_avg_gap_20d = Round-Safe -Value $positiveWindow.baseline_avg_gap_to_threshold_20d
        positive_window_candidate_avg_gap_20d = Round-Safe -Value $positiveWindow.candidate_avg_gap_to_threshold_20d
        hedge_window_candidate_hit_rate_20d = Round-Safe -Value $hedgeWindow.candidate_hit_rate_20d
        top_feature_deltas_20d = @($overall.top_feature_deltas_20d | Select-Object -First 5)
        top_feature_deltas_60d = @($overall.top_feature_deltas_60d | Select-Object -First 5)
    }
}

function Resolve-BlockerClass {
    param(
        [pscustomobject]$ReviewContext,
        [pscustomobject]$CompareSummary
    )

    if ($CompareSummary.compare_status -ne "ok") {
        return "dataset_coverage_gap"
    }

    $mode = [string]$ReviewContext.candidate_primary_failure_mode
    $runtimeBlock = [string]$ReviewContext.candidate_dominant_runtime_block
    $continuityFacet = [string]$ReviewContext.candidate_dominant_continuity_facet
    $reason = [string]$ReviewContext.candidate_first_runtime_floor_hit_without_l3_reason

    if (
        $mode -eq "strict_review_vs_runtime_mapping" -or
        $runtimeBlock -like "*review_gate_gap*" -or
        $reason -like "*review_gate_gap*"
    ) {
        return "review_gate_gap"
    }

    if (
        $mode -eq "posture_continuity_failure" -or
        ($continuityFacet -and $continuityFacet -ne "-")
    ) {
        return "posture_continuity"
    }

    if (
        $mode -eq "residual_review_l3_failure" -or
        $reason -like "*review_l3*"
    ) {
        return "residual_review_l3"
    }

    if ($ReviewContext.primary_workstream -eq "weak_signal_continuity") {
        return "posture_continuity"
    }
    if ($ReviewContext.primary_workstream -eq "prewarning_signal_gap") {
        return "review_gate_gap"
    }

    if ($mode) {
        return $mode
    }

    $retention = $CompareSummary.positive_window_retention_20d
    $delta20 = $CompareSummary.overall_avg_delta_p_20d
    if ($null -ne $retention -and $retention -lt 0.8) {
        return "candidate_probability_continuity_regression"
    }
    if ($null -ne $delta20 -and $delta20 -lt -0.05) {
        return "candidate_probability_level_drop"
    }

    "no_review_focus_signal"
}

function Build-Takeaway {
    param(
        [string]$BlockerClass,
        [pscustomobject]$Scenario,
        [pscustomobject]$ReviewContext,
        [pscustomobject]$CompareSummary
    )

    switch ($BlockerClass) {
        "dataset_coverage_gap" {
            return "No usable rows exist for this scenario window in the current formal datasets; backfill free history or extend dataset coverage first."
        }
        "review_gate_gap" {
            return "Runtime risk is already visible here, but strict review still fails mainly on gate mapping; review p20d/p60d strict conditions first."
        }
        "posture_continuity" {
            return "This is not a no-signal case; the main problem is broken posture/months/trigger continuity, so repair continuity first."
        }
        "residual_review_l3" {
            return "The main blocker is no longer floor or continuity; the remaining gap is the final strict L3 actionable conversion."
        }
        "candidate_probability_continuity_regression" {
            return "The candidate is materially weaker than baseline on positive-window 20d continuity; inspect training targets and threshold lift first."
        }
        "candidate_probability_level_drop" {
            return "The candidate pushes the overall 20d probability level down too far; inspect curve, trigger, and USDJPY feature weights first."
        }
        default {
            if ($ReviewContext.primary_workstream) {
                return "This looks more like a shared workstream issue; continue with the historical workstream named by release review."
            }
            return "There is no clear focus blocker yet; continue manual review with compare feature deltas and coverage gaps."
        }
    }
}

function Group-ScenarioCounts {
    param(
        [object[]]$Rows,
        [scriptblock]$Selector
    )

    @(
        $Rows |
            Group-Object -Property $Selector |
            Sort-Object Name |
            ForEach-Object {
                [pscustomobject]@{
                    key = $_.Name
                    count = $_.Count
                    scenarios = @($_.Group | ForEach-Object { $_.scenario_id })
                }
            }
    )
}

$reviewReportPath = Resolve-ReviewReportPath `
    -BaselineRelease $BaselineReleaseId `
    -CandidateRelease $CandidateReleaseId `
    -Mode $HistoryMode `
    -ExplicitPath $ReportPath
$reviewReport = Read-JsonFile -Path $reviewReportPath
$scenarios = Load-ScenarioPackMetadata -RequestedScenarioIds $ScenarioIds

$resolvedOutputDir = Join-Path $Root $OutputDir
New-Item -ItemType Directory -Force -Path $resolvedOutputDir | Out-Null
$compareOutputDir = Join-Path $resolvedOutputDir "cmp"
New-Item -ItemType Directory -Force -Path $compareOutputDir | Out-Null

$scenarioSummaries = foreach ($scenario in $scenarios) {
    Write-Host ("Auditing scenario {0} ({1} -> {2})" -f $scenario.scenario_id, $scenario.pre_warning_start, $scenario.crisis_end)
    $reviewContext = Resolve-ReviewScenarioContext -ReviewReport $reviewReport -ScenarioId $scenario.scenario_id
    $compareResult = Invoke-FormalCompareWithFallback -Scenario $scenario -CompareOutputDir $compareOutputDir
    $compareSummary = Build-CompareSummary -CompareResult $compareResult
    $blockerClass = Resolve-BlockerClass -ReviewContext $reviewContext -CompareSummary $compareSummary
    $takeaway = Build-Takeaway -BlockerClass $blockerClass -Scenario $scenario -ReviewContext $reviewContext -CompareSummary $compareSummary

    [pscustomobject]@{
        scenario_id = $scenario.scenario_id
        scenario_label = $scenario.scenario_label
        family = $scenario.family
        training_role = $scenario.training_role
        recommended_role = $scenario.recommended_role
        coverage_grade = $scenario.coverage_grade
        point_in_time_mode = $scenario.point_in_time_mode
        current_status = $scenario.current_status
        pre_warning_start = $scenario.pre_warning_start
        crisis_start = $scenario.crisis_start
        acute_start = $scenario.acute_start
        crisis_peak = $scenario.crisis_peak
        crisis_end = $scenario.crisis_end
        protected_window = $scenario.protected_window
        protected_action_levels = @($scenario.protected_action_levels)
        usable_for_main_training = $scenario.usable_for_main_training
        usable_for_extension_training = $scenario.usable_for_extension_training
        usable_for_protected_stress = $scenario.usable_for_protected_stress
        usable_for_historical_analog = $scenario.usable_for_historical_analog
        free_sources = @($scenario.free_sources)
        blocking_gaps = @($scenario.blocking_gaps)
        evidence_basis = $scenario.evidence_basis
        review_present = $reviewContext.review_present
        primary_workstream = $reviewContext.primary_workstream
        suggested_review = $reviewContext.suggested_review
        outcome = $reviewContext.outcome
        signal_source = $reviewContext.signal_source
        baseline_lead_time_days = $reviewContext.baseline_lead_time_days
        candidate_lead_time_days = $reviewContext.candidate_lead_time_days
        baseline_actionable_lead_time_days = $reviewContext.baseline_actionable_lead_time_days
        candidate_actionable_lead_time_days = $reviewContext.candidate_actionable_lead_time_days
        actionable_delta_days = $reviewContext.actionable_delta_days
        baseline_primary_failure_mode = $reviewContext.baseline_primary_failure_mode
        candidate_primary_failure_mode = $reviewContext.candidate_primary_failure_mode
        baseline_dominant_runtime_block = $reviewContext.baseline_dominant_runtime_block
        candidate_dominant_runtime_block = $reviewContext.candidate_dominant_runtime_block
        baseline_dominant_continuity_facet = $reviewContext.baseline_dominant_continuity_facet
        candidate_dominant_continuity_facet = $reviewContext.candidate_dominant_continuity_facet
        baseline_first_runtime_floor_hit_without_l3_reason = $reviewContext.baseline_first_runtime_floor_hit_without_l3_reason
        candidate_first_runtime_floor_hit_without_l3_reason = $reviewContext.candidate_first_runtime_floor_hit_without_l3_reason
        compare_status = $compareSummary.compare_status
        compare_selector_reason = $compareSummary.compare_selector_reason
        compare_selector_identity = $compareSummary.compare_selector_identity
        compare_dataset_key = $compareSummary.compare_dataset_key
        compare_json_path = $compareSummary.compare_json_path
        attempted_datasets = @($compareSummary.attempted_datasets)
        row_count = $compareSummary.row_count
        baseline_threshold_20d = $compareSummary.baseline_threshold_20d
        candidate_threshold_20d = $compareSummary.candidate_threshold_20d
        baseline_threshold_60d = $compareSummary.baseline_threshold_60d
        candidate_threshold_60d = $compareSummary.candidate_threshold_60d
        baseline_hit_count_20d = $compareSummary.baseline_hit_count_20d
        candidate_hit_count_20d = $compareSummary.candidate_hit_count_20d
        baseline_hit_count_60d = $compareSummary.baseline_hit_count_60d
        candidate_hit_count_60d = $compareSummary.candidate_hit_count_60d
        overall_avg_delta_p_20d = $compareSummary.overall_avg_delta_p_20d
        overall_avg_delta_p_60d = $compareSummary.overall_avg_delta_p_60d
        overall_baseline_hit_rate_20d = $compareSummary.overall_baseline_hit_rate_20d
        overall_candidate_hit_rate_20d = $compareSummary.overall_candidate_hit_rate_20d
        overall_baseline_hit_rate_60d = $compareSummary.overall_baseline_hit_rate_60d
        overall_candidate_hit_rate_60d = $compareSummary.overall_candidate_hit_rate_60d
        positive_window_baseline_hit_rate_20d = $compareSummary.positive_window_baseline_hit_rate_20d
        positive_window_candidate_hit_rate_20d = $compareSummary.positive_window_candidate_hit_rate_20d
        positive_window_retention_20d = $compareSummary.positive_window_retention_20d
        positive_window_baseline_avg_gap_20d = $compareSummary.positive_window_baseline_avg_gap_20d
        positive_window_candidate_avg_gap_20d = $compareSummary.positive_window_candidate_avg_gap_20d
        hedge_window_candidate_hit_rate_20d = $compareSummary.hedge_window_candidate_hit_rate_20d
        top_feature_deltas_20d = @($compareSummary.top_feature_deltas_20d)
        top_feature_deltas_60d = @($compareSummary.top_feature_deltas_60d)
        compare_gap_reason = $compareSummary.compare_gap_reason
        blocker_class = $blockerClass
        takeaway = $takeaway
    }
}

$reportStem = "{0}-vs-{1}-{2}-scenario-pack-audit" -f `
    (Short-ReleaseToken -ReleaseId $BaselineReleaseId), `
    (Short-ReleaseToken -ReleaseId $CandidateReleaseId), `
    (Sanitize-FileComponent $HistoryMode)
$jsonPath = Join-Path $resolvedOutputDir "$reportStem.json"

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    baseline_release_id = $BaselineReleaseId
    candidate_release_id = $CandidateReleaseId
    history_mode = $HistoryMode
    market_scope = $MarketScope
    review_report_path = $reviewReportPath
    requested_scenarios = @($ScenarioIds)
    compare_ok_count = @($scenarioSummaries | Where-Object { $_.compare_status -eq "ok" }).Count
    compare_missing_count = @($scenarioSummaries | Where-Object { $_.compare_status -ne "ok" }).Count
    blocker_counts = @(Group-ScenarioCounts -Rows $scenarioSummaries -Selector { $_.blocker_class })
    coverage_grade_counts = @(Group-ScenarioCounts -Rows $scenarioSummaries -Selector { $_.coverage_grade })
    scenario_summaries = @($scenarioSummaries)
}

New-Item -ItemType Directory -Force -Path $resolvedOutputDir | Out-Null
$report | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $jsonPath -Encoding utf8

Write-Host "Formal candidate scenario-pack audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  review   : $reviewReportPath"
Write-Host "  output   : $jsonPath"
Write-Host ""

Write-Host "Blocker mix"
$report.blocker_counts |
    Select-Object key, count, @{Name = "scenarios"; Expression = { $_.scenarios -join ", " } } |
    Format-Table -Wrap -AutoSize
Write-Host ""

Write-Host "Scenario summary"
$scenarioSummaries |
    Select-Object scenario_id, coverage_grade, training_role, compare_status, blocker_class, positive_window_retention_20d, overall_avg_delta_p_20d, candidate_primary_failure_mode |
    Format-Table -Wrap -AutoSize
