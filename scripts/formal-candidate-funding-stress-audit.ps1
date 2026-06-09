param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$ScenarioId = "us_funding_stress_2011",
    [string]$MarketScope = "financial_system",
    [string]$DatasetKey = "",
    [string]$DatasetId = "",
    [string]$DatasetVersion = "",
    [string]$OutputDir = "artifacts/research/funding-stress-audit"
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
function Resolve-DatasetSelectors {
    $selectors = New-Object 'System.Collections.Generic.List[object]'
    if ($DatasetKey) {
        Add-DatasetSelector -Selectors $selectors -SelectorKey $DatasetKey -Reason "explicit dataset key"
        return $selectors.ToArray()
    }
    if ($DatasetId) {
        Add-DatasetSelector -Selectors $selectors -SelectorId $DatasetId -SelectorVersion $DatasetVersion -Reason "explicit dataset id"
        return $selectors.ToArray()
    }
    Add-DatasetSelector -Selectors $selectors -SelectorId "formal_v1_ext_stress_1990_daily" -Reason "2011 protected stress extension coverage"
    Add-DatasetSelector -Selectors $selectors -SelectorId "formal_v1_main_1990_daily" -Reason "main fallback coverage"
    return $selectors.ToArray()
}
function Invoke-CargoWithFallback {
    param(
        [string]$Kind,
        [string]$TargetOutputDir,
        [string]$FromDate,
        [string]$ToDate
    )
    $attempts = New-Object 'System.Collections.Generic.List[object]'
    foreach ($selector in (Resolve-DatasetSelectors)) {
        if ($Kind -eq "compare") {
            $args = @(
                "run", "-p", "fc-worker", "--",
                "research", "release", "formal-probability-compare",
                "--market-scope", $MarketScope,
                "--baseline-release-id", $BaselineReleaseId,
                "--candidate-release-id", $CandidateReleaseId,
                "--scenario-id", $ScenarioId,
                "--from", $FromDate,
                "--to", $ToDate,
                "--output-dir", $TargetOutputDir
            )
        } elseif ($Kind -eq "slice") {
            $args = @(
                "run", "-p", "fc-worker", "--",
                "research", "dataset", "slice-main",
                "--market-scope", $MarketScope,
                "--scenario-id", $ScenarioId,
                "--from", $FromDate,
                "--to", $ToDate,
                "--output-dir", $TargetOutputDir
            )
        } else {
            throw "Unsupported artifact kind: $Kind"
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
                    (Sanitize-FileComponent $FromDate), `
                    (Sanitize-FileComponent $ToDate), `
                    (Sanitize-FileComponent $ScenarioId)
            } else {
                $pattern = "*-$ScenarioId-slice-from-$FromDate-to-$ToDate.json"
            }
            $artifact = Get-ChildItem -LiteralPath $TargetOutputDir -Filter $pattern |
                Sort-Object LastWriteTime -Descending |
                Select-Object -First 1
            if (-not $artifact) {
                throw "Could not locate $Kind artifact for scenario $ScenarioId with pattern $pattern"
            }
            return [pscustomobject]@{
                status = "ok"
                reason = $selector.reason
                identity = $selector.identity
                dataset_key = $selector.dataset_key
                dataset_id = $selector.dataset_id
                dataset_version = $selector.dataset_version
                artifact_path = (Resolve-Path -LiteralPath $artifact.FullName).Path
                attempts = $attempts.ToArray()
            }
        }
        $attempts.Add([pscustomobject]@{
                identity = $selector.identity
                reason = $selector.reason
                exit_code = $exitCode
                empty_window = $isEmpty
                output = $commandText
            })
        if ($isEmpty) {
            Write-Host ("Skipped empty {0} window for {1} using {2}" -f $Kind, $ScenarioId, $selector.identity)
            continue
        }
        throw "$Kind command failed for scenario $ScenarioId`n$commandText"
    }
    $attemptSummary = ($attempts.ToArray() | ConvertTo-Json -Depth 5)
    throw "Could not create $Kind artifact for $ScenarioId with any dataset selector.`n$attemptSummary"
}
$scenarioCatalogPath = Join-Path $Root "config/research_crisis_scenarios.us.json"
$coverageCatalogPath = Join-Path $Root "config/research_scenario_data_coverage.us.json"
$scenarioCatalog = Read-JsonFile -Path $scenarioCatalogPath
$coverageCatalog = Read-JsonFile -Path $coverageCatalogPath
$scenario = @($scenarioCatalog.scenarios | Where-Object { $_.scenario_id -eq $ScenarioId }) | Select-Object -First 1
if (-not $scenario) {
    throw "Scenario catalog does not contain scenario_id=$ScenarioId"
}
$coverage = @($coverageCatalog.records | Where-Object { $_.scenario_id -eq $ScenarioId }) | Select-Object -First 1
if (-not $coverage) {
    throw "Scenario coverage catalog does not contain scenario_id=$ScenarioId"
}
$fromDate = [string]$scenario.pre_warning_start
$toDate = [string]$scenario.crisis_end
$resolvedOutputDir = Join-Path $Root $OutputDir
$compareOutputDir = Join-Path $resolvedOutputDir "compares"
$sliceOutputDir = Join-Path $resolvedOutputDir "slices"
New-Item -ItemType Directory -Force -Path $compareOutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $sliceOutputDir | Out-Null
Write-Host ("Auditing funding-stress scenario {0} ({1} -> {2})" -f $ScenarioId, $fromDate, $toDate)
$compareResult = Invoke-CargoWithFallback -Kind "compare" -TargetOutputDir $compareOutputDir -FromDate $fromDate -ToDate $toDate
$sliceResult = Invoke-CargoWithFallback -Kind "slice" -TargetOutputDir $sliceOutputDir -FromDate $fromDate -ToDate $toDate
$reportStem = "{0}-vs-{1}-{2}-funding-stress-audit" -f `
    (Sanitize-FileComponent $BaselineReleaseId), `
    (Sanitize-FileComponent $CandidateReleaseId), `
    (Sanitize-FileComponent $ScenarioId)
$reportPath = Join-Path $resolvedOutputDir "$reportStem.json"
$manifestPath = Join-Path $resolvedOutputDir "$reportStem-manifest.json"
$manifest = [pscustomobject]@{
    generated_at = (Get-Date).ToUniversalTime().ToString("o")
    baseline_release_id = $BaselineReleaseId
    candidate_release_id = $CandidateReleaseId
    market_scope = $MarketScope
    scenario = [pscustomobject]@{
        scenario_id = $scenario.scenario_id
        label = $scenario.label
        family = $scenario.family
        pre_warning_start = $scenario.pre_warning_start
        crisis_start = $scenario.crisis_start
        acute_start = $scenario.acute_start
        crisis_end = $scenario.crisis_end
        training_role = $scenario.training_role
        protected_window = [bool]$scenario.protected_window
        protected_action_levels = @($scenario.protected_action_levels)
        default_horizon_roles = @($scenario.default_horizon_roles)
    }
    coverage = [pscustomobject]@{
        coverage_grade = $coverage.coverage_grade
        recommended_role = $coverage.recommended_role
        point_in_time_mode = $coverage.point_in_time_mode
        free_sources = @($coverage.free_sources)
        blocking_gaps = @($coverage.blocking_gaps)
    }
    compare = $compareResult
    slice = $sliceResult
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
  const number = Number(value);
  if (!Number.isFinite(number)) return null;
  return Number(number.toFixed(digits));
}
function average(values) {
  const filtered = values.filter((value) => Number.isFinite(Number(value))).map(Number);
  if (filtered.length === 0) return null;
  return round(filtered.reduce((sum, value) => sum + value, 0) / filtered.length);
}
function maxWithDate(rows, key) {
  let best = null;
  for (const row of rows) {
    const value = Number(row[key]);
    if (!Number.isFinite(value)) continue;
    if (!best || value > best.value) {
      best = { value, date: row.as_of_date };
    }
  }
  return best ? { value: round(best.value), date: best.date } : { value: null, date: null };
}
function countsBy(rows, key, missingLabel = "missing") {
  const counts = new Map();
  for (const row of rows) {
    const value = row[key] == null || row[key] === "" ? missingLabel : String(row[key]);
    counts.set(value, (counts.get(value) || 0) + 1);
  }
  return [...counts.entries()]
    .sort(([left], [right]) => left.localeCompare(right))
    .map(([value, count]) => ({ value, count }));
}
function streakStats(rows, predicate) {
  const sorted = [...rows].sort((left, right) => String(left.as_of_date).localeCompare(String(right.as_of_date)));
  let hitCount = 0;
  let segmentCount = 0;
  let current = 0;
  let currentStart = null;
  let maxStreak = 0;
  let maxStart = null;
  let maxEnd = null;
  let firstHit = null;
  let lastHit = null;
  for (const row of sorted) {
    if (predicate(row)) {
      hitCount += 1;
      if (!firstHit) firstHit = row.as_of_date;
      lastHit = row.as_of_date;
      if (current === 0) {
        segmentCount += 1;
        currentStart = row.as_of_date;
      }
      current += 1;
      if (current > maxStreak) {
        maxStreak = current;
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
    segment_count: segmentCount,
    max_streak: maxStreak,
    first_hit_date: firstHit,
    last_hit_date: lastHit,
    max_streak_start: maxStart,
    max_streak_end: maxEnd
  };
}
function nearThresholdStats(rows, key, threshold, band = 0.05) {
  if (threshold == null) {
    return { count: 0, first_date: null, last_date: null, max_value: null, min_gap_to_threshold: null };
  }
  const nearRows = rows
    .map((row) => ({ row, value: Number(row[key]) }))
    .filter(({ value }) => Number.isFinite(value) && value < threshold && threshold - value <= band)
    .sort((left, right) => String(left.row.as_of_date).localeCompare(String(right.row.as_of_date)));
  if (nearRows.length === 0) {
    return { count: 0, first_date: null, last_date: null, max_value: null, min_gap_to_threshold: null };
  }
  const maxValue = Math.max(...nearRows.map(({ value }) => value));
  return {
    count: nearRows.length,
    first_date: nearRows[0].row.as_of_date,
    last_date: nearRows[nearRows.length - 1].row.as_of_date,
    max_value: round(maxValue),
    min_gap_to_threshold: round(threshold - maxValue)
  };
}
function getFeature(row, featureName) {
  const value = row.features?.[featureName] ?? row[featureName];
  const number = Number(value);
  return Number.isFinite(number) ? number : null;
}
function featureMean(rows, featureName) {
  return average(rows.map((row) => getFeature(row, featureName)).filter((value) => value != null));
}
function featureStd(rows, featureName) {
  const values = rows.map((row) => getFeature(row, featureName)).filter((value) => value != null);
  if (values.length < 2) return null;
  const mean = values.reduce((sum, value) => sum + value, 0) / values.length;
  const variance = values.reduce((sum, value) => sum + Math.pow(value - mean, 2), 0) / (values.length - 1);
  return Math.sqrt(variance);
}
function featureSeparation(leftLabel, leftRows, rightLabel, rightRows, featureNames) {
  const allRows = [...leftRows, ...rightRows];
  return featureNames
    .map((feature) => {
      const leftMean = featureMean(leftRows, feature);
      const rightMean = featureMean(rightRows, feature);
      const std = featureStd(allRows, feature);
      if (leftMean == null || rightMean == null) return null;
      const meanDelta = leftMean - rightMean;
      return {
        feature,
        left_group: leftLabel,
        right_group: rightLabel,
        left_mean: round(leftMean, 4),
        right_mean: round(rightMean, 4),
        mean_delta: round(meanDelta, 4),
        standardized_gap: std && std > 0 ? round(meanDelta / std, 4) : null
      };
    })
    .filter(Boolean)
    .sort((left, right) => Math.abs(right.standardized_gap ?? 0) - Math.abs(left.standardized_gap ?? 0));
}
function summarizeGroup(label, rows, thresholds) {
  return {
    label,
    row_count: rows.length,
    avg_baseline_p20d: average(rows.map((row) => row.baseline_final_p_20d)),
    avg_candidate_p20d: average(rows.map((row) => row.candidate_final_p_20d)),
    avg_delta_p20d: average(rows.map((row) => row.delta_final_p_20d)),
    avg_candidate_p60d: average(rows.map((row) => row.candidate_final_p_60d)),
    candidate_max_p20d: maxWithDate(rows, "candidate_final_p_20d"),
    candidate_max_p60d: maxWithDate(rows, "candidate_final_p_60d"),
    candidate_hit_20d: streakStats(rows, (row) => Boolean(row.candidate_hit_20d)),
    candidate_hit_60d: streakStats(rows, (row) => Boolean(row.candidate_hit_60d)),
    near_candidate_20d_5pp: nearThresholdStats(rows, "candidate_final_p_20d", thresholds.candidate_20d, 0.05),
    near_candidate_60d_5pp: nearThresholdStats(rows, "candidate_final_p_60d", thresholds.candidate_60d, 0.05),
    split_counts: countsBy(rows, "split_name"),
    phase_counts: countsBy(rows, "action_episode_phase"),
    action_level_counts: countsBy(rows, "primary_action_level", "none")
  };
}
function featureSnapshot(rows, featureNames) {
  return featureNames
    .filter((feature) => rows.some((row) => getFeature(row, feature) != null))
    .map((feature) => ({ feature, mean: round(featureMean(rows, feature), 4) }));
}
function buildDiagnosis(rows, thresholds, featureNames) {
  const candidateMax20 = maxWithDate(rows, "candidate_final_p_20d");
  const candidateMax60 = maxWithDate(rows, "candidate_final_p_60d");
  const allEvaluation = rows.length > 0 && rows.every((row) => row.split_name === "evaluation");
  const has20Hit = rows.some((row) => Boolean(row.candidate_hit_20d));
  const has60Hit = rows.some((row) => Boolean(row.candidate_hit_60d));
  const hasMixedProxy = featureNames.includes("family_proxy__mixed_systemic");
  const candidateBelowBaseline = average(rows.map((row) => row.delta_final_p_20d)) < 0;
  const reasons = [];
  const nextActions = [];
  if (!has20Hit && !has60Hit) {
    reasons.push("No candidate 20d/60d runtime-floor hits in this funding-stress window.");
  }
  if (candidateMax20.value != null && thresholds.candidate_20d != null && candidateMax20.value < thresholds.candidate_20d) {
    reasons.push(`Candidate max 20d ${round(candidateMax20.value, 4)} remains below runtime hedge floor ${round(thresholds.candidate_20d, 4)}.`);
    nextActions.push("Audit feature separation before lowering runtime floors; the current signal is visible but still below the action threshold.");
  }
  if (candidateMax60.value != null && thresholds.candidate_60d != null && candidateMax60.value < thresholds.candidate_60d) {
    reasons.push(`Candidate max 60d ${round(candidateMax60.value, 4)} remains below runtime prepare floor ${round(thresholds.candidate_60d, 4)}.`);
  }
  if (allEvaluation) {
    reasons.push("All rows are evaluation split, so this exact 2011 slice cannot directly change trained coefficients without topology/split changes or analogous train rows.");
    nextActions.push("Add train-topology repair or analogous trainable mixed-systemic funding-stress rows before expecting retraining to learn this window.");
  }
  if (!hasMixedProxy) {
    reasons.push("family_proxy__mixed_systemic is absent from the active slice features, so mixed-systemic context is not explicitly represented as a family proxy here.");
    nextActions.push("Check whether mixed-systemic family context should be derived into the formal feature set for 2011-like funding stress.");
  }
  if (candidateBelowBaseline) {
    reasons.push("Average candidate 20d probability is below baseline in this window, indicating margin erosion rather than only a high-threshold problem.");
    nextActions.push("Compare tracked 20d feature weights and overlay contributions for funding/liquidity features before promoting this candidate.");
  }
  return {
    primary_class: !has20Hit && !has60Hit ? "no_runtime_floor_signal" : "partial_runtime_signal",
    trainability_class: allEvaluation ? "evaluation_only_window" : "trainable_or_mixed_split_window",
    family_context_class: hasMixedProxy ? "mixed_systemic_proxy_present" : "mixed_systemic_proxy_missing",
    candidate_margin_class: candidateBelowBaseline ? "candidate_margin_erosion" : "candidate_margin_preserved_or_improved",
    reasons,
    next_actions: [...new Set(nextActions)]
  };
}
const manifest = readJson(manifestPath);
const compare = readJson(manifest.compare.artifact_path);
const slice = readJson(manifest.slice.artifact_path);
const compareByDate = new Map(compare.rows.map((row) => [row.as_of_date, row]));
const preserveSliceKeys = [
  "split_name",
  "primary_scenario_id",
  "scenario_family",
  "scenario_training_role",
  "regime_5d",
  "regime_20d",
  "regime_60d",
  "primary_action_level",
  "action_episode_id",
  "action_episode_phase",
  "protected_action_window",
  "features"
];
const rows = slice.rows
  .map((sliceRow) => {
    const merged = { ...sliceRow, ...(compareByDate.get(sliceRow.as_of_date) || {}) };
    for (const key of preserveSliceKeys) {
      if (sliceRow[key] !== undefined && sliceRow[key] !== null && sliceRow[key] !== "") {
        merged[key] = sliceRow[key];
      }
    }
    return merged;
  })
  .filter((row) => row.baseline_final_p_20d != null && row.candidate_final_p_20d != null);
const thresholds = {
  baseline_20d: compare.baseline_thresholds.find((row) => row.horizon_days === 20)?.decision_threshold ?? null,
  candidate_20d: compare.candidate_thresholds.find((row) => row.horizon_days === 20)?.decision_threshold ?? null,
  baseline_60d: compare.baseline_thresholds.find((row) => row.horizon_days === 60)?.decision_threshold ?? null,
  candidate_60d: compare.candidate_thresholds.find((row) => row.horizon_days === 60)?.decision_threshold ?? null
};
const featureNames = [...new Set(slice.feature_names || rows.flatMap((row) => Object.keys(row.features || {})))].sort();
const relevantFeatures = [
  "family_proxy__mixed_systemic",
  "family_proxy__systemic_credit",
  "family_proxy__acute_liquidity",
  "overall_score",
  "structural_score",
  "trigger_score",
  "external_dimension_score",
  "us_nfci_level",
  "us_stlfsi_level",
  "us_baa_10y_spread_level",
  "us_curve_10y2y_level",
  "us_vix_level",
  "us_vix_change_5d",
  "us_usdjpy_level",
  "us_usdjpy_change_20d",
  "us_fed_funds_level",
  "us_unemployment_level",
  "us_housing_starts_level"
];
const availableRelevantFeatures = relevantFeatures.filter((feature) => featureNames.includes(feature));
const missingRelevantFeatures = relevantFeatures.filter((feature) => !featureNames.includes(feature));
const preparePrimaryRows = rows.filter((row) => row.action_episode_phase === "primary" && row.primary_action_level === "prepare");
const hedgePrimaryRows = rows.filter((row) => row.action_episode_phase === "primary" && row.primary_action_level === "hedge");
const primaryRows = rows.filter((row) => row.action_episode_phase === "primary");
const lateRows = rows.filter((row) => row.action_episode_phase === "late_validation");
const positive20Rows = rows.filter((row) => row.regime_20d === "positive_window");
const prewarning20Rows = rows.filter((row) => row.regime_20d === "pre_warning_buffer");
const normal20Rows = rows.filter((row) => row.regime_20d === "normal");
const candidateTop20Rows = [...rows]
  .sort((left, right) => Number(right.candidate_final_p_20d) - Number(left.candidate_final_p_20d))
  .slice(0, Math.min(10, rows.length));
const candidateRestRows = rows.filter((row) => !candidateTop20Rows.some((topRow) => topRow.as_of_date === row.as_of_date));
const report = {
  generated_at: new Date().toISOString(),
  compare_path: manifest.compare.artifact_path,
  slice_path: manifest.slice.artifact_path,
  baseline_release_id: compare.baseline_release_id,
  candidate_release_id: compare.candidate_release_id,
  market_scope: compare.market_scope,
  scenario: manifest.scenario,
  coverage: manifest.coverage,
  dataset_key: compare.dataset_key,
  from_date: compare.from_date,
  to_date: compare.to_date,
  row_count: rows.length,
  thresholds,
  dataset_evidence: {
    split_counts: countsBy(rows, "split_name"),
    regime_20d_counts: countsBy(rows, "regime_20d"),
    regime_60d_counts: countsBy(rows, "regime_60d"),
    action_phase_counts: countsBy(rows, "action_episode_phase"),
    action_level_counts: countsBy(rows, "primary_action_level", "none"),
    protected_row_count: rows.filter((row) => Boolean(row.protected_action_window)).length,
    label_20d_count: rows.filter((row) => Number(row.label_20d) === 1).length,
    label_60d_count: rows.filter((row) => Number(row.label_60d) === 1).length,
    prepare_episode_count: rows.filter((row) => Number(row.prepare_episode_label) === 1).length,
    hedge_episode_count: rows.filter((row) => Number(row.hedge_episode_label) === 1).length,
    avg_coverage_score: average(rows.map((row) => row.coverage_score)),
    feature_name_count: featureNames.length,
    available_relevant_features: availableRelevantFeatures,
    missing_relevant_features: missingRelevantFeatures
  },
  probability_evidence: {
    full_window: summarizeGroup("full_window", rows, thresholds),
    primary_phase: summarizeGroup("primary_phase", primaryRows, thresholds),
    prepare_primary: summarizeGroup("prepare_primary", preparePrimaryRows, thresholds),
    hedge_primary: summarizeGroup("hedge_primary", hedgePrimaryRows, thresholds),
    late_validation: summarizeGroup("late_validation", lateRows, thresholds),
    positive_window_20d: summarizeGroup("positive_window_20d", positive20Rows, thresholds),
    pre_warning_buffer_20d: summarizeGroup("pre_warning_buffer_20d", prewarning20Rows, thresholds),
    normal_20d: summarizeGroup("normal_20d", normal20Rows, thresholds)
  },
  feature_context: {
    full_window_means: featureSnapshot(rows, availableRelevantFeatures),
    candidate_top_20d_dates: candidateTop20Rows.map((row) => ({
      as_of_date: row.as_of_date,
      candidate_p20d: round(row.candidate_final_p_20d),
      baseline_p20d: round(row.baseline_final_p_20d),
      action_phase: row.action_episode_phase,
      action_level: row.primary_action_level,
      regime_20d: row.regime_20d
    })),
    separation: {
      hedge_vs_prepare_primary: featureSeparation("hedge_primary", hedgePrimaryRows, "prepare_primary", preparePrimaryRows, availableRelevantFeatures).slice(0, 12),
      positive_window_vs_normal_20d: featureSeparation("positive_window_20d", positive20Rows, "normal_20d", normal20Rows, availableRelevantFeatures).slice(0, 12),
      prewarning_buffer_vs_normal_20d: featureSeparation("pre_warning_buffer_20d", prewarning20Rows, "normal_20d", normal20Rows, availableRelevantFeatures).slice(0, 12),
      late_validation_vs_primary: featureSeparation("late_validation", lateRows, "primary", primaryRows, availableRelevantFeatures).slice(0, 12),
      candidate_top20_vs_rest: featureSeparation("candidate_top20", candidateTop20Rows, "rest", candidateRestRows, availableRelevantFeatures).slice(0, 12)
    }
  }
};
report.diagnosis = buildDiagnosis(rows, thresholds, featureNames);
fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
'@
$reportJson = $analysisScript | node - $manifestPath $reportPath
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build funding-stress audit report."
}
$report = $reportJson | ConvertFrom-Json
Write-Host "Formal candidate funding-stress audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  scenario : $ScenarioId"
Write-Host "  compare  : $($report.compare_path)"
Write-Host "  slice    : $($report.slice_path)"
Write-Host "  output   : $reportPath"
Write-Host ""
Write-Host "Threshold gap"
@(
    [pscustomobject]@{
        horizon = "20d"
        candidate_max = $report.probability_evidence.full_window.candidate_max_p20d.value
        candidate_floor = $report.thresholds.candidate_20d
        near_5pp = $report.probability_evidence.full_window.near_candidate_20d_5pp.count
        hits = $report.probability_evidence.full_window.candidate_hit_20d.hit_count
    }
    [pscustomobject]@{
        horizon = "60d"
        candidate_max = $report.probability_evidence.full_window.candidate_max_p60d.value
        candidate_floor = $report.thresholds.candidate_60d
        near_5pp = $report.probability_evidence.full_window.near_candidate_60d_5pp.count
        hits = $report.probability_evidence.full_window.candidate_hit_60d.hit_count
    }
) | Format-Table -AutoSize
Write-Host ""
Write-Host "Dataset evidence"
@(
    [pscustomobject]@{ metric = "rows"; value = $report.row_count }
    [pscustomobject]@{ metric = "splits"; value = (($report.dataset_evidence.split_counts | ForEach-Object { "{0}={1}" -f $_.value, $_.count }) -join ", ") }
    [pscustomobject]@{ metric = "action levels"; value = (($report.dataset_evidence.action_level_counts | ForEach-Object { "{0}={1}" -f $_.value, $_.count }) -join ", ") }
    [pscustomobject]@{ metric = "features"; value = $report.dataset_evidence.feature_name_count }
    [pscustomobject]@{ metric = "missing relevant"; value = (($report.dataset_evidence.missing_relevant_features | Select-Object -First 6) -join ", ") }
) | Format-Table -AutoSize
Write-Host ""
Write-Host "Diagnosis"
$report.diagnosis | ConvertTo-Json -Depth 4
