param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$HistoryMode = "default",
    [string]$ReportPath = "",
    [string[]]$Workstreams = @("prewarning_signal_gap", "weak_signal_continuity"),
    [string]$MarketScope = "financial_system",
    [string]$DatasetKey = "",
    [string]$DatasetId = "",
    [string]$DatasetVersion = "",
    [string]$OutputDir = "artifacts/research/workstream-audit"
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

function Group-CountSummary {
    param(
        [object[]]$Rows,
        [scriptblock]$Selector
    )

    @($Rows |
        Group-Object -Property $Selector |
        Sort-Object Name |
        ForEach-Object { "{0}={1}" -f $_.Name, $_.Count })
}

function Measure-AverageSafe {
    param(
        [object[]]$Rows,
        [string]$PropertyName
    )

    if (-not $Rows -or $Rows.Count -eq 0) {
        return $null
    }

    $measure = $Rows | Measure-Object -Property $PropertyName -Average
    if ($null -eq $measure.Average) {
        return $null
    }

    [math]::Round([double]$measure.Average, 4)
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

function Resolve-ScenarioAuditRows {
    param(
        [string]$ReviewReportPath,
        [string[]]$IncludeWorkstreams
    )

    $parser = @'
const fs = require("fs");
const reportPath = process.argv[2];
const includeWorkstreams = new Set(process.argv.slice(3));
const review = JSON.parse(fs.readFileSync(reportPath, "utf8"));
const focusById = new Map(
  (review.scenario_focus || []).map((scenario) => [
    scenario.scenario_id,
    {
      window_start: scenario.window_start ?? null,
      window_end: scenario.window_end ?? null,
      crisis_start: scenario.crisis_start ?? null,
      crisis_end: scenario.crisis_end ?? null
    }
  ])
);

const rows = (review.historical_audit_priorities || [])
  .filter((priority) => includeWorkstreams.has(priority.primary_workstream))
  .map((priority) => {
    const focus = focusById.get(priority.scenario_id);
    if (!focus) {
      return null;
    }

    return {
      scenario_id: priority.scenario_id,
      scenario_name: priority.scenario_name,
      workstream: priority.primary_workstream,
      scenario_family: priority.scenario_family ?? null,
      training_role: priority.training_role ?? null,
      protected_window: Boolean(priority.protected_window),
      suggested_review: priority.suggested_review ?? null,
      window_start: focus.window_start,
      window_end: focus.window_end,
      crisis_start: focus.crisis_start,
      crisis_end: focus.crisis_end
    };
  })
  .filter(Boolean);

console.log(
  JSON.stringify(
    {
      reviewed_at: review.reviewed_at ?? null,
      history_mode: review.history_mode ?? null,
      baseline_release: review.baseline_release?.release_id ?? null,
      candidate_release: review.candidate_release?.release_id ?? null,
      rows
    },
    null,
    2
  )
);
'@

    $json = $parser | node - $ReviewReportPath @IncludeWorkstreams
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to parse release-review artifact: $ReviewReportPath"
    }

    $json | ConvertFrom-Json
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
    $scenarioFamily = [string]$Scenario.scenario_family
    $windowStart = $null
    if ($Scenario.window_start) {
        try {
            $windowStart = [datetime]$Scenario.window_start
        } catch {
            $windowStart = $null
        }
    }

    $isAcuteExtension = $scenarioFamily -eq "acute_market_liquidity_crash" -and (
        $trainingRole -eq "extension_only" -or
        ($windowStart -and $windowStart -lt [datetime]"1990-01-01")
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

function Invoke-DatasetSlice {
    param(
        [pscustomobject]$Scenario,
        [string]$SliceOutputDir
    )

    $attempts = New-Object 'System.Collections.Generic.List[object]'
    foreach ($selector in (Resolve-ScenarioDatasetSelectors -Scenario $Scenario)) {
        $args = @(
            "run", "-p", "fc-worker", "--",
            "research", "dataset", "slice-main",
            "--market-scope", $MarketScope,
            "--scenario-id", $Scenario.scenario_id,
            "--from", $Scenario.window_start,
            "--to", $Scenario.window_end,
            "--output-dir", $SliceOutputDir
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
        $isEmptySlice = $commandText -like "*formal dataset slice is empty*"

        if ($exitCode -eq 0) {
            if ($commandText) {
                Write-Host $commandText
            }
            $json = Get-ChildItem -LiteralPath $SliceOutputDir -Filter "*-$($Scenario.scenario_id)-slice-from-$($Scenario.window_start)-to-$($Scenario.window_end).json" |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if (-not $json) {
                throw "Could not locate dataset slice JSON for scenario $($Scenario.scenario_id)"
            }

            return [pscustomobject]@{
                status = "ok"
                selector_reason = $selector.reason
                dataset_key = $selector.dataset_key
                dataset_id = $selector.dataset_id
                dataset_version = $selector.dataset_version
                attempts = $attempts.ToArray()
                slice_json_path = (Resolve-Path -LiteralPath $json.FullName).Path
            }
        }

        $attempts.Add([pscustomobject]@{
                selector_reason = $selector.reason
                dataset_key = $selector.dataset_key
                dataset_id = $selector.dataset_id
                dataset_version = $selector.dataset_version
                exit_code = $exitCode
                empty_slice = $isEmptySlice
                output = $commandText
            })
        if ($isEmptySlice) {
            $selectorLabel = if ($selector.dataset_key) {
                $selector.dataset_key
            } elseif ($selector.dataset_version) {
                "{0}:{1}" -f $selector.dataset_id, $selector.dataset_version
            } else {
                $selector.dataset_id
            }
            Write-Host ("Skipped empty dataset selector {0} for scenario {1}" -f $selectorLabel, $Scenario.scenario_id)
        }
        if (-not $isEmptySlice) {
            throw "formal dataset slice failed for scenario $($Scenario.scenario_id)`n$commandText"
        }
    }

    return [pscustomobject]@{
        status = "missing"
        selector_reason = $null
        dataset_key = $null
        dataset_id = $null
        dataset_version = $null
        attempts = $attempts.ToArray()
        slice_json_path = $null
    }
}

function Build-SliceScenarioSummary {
    param(
        [pscustomobject]$Scenario,
        [pscustomobject]$SliceResult
    )

    if ($SliceResult.status -ne "ok") {
        $attemptedDatasets = @(
            $SliceResult.attempts |
                ForEach-Object {
                    if ($_.dataset_key) {
                        $_.dataset_key
                    } elseif ($_.dataset_version) {
                        "{0}:{1}" -f $_.dataset_id, $_.dataset_version
                    } else {
                        $_.dataset_id
                    }
                }
        )

        return [pscustomobject]@{
            scenario_id = $Scenario.scenario_id
            scenario_name = $Scenario.scenario_name
            workstream = $Scenario.workstream
            scenario_family = $Scenario.scenario_family
            training_role = $Scenario.training_role
            protected_window = $Scenario.protected_window
            suggested_review = $Scenario.suggested_review
            window_start = $Scenario.window_start
            window_end = $Scenario.window_end
            crisis_start = $Scenario.crisis_start
            crisis_end = $Scenario.crisis_end
            slice_status = "missing"
            slice_selector_reason = $null
            attempted_datasets = $attemptedDatasets
            dataset_key = $null
            feature_set_version = $null
            label_version = $null
            row_count = 0
            split_counts = @()
            quality_counts = @()
            regime20_counts = @()
            regime60_counts = @()
            action_phase_counts = @()
            primary_action_level_counts = @()
            avg_coverage_score = $null
            avg_core_feature_coverage = $null
            avg_trigger_feature_coverage = $null
            avg_external_feature_coverage = $null
            positive_label_5d_count = 0
            positive_label_20d_count = 0
            positive_label_60d_count = 0
            prepare_primary_count = 0
            hedge_primary_count = 0
            defend_primary_count = 0
            protected_row_count = 0
            feature_name_count = 0
            feature_names = @()
            slice_json_path = $null
            slice_gap_reason = "No dataset rows were available for this scenario window in the candidate dataset selectors."
        }
    }

    $SliceJsonPath = $SliceResult.slice_json_path
    $slice = [System.IO.File]::ReadAllText($SliceJsonPath) | ConvertFrom-Json
    $attemptedDatasets = @(
        $SliceResult.attempts |
            ForEach-Object {
                if ($_.dataset_key) {
                    $_.dataset_key
                } elseif ($_.dataset_version) {
                    "{0}:{1}" -f $_.dataset_id, $_.dataset_version
                } else {
                    $_.dataset_id
                }
            }
    )
    if ($slice.dataset_key) {
        $attemptedDatasets += $slice.dataset_key
    } elseif ($SliceResult.dataset_key) {
        $attemptedDatasets += $SliceResult.dataset_key
    } elseif ($SliceResult.dataset_id) {
        if ($SliceResult.dataset_version) {
            $attemptedDatasets += ("{0}:{1}" -f $SliceResult.dataset_id, $SliceResult.dataset_version)
        } else {
            $attemptedDatasets += $SliceResult.dataset_id
        }
    }
    $rows = @($slice.rows)
    $label5 = @($rows | Where-Object { [int]$_.label_5d -gt 0 }).Count
    $label20 = @($rows | Where-Object { [int]$_.label_20d -gt 0 }).Count
    $label60 = @($rows | Where-Object { [int]$_.label_60d -gt 0 }).Count
    $preparePrimary = @($rows | Where-Object { [int]$_.prepare_episode_label -gt 0 }).Count
    $hedgePrimary = @($rows | Where-Object { [int]$_.hedge_episode_label -gt 0 }).Count
    $defendPrimary = @($rows | Where-Object { [int]$_.defend_episode_label -gt 0 }).Count
    $protectedRows = @($rows | Where-Object { $_.protected_action_window }).Count

    [pscustomobject]@{
        scenario_id = $Scenario.scenario_id
        scenario_name = $Scenario.scenario_name
        workstream = $Scenario.workstream
        scenario_family = $Scenario.scenario_family
        training_role = $Scenario.training_role
        protected_window = $Scenario.protected_window
        suggested_review = $Scenario.suggested_review
        window_start = $Scenario.window_start
        window_end = $Scenario.window_end
        crisis_start = $Scenario.crisis_start
        crisis_end = $Scenario.crisis_end
        slice_status = "ok"
        slice_selector_reason = $SliceResult.selector_reason
        attempted_datasets = @($attemptedDatasets | Where-Object { $_ } | Sort-Object -Unique)
        dataset_key = $slice.dataset_key
        feature_set_version = $slice.dataset.feature_set_version
        label_version = $slice.dataset.label_version
        row_count = @($rows).Count
        split_counts = @(Group-CountSummary -Rows $rows -Selector { $_.split_name })
        quality_counts = @(Group-CountSummary -Rows $rows -Selector { $_.sample_quality_grade })
        regime20_counts = @(Group-CountSummary -Rows $rows -Selector { $_.regime_20d })
        regime60_counts = @(Group-CountSummary -Rows $rows -Selector { $_.regime_60d })
        action_phase_counts = @(Group-CountSummary -Rows $rows -Selector { $_.action_episode_phase })
        primary_action_level_counts = @(Group-CountSummary -Rows $rows -Selector {
                if ($_.primary_action_level) { $_.primary_action_level } else { "none" }
            })
        avg_coverage_score = Measure-AverageSafe -Rows $rows -PropertyName "coverage_score"
        avg_core_feature_coverage = Measure-AverageSafe -Rows $rows -PropertyName "core_feature_coverage"
        avg_trigger_feature_coverage = Measure-AverageSafe -Rows $rows -PropertyName "trigger_feature_coverage"
        avg_external_feature_coverage = Measure-AverageSafe -Rows $rows -PropertyName "external_feature_coverage"
        positive_label_5d_count = $label5
        positive_label_20d_count = $label20
        positive_label_60d_count = $label60
        prepare_primary_count = $preparePrimary
        hedge_primary_count = $hedgePrimary
        defend_primary_count = $defendPrimary
        protected_row_count = $protectedRows
        feature_name_count = @($slice.feature_names).Count
        feature_names = @($slice.feature_names)
        slice_json_path = $SliceJsonPath
    }
}

function Build-WorkstreamSummary {
    param(
        [string]$Workstream,
        [object[]]$ScenarioSummaries
    )

    [pscustomobject]@{
        workstream = $Workstream
        scenario_count = @($ScenarioSummaries).Count
        scenarios = @($ScenarioSummaries | ForEach-Object { $_.scenario_name })
        covered_scenario_count = @($ScenarioSummaries | Where-Object { $_.slice_status -eq "ok" }).Count
        missing_scenario_count = @($ScenarioSummaries | Where-Object { $_.slice_status -ne "ok" }).Count
        missing_scenarios = @($ScenarioSummaries | Where-Object { $_.slice_status -ne "ok" } | ForEach-Object { $_.scenario_name })
        training_roles = @($ScenarioSummaries | ForEach-Object { $_.training_role } | Sort-Object -Unique)
        scenario_families = @($ScenarioSummaries | ForEach-Object { $_.scenario_family } | Sort-Object -Unique)
        total_rows = (@($ScenarioSummaries | Measure-Object -Property row_count -Sum).Sum)
        total_positive_label_5d_count = (@($ScenarioSummaries | Measure-Object -Property positive_label_5d_count -Sum).Sum)
        total_positive_label_20d_count = (@($ScenarioSummaries | Measure-Object -Property positive_label_20d_count -Sum).Sum)
        total_positive_label_60d_count = (@($ScenarioSummaries | Measure-Object -Property positive_label_60d_count -Sum).Sum)
        total_prepare_primary_count = (@($ScenarioSummaries | Measure-Object -Property prepare_primary_count -Sum).Sum)
        total_hedge_primary_count = (@($ScenarioSummaries | Measure-Object -Property hedge_primary_count -Sum).Sum)
        total_defend_primary_count = (@($ScenarioSummaries | Measure-Object -Property defend_primary_count -Sum).Sum)
        total_protected_row_count = (@($ScenarioSummaries | Measure-Object -Property protected_row_count -Sum).Sum)
        avg_coverage_score = Measure-AverageSafe -Rows $ScenarioSummaries -PropertyName "avg_coverage_score"
        avg_core_feature_coverage = Measure-AverageSafe -Rows $ScenarioSummaries -PropertyName "avg_core_feature_coverage"
        avg_trigger_feature_coverage = Measure-AverageSafe -Rows $ScenarioSummaries -PropertyName "avg_trigger_feature_coverage"
        avg_external_feature_coverage = Measure-AverageSafe -Rows $ScenarioSummaries -PropertyName "avg_external_feature_coverage"
    }
}

$reviewReportPath = Resolve-ReviewReportPath -BaselineRelease $BaselineReleaseId -CandidateRelease $CandidateReleaseId -Mode $HistoryMode -ExplicitPath $ReportPath
$reviewSummary = Resolve-ScenarioAuditRows -ReviewReportPath $reviewReportPath -IncludeWorkstreams $Workstreams
if (-not $reviewSummary.rows -or $reviewSummary.rows.Count -eq 0) {
    throw "No historical audit priorities matched workstreams: $($Workstreams -join ', ')"
}

$resolvedOutputDir = Join-Path $Root $OutputDir
$sliceOutputDir = Join-Path $resolvedOutputDir "slices"
New-Item -ItemType Directory -Force -Path $sliceOutputDir | Out-Null

$scenarioSummaries = foreach ($scenario in $reviewSummary.rows) {
    $sliceResult = Invoke-DatasetSlice -Scenario $scenario -SliceOutputDir $sliceOutputDir
    Build-SliceScenarioSummary -Scenario $scenario -SliceResult $sliceResult
}

$workstreamSummaries = foreach ($group in ($scenarioSummaries | Group-Object workstream)) {
    Build-WorkstreamSummary -Workstream $group.Name -ScenarioSummaries @($group.Group)
}

$reportStem = "{0}-vs-{1}-{2}-workstream-audit" -f `
    (Sanitize-FileComponent $BaselineReleaseId), `
    (Sanitize-FileComponent $CandidateReleaseId), `
    (Sanitize-FileComponent $HistoryMode)
$jsonPath = Join-Path $resolvedOutputDir "$reportStem.json"

$report = [pscustomobject]@{
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    review_report_path = $reviewReportPath
    baseline_release_id = $BaselineReleaseId
    candidate_release_id = $CandidateReleaseId
    history_mode = $reviewSummary.history_mode
    market_scope = $MarketScope
    dataset_key = $DatasetKey
    dataset_id = $DatasetId
    dataset_version = $DatasetVersion
    requested_workstreams = $Workstreams
    workstream_summaries = @($workstreamSummaries)
    scenario_summaries = @($scenarioSummaries)
}

New-Item -ItemType Directory -Force -Path $resolvedOutputDir | Out-Null
$report | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $jsonPath -Encoding utf8

Write-Host "Formal candidate workstream audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  review   : $reviewReportPath"
Write-Host "  output   : $jsonPath"
Write-Host ""

Write-Host "Workstream summary"
$workstreamSummaries |
    Select-Object workstream, scenario_count, total_rows, total_positive_label_5d_count, total_positive_label_20d_count, total_positive_label_60d_count, total_prepare_primary_count, total_hedge_primary_count, total_defend_primary_count, total_protected_row_count, avg_coverage_score |
    Format-Table -AutoSize
Write-Host ""

Write-Host "Scenario summary"
$scenarioSummaries |
    Select-Object scenario_name, workstream, training_role, row_count, positive_label_20d_count, positive_label_60d_count, prepare_primary_count, hedge_primary_count, defend_primary_count, avg_coverage_score |
    Format-Table -AutoSize
