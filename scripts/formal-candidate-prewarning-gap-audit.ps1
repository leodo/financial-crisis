param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string[]]$ScenarioIds = @(
        "us_black_monday_1987",
        "us_ltcm_1998",
        "us_dotcom_unwind_2000",
        "us_funding_stress_2011"
    ),
    [string]$MarketScope = "financial_system",
    [string]$DatasetKey = "",
    [string]$DatasetId = "",
    [string]$DatasetVersion = "",
    [string]$OutputDir = "artifacts/research/prewarning-gap-audit"
)

$ErrorActionPreference = "Stop"
if ($PSVersionTable.PSVersion.Major -ge 7) {
    $PSNativeCommandUseErrorActionPreference = $false
}

$Root = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location -LiteralPath $Root

function Sanitize-FileComponent {
    param([string]$Value)

    ($Value.ToCharArray() | ForEach-Object {
            if ($_ -match '[A-Za-z0-9._-]') { $_ } else { '_' }
        }) -join ''
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

function Invoke-CargoWithFallback {
    param(
        [pscustomobject]$Scenario,
        [string]$Kind,
        [string]$TargetOutputDir
    )

    $attempts = New-Object 'System.Collections.Generic.List[object]'
    foreach ($selector in (Resolve-ScenarioDatasetSelectors -Scenario $Scenario)) {
        if ($Kind -eq "compare") {
            $args = @(
                "run", "-p", "fc-worker", "--",
                "research", "release", "formal-probability-compare",
                "--market-scope", $MarketScope,
                "--baseline-release-id", $BaselineReleaseId,
                "--candidate-release-id", $CandidateReleaseId,
                "--scenario-id", $Scenario.scenario_id,
                "--from", $Scenario.pre_warning_start,
                "--to", $Scenario.crisis_end,
                "--output-dir", $TargetOutputDir
            )
        } elseif ($Kind -eq "slice") {
            $args = @(
                "run", "-p", "fc-worker", "--",
                "research", "dataset", "slice-main",
                "--market-scope", $MarketScope,
                "--scenario-id", $Scenario.scenario_id,
                "--from", $Scenario.pre_warning_start,
                "--to", $Scenario.crisis_end,
                "--output-dir", $TargetOutputDir
            )
        } else {
            throw "Unknown cargo fallback kind: $Kind"
        }

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
        $isEmpty = (
            $commandText -like "*formal dataset slice is empty*" -or
            $commandText -like "*has no rows*" -or
            $commandText -like "*no overlapping rows found*"
        )

        if ($exitCode -eq 0) {
            if ($commandText) {
                Write-Host $commandText
            }

            if ($Kind -eq "compare") {
                $pattern = "{0}-vs-{1}-{2}-{3}-formal-probability-compare-{4}.json" -f `
                    (Sanitize-FileComponent $BaselineReleaseId), `
                    (Sanitize-FileComponent $CandidateReleaseId), `
                    (Sanitize-FileComponent $Scenario.pre_warning_start), `
                    (Sanitize-FileComponent $Scenario.crisis_end), `
                    (Sanitize-FileComponent $Scenario.scenario_id)
            } else {
                $pattern = "*-{0}-slice-from-{1}-to-{2}.json" -f `
                    $Scenario.scenario_id, `
                    $Scenario.pre_warning_start, `
                    $Scenario.crisis_end
            }

            $artifact = Get-ChildItem -LiteralPath $TargetOutputDir -Filter $pattern |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if (-not $artifact) {
                throw "Could not locate $Kind artifact for scenario $($Scenario.scenario_id) with pattern $pattern"
            }

            return [pscustomobject]@{
                status = "ok"
                selector_reason = $selector.reason
                selector_identity = $selector.identity
                dataset_key = $selector.dataset_key
                dataset_id = $selector.dataset_id
                dataset_version = $selector.dataset_version
                artifact_path = (Resolve-Path -LiteralPath $artifact.FullName).Path
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
                empty_window = $isEmpty
                output = $commandText
            })

        if ($isEmpty) {
            Write-Host ("Skipped empty {0} window for {1} using {2}" -f $Kind, $Scenario.scenario_id, $selector.identity)
            continue
        }

        throw "$Kind command failed for scenario $($Scenario.scenario_id)`n$commandText"
    }

    return [pscustomobject]@{
        status = "missing"
        selector_reason = $null
        selector_identity = $null
        dataset_key = $null
        dataset_id = $null
        dataset_version = $null
        artifact_path = $null
        attempts = $attempts.ToArray()
    }
}

$scenarioCatalogPath = Join-Path $Root "config/research_crisis_scenarios.us.json"
$coverageCatalogPath = Join-Path $Root "config/research_scenario_data_coverage.us.json"
$scenarioCatalog = Read-JsonFile -Path $scenarioCatalogPath
$coverageCatalog = Read-JsonFile -Path $coverageCatalogPath

$resolvedScenarios = foreach ($scenarioId in $ScenarioIds) {
    $scenario = @($scenarioCatalog.scenarios | Where-Object { $_.scenario_id -eq $scenarioId }) | Select-Object -First 1
    if (-not $scenario) {
        throw "Scenario catalog does not contain scenario_id=$scenarioId"
    }

    $coverage = @($coverageCatalog.records | Where-Object { $_.scenario_id -eq $scenarioId }) | Select-Object -First 1
    if (-not $coverage) {
        throw "Scenario coverage catalog does not contain scenario_id=$scenarioId"
    }

    [pscustomobject]@{
        scenario_id = $scenario.scenario_id
        label = $scenario.label
        family = $scenario.family
        pre_warning_start = $scenario.pre_warning_start
        crisis_start = $scenario.crisis_start
        acute_start = $scenario.acute_start
        crisis_peak = $scenario.crisis_peak
        crisis_end = $scenario.crisis_end
        training_role = $scenario.training_role
        protected_window = [bool]$scenario.protected_window
        protected_action_levels = @($scenario.protected_action_levels)
        default_horizon_roles = @($scenario.default_horizon_roles)
        coverage_grade = $coverage.coverage_grade
        coverage_role = $coverage.recommended_role
        coverage_pit_mode = $coverage.point_in_time_mode
        free_sources = @($coverage.free_sources)
        blocking_gaps = @($coverage.blocking_gaps)
    }
}

$resolvedOutputDir = Join-Path $Root $OutputDir
$compareOutputDir = Join-Path $resolvedOutputDir "compares"
$sliceOutputDir = Join-Path $resolvedOutputDir "slices"
New-Item -ItemType Directory -Force -Path $compareOutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $sliceOutputDir | Out-Null

$scenarioResults = foreach ($scenario in $resolvedScenarios) {
    Write-Host ("Auditing prewarning gap scenario {0} ({1} -> {2})" -f $scenario.scenario_id, $scenario.pre_warning_start, $scenario.crisis_end)
    $compareResult = Invoke-CargoWithFallback -Scenario $scenario -Kind "compare" -TargetOutputDir $compareOutputDir
    $sliceResult = Invoke-CargoWithFallback -Scenario $scenario -Kind "slice" -TargetOutputDir $sliceOutputDir

    [pscustomobject]@{
        scenario = $scenario
        compare = $compareResult
        slice = $sliceResult
    }
}

$reportStem = "{0}-vs-{1}-prewarning-gap-audit" -f `
    (Sanitize-FileComponent $BaselineReleaseId), `
    (Sanitize-FileComponent $CandidateReleaseId)
$reportPath = Join-Path $resolvedOutputDir "$reportStem.json"
$manifestPath = Join-Path $resolvedOutputDir "$reportStem-manifest.json"

$manifest = [pscustomobject]@{
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    baseline_release_id = $BaselineReleaseId
    candidate_release_id = $CandidateReleaseId
    market_scope = $MarketScope
    scenario_catalog = "config/research_crisis_scenarios.us.json"
    coverage_catalog = "config/research_scenario_data_coverage.us.json"
    scenarios = @($scenarioResults)
}
$manifest | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath $manifestPath -Encoding utf8

$analysisScript = @'
const fs = require("fs");

const manifestPath = process.argv[2];
const reportPath = process.argv[3];

function readJson(path) {
  return JSON.parse(fs.readFileSync(path, "utf8").replace(/^\uFEFF/, ""));
}

function round(value, digits = 6) {
  if (value == null || Number.isNaN(Number(value))) return null;
  return Number(Number(value).toFixed(digits));
}

function average(values) {
  const filtered = values.filter((value) => typeof value === "number" && Number.isFinite(value));
  if (!filtered.length) return null;
  return filtered.reduce((sum, value) => sum + value, 0) / filtered.length;
}

function countBy(rows, selector) {
  const counts = new Map();
  for (const row of rows) {
    const key = selector(row) || "none";
    counts.set(key, (counts.get(key) || 0) + 1);
  }
  return [...counts.entries()]
    .sort(([left], [right]) => String(left).localeCompare(String(right)))
    .map(([key, count]) => `${key}=${count}`);
}

function threshold(thresholds, horizonDays) {
  return thresholds?.find((row) => Number(row.horizon_days) === horizonDays)?.decision_threshold ?? null;
}

function streakStats(rows, predicate) {
  let current = 0;
  let max = 0;
  let segments = 0;
  let firstHitDate = null;
  let lastHitDate = null;
  let currentStart = null;
  let maxStart = null;
  let maxEnd = null;
  let hitCount = 0;

  for (const row of rows) {
    if (predicate(row)) {
      hitCount += 1;
      if (current === 0) {
        segments += 1;
        currentStart = row.as_of_date;
        if (!firstHitDate) firstHitDate = row.as_of_date;
      }
      current += 1;
      lastHitDate = row.as_of_date;
      if (current > max) {
        max = current;
        maxStart = currentStart;
        maxEnd = row.as_of_date;
      }
    } else {
      current = 0;
      currentStart = null;
    }
  }

  return {
    hit_count: hitCount,
    segment_count: segments,
    max_streak: max,
    first_hit_date: firstHitDate,
    last_hit_date: lastHitDate,
    max_streak_start: maxStart,
    max_streak_end: maxEnd
  };
}

function summarizeSlice(slice) {
  if (!slice) {
    return {
      slice_status: "missing",
      dataset_key: null,
      row_count: 0,
      split_counts: [],
      regime20_counts: [],
      regime60_counts: [],
      action_phase_counts: [],
      primary_action_level_counts: [],
      label_5d_count: 0,
      label_20d_count: 0,
      label_60d_count: 0,
      prepare_primary_count: 0,
      hedge_primary_count: 0,
      defend_primary_count: 0,
      protected_row_count: 0,
      avg_coverage_score: null,
      avg_core_feature_coverage: null,
      avg_trigger_feature_coverage: null,
      avg_external_feature_coverage: null,
      feature_name_count: 0
    };
  }

  const rows = slice.rows || [];
  return {
    slice_status: "ok",
    dataset_key: slice.dataset_key ?? null,
    row_count: rows.length,
    split_counts: countBy(rows, (row) => row.split_name),
    regime20_counts: countBy(rows, (row) => row.regime_20d),
    regime60_counts: countBy(rows, (row) => row.regime_60d),
    action_phase_counts: countBy(rows, (row) => row.action_episode_phase),
    primary_action_level_counts: countBy(rows, (row) => row.primary_action_level || "none"),
    label_5d_count: rows.filter((row) => Number(row.label_5d) > 0).length,
    label_20d_count: rows.filter((row) => Number(row.label_20d) > 0).length,
    label_60d_count: rows.filter((row) => Number(row.label_60d) > 0).length,
    prepare_primary_count: rows.filter((row) => Number(row.prepare_episode_label) > 0).length,
    hedge_primary_count: rows.filter((row) => Number(row.hedge_episode_label) > 0).length,
    defend_primary_count: rows.filter((row) => Number(row.defend_episode_label) > 0).length,
    protected_row_count: rows.filter((row) => Boolean(row.protected_action_window)).length,
    avg_coverage_score: round(average(rows.map((row) => row.coverage_score)), 4),
    avg_core_feature_coverage: round(average(rows.map((row) => row.core_feature_coverage)), 4),
    avg_trigger_feature_coverage: round(average(rows.map((row) => row.trigger_feature_coverage)), 4),
    avg_external_feature_coverage: round(average(rows.map((row) => row.external_feature_coverage)), 4),
    feature_name_count: Array.isArray(slice.feature_names) ? slice.feature_names.length : 0
  };
}

function summarizeCompare(compare) {
  if (!compare) {
    return {
      compare_status: "missing",
      compare_dataset_key: null,
      compare_row_count: 0,
      thresholds: {},
      baseline_hit_20d: {},
      candidate_hit_20d: {},
      baseline_hit_60d: {},
      candidate_hit_60d: {},
      candidate_near_threshold_20d_5pp_count: 0,
      candidate_near_threshold_60d_5pp_count: 0,
      candidate_max_p_20d: null,
      candidate_max_p_20d_date: null,
      candidate_max_p_60d: null,
      candidate_max_p_60d_date: null,
      candidate_avg_p_20d: null,
      candidate_avg_p_60d: null,
      baseline_avg_p_20d: null,
      baseline_avg_p_60d: null,
      avg_delta_p_20d: null,
      avg_delta_p_60d: null,
      positive_window_candidate_hit_rate_20d: null,
      hedge_window_candidate_hit_rate_20d: null
    };
  }

  const rows = (compare.rows || []).slice().sort((left, right) =>
    String(left.as_of_date).localeCompare(String(right.as_of_date))
  );
  const baseline20 = threshold(compare.baseline_thresholds, 20);
  const candidate20 = threshold(compare.candidate_thresholds, 20);
  const baseline60 = threshold(compare.baseline_thresholds, 60);
  const candidate60 = threshold(compare.candidate_thresholds, 60);
  const overall = compare.summary?.overall_window ?? {};
  const positive20 = compare.summary?.positive_window_20d ?? {};
  const hedgeWindow = compare.summary?.hedge_window ?? {};

  return {
    compare_status: "ok",
    compare_dataset_key: compare.dataset_key ?? null,
    compare_row_count: compare.row_count ?? rows.length,
    thresholds: {
      baseline_20d: baseline20,
      candidate_20d: candidate20,
      baseline_60d: baseline60,
      candidate_60d: candidate60
    },
    baseline_hit_20d: streakStats(rows, (row) => Boolean(row.baseline_hit_20d)),
    candidate_hit_20d: streakStats(rows, (row) => Boolean(row.candidate_hit_20d)),
    baseline_hit_60d: streakStats(rows, (row) => Boolean(row.baseline_hit_60d)),
    candidate_hit_60d: streakStats(rows, (row) => Boolean(row.candidate_hit_60d)),
    candidate_near_threshold_20d_5pp_count:
      candidate20 == null ? 0 : rows.filter((row) => row.candidate_final_p_20d >= candidate20 - 0.05).length,
    candidate_near_threshold_60d_5pp_count:
      candidate60 == null ? 0 : rows.filter((row) => row.candidate_final_p_60d >= candidate60 - 0.05).length,
    baseline_max_p_20d: round(compare.summary?.baseline_max_p_20d),
    baseline_max_p_20d_date: compare.summary?.baseline_max_p_20d_date ?? null,
    candidate_max_p_20d: round(compare.summary?.candidate_max_p_20d),
    candidate_max_p_20d_date: compare.summary?.candidate_max_p_20d_date ?? null,
    baseline_max_p_60d: round(compare.summary?.baseline_max_p_60d),
    baseline_max_p_60d_date: compare.summary?.baseline_max_p_60d_date ?? null,
    candidate_max_p_60d: round(compare.summary?.candidate_max_p_60d),
    candidate_max_p_60d_date: compare.summary?.candidate_max_p_60d_date ?? null,
    candidate_avg_p_20d: round(average(rows.map((row) => row.candidate_final_p_20d))),
    candidate_avg_p_60d: round(average(rows.map((row) => row.candidate_final_p_60d))),
    baseline_avg_p_20d: round(average(rows.map((row) => row.baseline_final_p_20d))),
    baseline_avg_p_60d: round(average(rows.map((row) => row.baseline_final_p_60d))),
    avg_delta_p_20d: round(overall.avg_delta_p_20d),
    avg_delta_p_60d: round(overall.avg_delta_p_60d),
    positive_window_candidate_hit_rate_20d: round(positive20.candidate_hit_rate_20d),
    hedge_window_candidate_hit_rate_20d: round(hedgeWindow.candidate_hit_rate_20d)
  };
}

function diagnose(sliceSummary, compareSummary, scenario) {
  const reasons = [];
  if (sliceSummary.slice_status !== "ok" || compareSummary.compare_status !== "ok") {
    return {
      gap_class: "dataset_coverage_gap",
      reasons: ["No comparable dataset slice or probability compare artifact was available."],
      next_action: "Fix dataset selector and free-source coverage before modeling."
    };
  }

  if (sliceSummary.row_count === 0 || compareSummary.compare_row_count === 0) {
    return {
      gap_class: "dataset_coverage_gap",
      reasons: ["Scenario window produced zero rows."],
      next_action: "Fix the scenario window or formal dataset coverage first."
    };
  }

  const supportedLabels =
    sliceSummary.label_5d_count + sliceSummary.label_20d_count + sliceSummary.label_60d_count;
  const supportedActions =
    sliceSummary.prepare_primary_count + sliceSummary.hedge_primary_count + sliceSummary.defend_primary_count;
  if (supportedLabels === 0 && supportedActions === 0 && sliceSummary.protected_row_count === 0) {
    reasons.push("No forward labels, episode-native action labels, or protected rows exist in the slice.");
    return {
      gap_class: "label_window_gap",
      reasons,
      next_action: "Fix scenario labels or action episode windows before retraining."
    };
  }

  const candidateHits =
    (compareSummary.candidate_hit_20d?.hit_count || 0) + (compareSummary.candidate_hit_60d?.hit_count || 0);
  const baselineHits =
    (compareSummary.baseline_hit_20d?.hit_count || 0) + (compareSummary.baseline_hit_60d?.hit_count || 0);
  const candidateNear =
    compareSummary.candidate_near_threshold_20d_5pp_count +
    compareSummary.candidate_near_threshold_60d_5pp_count;

  if (candidateHits === 0 && baselineHits === 0) {
    if (candidateNear > 0) {
      reasons.push("No runtime-floor hits, but some rows are within 5pp of a decision threshold.");
      return {
        gap_class: "near_threshold_without_continuity",
        reasons,
        next_action: "Audit threshold and posture mapping because signals are near the floor."
      };
    }
    reasons.push("No baseline or candidate runtime-floor hits in 20d/60d.");
    return {
      gap_class: "no_runtime_floor_signal",
      reasons,
      next_action: "Audit feature separation and family context before changing thresholds."
    };
  }

  if (candidateHits < baselineHits || (compareSummary.avg_delta_p_20d ?? 0) < -0.01) {
    reasons.push("Candidate has fewer hits or lower average 20d probability than baseline.");
    return {
      gap_class: "candidate_margin_erosion",
      reasons,
      next_action: "Treat this as candidate margin erosion before using it as a new baseline."
    };
  }

  const candidateMaxStreak = Math.max(
    compareSummary.candidate_hit_20d?.max_streak || 0,
    compareSummary.candidate_hit_60d?.max_streak || 0
  );
  if (candidateHits > 0 && candidateMaxStreak < 3) {
    reasons.push("Candidate has hits but they do not form a sustained 3-day sequence.");
    return {
      gap_class: "weak_continuity",
      reasons,
      next_action: "Prioritize continuity objective or sustained-hit gate work."
    };
  }

  return {
    gap_class: scenario.protected_window ? "protected_context_signal_present" : "prewarning_signal_present",
    reasons: ["The candidate forms runtime-floor hits without immediate evidence of a dataset or label gap."],
    next_action: "Keep as usable historical evidence and focus next on false positives and cross-scenario generalization."
  };
}

const manifest = readJson(manifestPath);
const scenarioSummaries = manifest.scenarios.map((item) => {
  const scenario = item.scenario;
  const compare = item.compare.status === "ok" ? readJson(item.compare.artifact_path) : null;
  const slice = item.slice.status === "ok" ? readJson(item.slice.artifact_path) : null;
  const sliceSummary = summarizeSlice(slice);
  const compareSummary = summarizeCompare(compare);
  const diagnosis = diagnose(sliceSummary, compareSummary, scenario);

  return {
    scenario_id: scenario.scenario_id,
    scenario_label: scenario.label,
    family: scenario.family,
    training_role: scenario.training_role,
    protected_window: Boolean(scenario.protected_window),
    protected_action_levels: scenario.protected_action_levels || [],
    default_horizon_roles: scenario.default_horizon_roles || [],
    pre_warning_start: scenario.pre_warning_start,
    crisis_start: scenario.crisis_start,
    crisis_end: scenario.crisis_end,
    coverage_grade: scenario.coverage_grade,
    coverage_role: scenario.coverage_role,
    coverage_pit_mode: scenario.coverage_pit_mode,
    free_sources: scenario.free_sources || [],
    blocking_gaps: scenario.blocking_gaps || [],
    compare_selector: {
      status: item.compare.status,
      reason: item.compare.selector_reason,
      identity: item.compare.selector_identity,
      artifact_path: item.compare.artifact_path
    },
    slice_selector: {
      status: item.slice.status,
      reason: item.slice.selector_reason,
      identity: item.slice.selector_identity,
      artifact_path: item.slice.artifact_path
    },
    dataset_evidence: sliceSummary,
    probability_evidence: compareSummary,
    diagnosis
  };
});

const gapCounts = countBy(scenarioSummaries, (row) => row.diagnosis.gap_class);
const report = {
  generated_at: manifest.generated_at,
  baseline_release_id: manifest.baseline_release_id,
  candidate_release_id: manifest.candidate_release_id,
  market_scope: manifest.market_scope,
  scenario_catalog: manifest.scenario_catalog,
  coverage_catalog: manifest.coverage_catalog,
  scenario_count: scenarioSummaries.length,
  gap_counts: gapCounts,
  scenario_summaries: scenarioSummaries
};

fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
'@

$reportJson = $analysisScript | node - $manifestPath $reportPath
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build prewarning gap audit report."
}
$report = $reportJson | ConvertFrom-Json

Write-Host "Formal candidate prewarning gap audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  output   : $reportPath"
Write-Host ""

Write-Host "Gap counts"
$report.gap_counts | Format-Table -AutoSize
Write-Host ""

Write-Host "Scenario diagnosis"
$report.scenario_summaries |
    Select-Object scenario_label,
    @{Name = "gap_class"; Expression = { $_.diagnosis.gap_class } },
    @{Name = "rows"; Expression = { $_.dataset_evidence.row_count } },
    @{Name = "labels_5_20_60"; Expression = { "{0}/{1}/{2}" -f $_.dataset_evidence.label_5d_count, $_.dataset_evidence.label_20d_count, $_.dataset_evidence.label_60d_count } },
    @{Name = "actions_p_h_d"; Expression = { "{0}/{1}/{2}" -f $_.dataset_evidence.prepare_primary_count, $_.dataset_evidence.hedge_primary_count, $_.dataset_evidence.defend_primary_count } },
    @{Name = "candidate_hits_20d"; Expression = { $_.probability_evidence.candidate_hit_20d.hit_count } },
    @{Name = "candidate_max_p20"; Expression = { $_.probability_evidence.candidate_max_p_20d } },
    @{Name = "next_action"; Expression = { $_.diagnosis.next_action } } |
    Format-Table -AutoSize
