param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$ScenarioId = "us_rate_shock_2022",
    [string]$FromDate = "2021-10-05",
    [string]$ToDate = "2022-10-31",
    [string]$MarketScope = "financial_system",
    [string]$DatasetKey = "",
    [string]$DatasetId = "",
    [string]$DatasetVersion = "",
    [string]$OutputDir = "artifacts/research/rate-shock-audit"
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

function Invoke-CargoChecked {
    param([string[]]$CommandArgs)

    & cargo @CommandArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo command failed: cargo $($CommandArgs -join ' ')"
    }
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

$resolvedOutputDir = Join-Path $Root $OutputDir
$compareOutputDir = Join-Path $resolvedOutputDir "formal-probability-compares"
$sliceOutputDir = Join-Path $resolvedOutputDir "formal-dataset-slices"
New-Item -ItemType Directory -Force -Path $compareOutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $sliceOutputDir | Out-Null

$compareArgs = @(
    "run", "-p", "fc-worker", "--",
    "research", "release", "formal-probability-compare",
    "--market-scope", $MarketScope,
    "--baseline-release-id", $BaselineReleaseId,
    "--candidate-release-id", $CandidateReleaseId,
    "--scenario-id", $ScenarioId,
    "--from", $FromDate,
    "--to", $ToDate,
    "--output-dir", $compareOutputDir
)
if ($DatasetKey) {
    $compareArgs += @("--dataset-key", $DatasetKey)
} elseif ($DatasetId) {
    $compareArgs += @("--dataset-id", $DatasetId)
    if ($DatasetVersion) {
        $compareArgs += @("--dataset-version", $DatasetVersion)
    }
}
Invoke-CargoChecked -CommandArgs $compareArgs

$sliceArgs = @(
    "run", "-p", "fc-worker", "--",
    "research", "dataset", "slice-main",
    "--market-scope", $MarketScope,
    "--scenario-id", $ScenarioId,
    "--from", $FromDate,
    "--to", $ToDate,
    "--output-dir", $sliceOutputDir
)
if ($DatasetKey) {
    $sliceArgs += @("--dataset-key", $DatasetKey)
} elseif ($DatasetId) {
    $sliceArgs += @("--dataset-id", $DatasetId)
    if ($DatasetVersion) {
        $sliceArgs += @("--dataset-version", $DatasetVersion)
    }
}
Invoke-CargoChecked -CommandArgs $sliceArgs

$comparePattern = "{0}-vs-{1}-{2}-{3}-formal-probability-compare-{4}.json" -f `
    (Sanitize-FileComponent $BaselineReleaseId), `
    (Sanitize-FileComponent $CandidateReleaseId), `
    (Sanitize-FileComponent $FromDate), `
    (Sanitize-FileComponent $ToDate), `
    (Sanitize-FileComponent $ScenarioId)
$slicePattern = "*-$ScenarioId-slice-from-$FromDate-to-$ToDate.json"

$comparePath = Find-LatestArtifact -Directory $compareOutputDir -Pattern $comparePattern
$slicePath = Find-LatestArtifact -Directory $sliceOutputDir -Pattern $slicePattern

$reportStem = "{0}-vs-{1}-{2}-{3}-{4}-rate-shock-audit" -f `
    (Sanitize-FileComponent $BaselineReleaseId), `
    (Sanitize-FileComponent $CandidateReleaseId), `
    (Sanitize-FileComponent $ScenarioId), `
    (Sanitize-FileComponent $FromDate), `
    (Sanitize-FileComponent $ToDate)
$reportPath = Join-Path $resolvedOutputDir "$reportStem.json"

$analysisScript = @'
const fs = require("fs");

const comparePath = process.argv[2];
const slicePath = process.argv[3];
const reportPath = process.argv[4];

const compare = JSON.parse(fs.readFileSync(comparePath, "utf8"));
const slice = JSON.parse(fs.readFileSync(slicePath, "utf8"));

const compareByDate = new Map(compare.rows.map((row) => [row.as_of_date, row]));
const rows = slice.rows
  .map((sliceRow) => ({ ...sliceRow, ...(compareByDate.get(sliceRow.as_of_date) || {}) }))
  .filter((row) => row.baseline_final_p_20d != null && row.candidate_final_p_20d != null);

const baseline20 = compare.baseline_thresholds.find((row) => row.horizon_days === 20)?.decision_threshold ?? null;
const candidate20 = compare.candidate_thresholds.find((row) => row.horizon_days === 20)?.decision_threshold ?? null;
const baseline60 = compare.baseline_thresholds.find((row) => row.horizon_days === 60)?.decision_threshold ?? null;
const candidate60 = compare.candidate_thresholds.find((row) => row.horizon_days === 60)?.decision_threshold ?? null;

function round(value, digits = 6) {
  if (value == null || Number.isNaN(value)) return null;
  return Number(value.toFixed(digits));
}

function average(values) {
  if (!values.length) return null;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function featureMean(rows, featureName) {
  const values = rows
    .map((row) => row.features?.[featureName])
    .filter((value) => typeof value === "number" && Number.isFinite(value));
  return average(values);
}

function featureStd(rows, featureName) {
  const values = rows
    .map((row) => row.features?.[featureName])
    .filter((value) => typeof value === "number" && Number.isFinite(value));
  if (values.length < 2) return null;
  const mean = average(values);
  const variance = values.reduce((sum, value) => sum + (value - mean) ** 2, 0) / values.length;
  const std = Math.sqrt(variance);
  return std > 0 ? std : null;
}

function streakStats(groupRows, predicate) {
  let current = 0;
  let max = 0;
  let segments = 0;
  let firstHitDate = null;
  let lastHitDate = null;
  let currentStart = null;
  let maxStart = null;
  let maxEnd = null;
  for (const row of groupRows) {
    if (predicate(row)) {
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
    hit_count: groupRows.filter(predicate).length,
    segment_count: segments,
    max_streak: max,
    first_hit_date: firstHitDate,
    last_hit_date: lastHitDate,
    max_streak_start: maxStart,
    max_streak_end: maxEnd
  };
}

function summarizeGroup(label, groupRows) {
  const baseline20Stats = streakStats(groupRows, (row) => Boolean(row.baseline_hit_20d));
  const candidate20Stats = streakStats(groupRows, (row) => Boolean(row.candidate_hit_20d));
  const baseline60Stats = streakStats(groupRows, (row) => Boolean(row.baseline_hit_60d));
  const candidate60Stats = streakStats(groupRows, (row) => Boolean(row.candidate_hit_60d));
  const baselineP20 = groupRows.map((row) => row.baseline_final_p_20d);
  const candidateP20 = groupRows.map((row) => row.candidate_final_p_20d);
  const baselineP60 = groupRows.map((row) => row.baseline_final_p_60d);
  const candidateP60 = groupRows.map((row) => row.candidate_final_p_60d);

  const maxBaseline20Row = groupRows.reduce((best, row) =>
    !best || row.baseline_final_p_20d > best.baseline_final_p_20d ? row : best, null);
  const maxCandidate20Row = groupRows.reduce((best, row) =>
    !best || row.candidate_final_p_20d > best.candidate_final_p_20d ? row : best, null);
  const maxBaseline60Row = groupRows.reduce((best, row) =>
    !best || row.baseline_final_p_60d > best.baseline_final_p_60d ? row : best, null);
  const maxCandidate60Row = groupRows.reduce((best, row) =>
    !best || row.candidate_final_p_60d > best.candidate_final_p_60d ? row : best, null);

  const near20_5pp_baseline = groupRows.filter((row) => baseline20 != null && row.baseline_final_p_20d >= baseline20 - 0.05).length;
  const near20_5pp_candidate = groupRows.filter((row) => candidate20 != null && row.candidate_final_p_20d >= candidate20 - 0.05).length;
  const near60_5pp_baseline = groupRows.filter((row) => baseline60 != null && row.baseline_final_p_60d >= baseline60 - 0.05).length;
  const near60_5pp_candidate = groupRows.filter((row) => candidate60 != null && row.candidate_final_p_60d >= candidate60 - 0.05).length;

  return {
    label,
    row_count: groupRows.length,
    baseline_avg_p_20d: round(average(baselineP20)),
    candidate_avg_p_20d: round(average(candidateP20)),
    avg_delta_p_20d: round(average(groupRows.map((row) => row.candidate_final_p_20d - row.baseline_final_p_20d))),
    baseline_avg_gap_to_threshold_20d: round(average(groupRows.map((row) => baseline20 == null ? null : row.baseline_final_p_20d - baseline20).filter((value) => value != null))),
    candidate_avg_gap_to_threshold_20d: round(average(groupRows.map((row) => candidate20 == null ? null : row.candidate_final_p_20d - candidate20).filter((value) => value != null))),
    baseline_avg_p_60d: round(average(baselineP60)),
    candidate_avg_p_60d: round(average(candidateP60)),
    avg_delta_p_60d: round(average(groupRows.map((row) => row.candidate_final_p_60d - row.baseline_final_p_60d))),
    baseline_avg_gap_to_threshold_60d: round(average(groupRows.map((row) => baseline60 == null ? null : row.baseline_final_p_60d - baseline60).filter((value) => value != null))),
    candidate_avg_gap_to_threshold_60d: round(average(groupRows.map((row) => candidate60 == null ? null : row.candidate_final_p_60d - candidate60).filter((value) => value != null))),
    baseline_hit_rate_20d: round(groupRows.length ? baseline20Stats.hit_count / groupRows.length : 0),
    candidate_hit_rate_20d: round(groupRows.length ? candidate20Stats.hit_count / groupRows.length : 0),
    baseline_hit_rate_60d: round(groupRows.length ? baseline60Stats.hit_count / groupRows.length : 0),
    candidate_hit_rate_60d: round(groupRows.length ? candidate60Stats.hit_count / groupRows.length : 0),
    baseline_hit_20d: baseline20Stats,
    candidate_hit_20d: candidate20Stats,
    baseline_hit_60d: baseline60Stats,
    candidate_hit_60d: candidate60Stats,
    baseline_near_threshold_20d_within_5pp_count: near20_5pp_baseline,
    candidate_near_threshold_20d_within_5pp_count: near20_5pp_candidate,
    baseline_near_threshold_60d_within_5pp_count: near60_5pp_baseline,
    candidate_near_threshold_60d_within_5pp_count: near60_5pp_candidate,
    baseline_max_p_20d: maxBaseline20Row ? round(maxBaseline20Row.baseline_final_p_20d) : null,
    baseline_max_p_20d_date: maxBaseline20Row?.as_of_date ?? null,
    candidate_max_p_20d: maxCandidate20Row ? round(maxCandidate20Row.candidate_final_p_20d) : null,
    candidate_max_p_20d_date: maxCandidate20Row?.as_of_date ?? null,
    baseline_max_p_60d: maxBaseline60Row ? round(maxBaseline60Row.baseline_final_p_60d) : null,
    baseline_max_p_60d_date: maxBaseline60Row?.as_of_date ?? null,
    candidate_max_p_60d: maxCandidate60Row ? round(maxCandidate60Row.candidate_final_p_60d) : null,
    candidate_max_p_60d_date: maxCandidate60Row?.as_of_date ?? null
  };
}

function distinctGroups(rows, selector) {
  const seen = new Set();
  const output = [];
  for (const row of rows) {
    const key = selector(row);
    if (!seen.has(key)) {
      seen.add(key);
      output.push(key);
    }
  }
  return output;
}

function buildFeatureGapSummary(rowsA, rowsB, labelA, labelB, featureNames, allRows) {
  if (!rowsA.length || !rowsB.length) return [];
  return featureNames
    .map((featureName) => {
      const meanA = featureMean(rowsA, featureName);
      const meanB = featureMean(rowsB, featureName);
      const std = featureStd(allRows, featureName);
      if (meanA == null || meanB == null || std == null) return null;
      return {
        feature: featureName,
        left_group: labelA,
        right_group: labelB,
        left_mean: round(meanA, 4),
        right_mean: round(meanB, 4),
        mean_delta: round(meanA - meanB, 4),
        standardized_gap: round((meanA - meanB) / std, 4)
      };
    })
    .filter(Boolean)
    .sort((left, right) => Math.abs(right.standardized_gap) - Math.abs(left.standardized_gap))
    .slice(0, 8);
}

const sortedRows = rows.slice().sort((left, right) => left.as_of_date.localeCompare(right.as_of_date));
const phaseGroups = distinctGroups(sortedRows, (row) => row.action_episode_phase || "outside");
const actionLevelGroups = distinctGroups(sortedRows, (row) => row.primary_action_level || "none");
const splitGroups = distinctGroups(sortedRows, (row) => row.split_name || "unknown").map((splitName) => ({
  split_name: splitName,
  row_count: sortedRows.filter((row) => (row.split_name || "unknown") === splitName).length
}));

const phaseSummaries = phaseGroups.map((phase) =>
  summarizeGroup(phase, sortedRows.filter((row) => (row.action_episode_phase || "outside") === phase))
);
const actionLevelSummaries = actionLevelGroups.map((level) =>
  summarizeGroup(level, sortedRows.filter((row) => (row.primary_action_level || "none") === level))
);

const preparePrimaryRows = sortedRows.filter((row) => row.action_episode_phase === "primary" && row.primary_action_level === "prepare");
const hedgePrimaryRows = sortedRows.filter((row) => row.action_episode_phase === "primary" && row.primary_action_level === "hedge");
const lateValidationRows = sortedRows.filter((row) => row.action_episode_phase === "late_validation");
const primaryRows = sortedRows.filter((row) => row.action_episode_phase === "primary");

const featureNames = Array.isArray(slice.feature_names) ? slice.feature_names : [];
const featureGapSummary = {
  hedge_vs_prepare_primary: buildFeatureGapSummary(
    hedgePrimaryRows,
    preparePrimaryRows,
    "hedge_primary",
    "prepare_primary",
    featureNames,
    sortedRows
  ),
  late_validation_vs_primary: buildFeatureGapSummary(
    lateValidationRows,
    primaryRows,
    "late_validation",
    "primary",
    featureNames,
    sortedRows
  )
};

const report = {
  generated_at: new Date().toISOString(),
  compare_path: comparePath,
  slice_path: slicePath,
  baseline_release_id: compare.baseline_release_id,
  candidate_release_id: compare.candidate_release_id,
  dataset_key: compare.dataset_key,
  scenario_id: compare.scenario_id,
  from_date: compare.from_date,
  to_date: compare.to_date,
  thresholds: {
    baseline_20d: baseline20,
    candidate_20d: candidate20,
    baseline_60d: baseline60,
    candidate_60d: candidate60
  },
  compare_summary: compare.summary,
  split_counts: splitGroups,
  phase_summaries: phaseSummaries,
  action_level_summaries: actionLevelSummaries,
  continuity_focus: {
    prepare_primary: summarizeGroup("prepare_primary", preparePrimaryRows),
    hedge_primary: summarizeGroup("hedge_primary", hedgePrimaryRows),
    primary_phase: summarizeGroup("primary_phase", primaryRows),
    late_validation: summarizeGroup("late_validation", lateValidationRows)
  },
  feature_separation: featureGapSummary
};

fs.writeFileSync(reportPath, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
'@

$reportJson = $analysisScript | node - $comparePath $slicePath $reportPath
if ($LASTEXITCODE -ne 0) {
    throw "Failed to build rate-shock audit report."
}
$report = $reportJson | ConvertFrom-Json

Write-Host "Formal candidate rate-shock audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  scenario : $ScenarioId"
Write-Host "  compare  : $comparePath"
Write-Host "  slice    : $slicePath"
Write-Host "  output   : $reportPath"
Write-Host ""

Write-Host "Thresholds"
@(
    [pscustomobject]@{
        horizon = "20d"
        baseline = $report.thresholds.baseline_20d
        candidate = $report.thresholds.candidate_20d
    }
    [pscustomobject]@{
        horizon = "60d"
        baseline = $report.thresholds.baseline_60d
        candidate = $report.thresholds.candidate_60d
    }
) | Format-Table -AutoSize
Write-Host ""

Write-Host "Split counts"
$report.split_counts | Format-Table -AutoSize
Write-Host ""

Write-Host "Continuity by phase"
$report.phase_summaries |
    Select-Object label, row_count,
    baseline_avg_p_20d, candidate_avg_p_20d, avg_delta_p_20d,
    baseline_hit_rate_20d, candidate_hit_rate_20d,
    @{Name = "baseline_20d_max_streak"; Expression = { $_.baseline_hit_20d.max_streak } },
    @{Name = "candidate_20d_max_streak"; Expression = { $_.candidate_hit_20d.max_streak } },
    baseline_avg_p_60d, candidate_avg_p_60d,
    baseline_hit_rate_60d, candidate_hit_rate_60d |
    Format-Table -AutoSize
Write-Host ""

Write-Host "Continuity by action level"
$report.action_level_summaries |
    Select-Object label, row_count,
    baseline_avg_p_20d, candidate_avg_p_20d, avg_delta_p_20d,
    @{Name = "base_seg_20d"; Expression = { $_.baseline_hit_20d.segment_count } },
    @{Name = "cand_seg_20d"; Expression = { $_.candidate_hit_20d.segment_count } },
    @{Name = "base_streak_20d"; Expression = { $_.baseline_hit_20d.max_streak } },
    @{Name = "cand_streak_20d"; Expression = { $_.candidate_hit_20d.max_streak } },
    @{Name = "base_near_20d"; Expression = { $_.baseline_near_threshold_20d_within_5pp_count } },
    @{Name = "cand_near_20d"; Expression = { $_.candidate_near_threshold_20d_within_5pp_count } } |
    Format-Table -AutoSize
Write-Host ""

Write-Host "Top feature separation: hedge primary vs prepare primary"
$report.feature_separation.hedge_vs_prepare_primary |
    Select-Object feature, left_mean, right_mean, mean_delta, standardized_gap |
    Format-Table -AutoSize
Write-Host ""

Write-Host "Top feature separation: late validation vs primary"
$report.feature_separation.late_validation_vs_primary |
    Select-Object feature, left_mean, right_mean, mean_delta, standardized_gap |
    Format-Table -AutoSize
