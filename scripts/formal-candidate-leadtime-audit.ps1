param(
    [Parameter(Mandatory = $true)]
    [string]$BaselineReleaseId,
    [Parameter(Mandatory = $true)]
    [string]$CandidateReleaseId,
    [string]$HistoryMode = "strict_rebuild",
    [string]$ReportPath = "",
    [string]$OutputPath = ""
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

function Load-LeadtimeAuditSummary {
    param([string]$ReviewReportPath)

    $parser = @'
const fs = require("fs");
const reportPath = process.argv[2];
const review = JSON.parse(fs.readFileSync(reportPath, "utf8"));
const comparison = review.comparison || {};
const runtimeRows = (comparison.runtime_separation_summary || []).map((row) => ({
  horizon_days: row.horizon_days,
  baseline_diagnosis: row.baseline_diagnosis ?? null,
  candidate_diagnosis: row.candidate_diagnosis ?? null,
  baseline_threshold: row.baseline_threshold ?? null,
  candidate_threshold: row.candidate_threshold ?? null,
  baseline_early_warning_regime: row.baseline_early_warning_regime ?? null,
  candidate_early_warning_regime: row.candidate_early_warning_regime ?? null,
  baseline_early_warning_avg_probability: row.baseline_early_warning_avg_probability ?? null,
  candidate_early_warning_avg_probability: row.candidate_early_warning_avg_probability ?? null,
  baseline_normal_avg_probability: row.baseline_normal_avg_probability ?? null,
  candidate_normal_avg_probability: row.candidate_normal_avg_probability ?? null,
  baseline_early_warning_gap_vs_normal: row.baseline_early_warning_gap_vs_normal ?? null,
  candidate_early_warning_gap_vs_normal: row.candidate_early_warning_gap_vs_normal ?? null,
  baseline_floor_gap: row.baseline_floor_gap ?? null,
  candidate_floor_gap: row.candidate_floor_gap ?? null,
  baseline_threshold_hit_rate: row.baseline_threshold_hit_rate ?? null,
  candidate_threshold_hit_rate: row.candidate_threshold_hit_rate ?? null
}));

const metricRows = [
  "timely_warning_rate",
  "strict_actionable_point_count",
  "runtime_floor_hit_count",
  "actionable_precision",
  "longest_false_positive_episode_days",
  "current_p_5d",
  "current_p_20d",
  "current_p_60d"
]
  .filter((metric) => comparison[metric] != null)
  .map((metric) => ({
    metric,
    baseline: comparison[metric].baseline ?? null,
    candidate: comparison[metric].candidate ?? null,
    delta: comparison[metric].delta ?? null
  }));

const leadtimeGapRows = (comparison.backtest_scenarios || [])
  .filter((scenario) =>
    (scenario.baseline_lead_time_days != null && scenario.baseline_actionable_lead_time_days == null) ||
    (scenario.candidate_lead_time_days != null && scenario.candidate_actionable_lead_time_days == null)
  )
  .map((scenario) => ({
    scenario_id: scenario.scenario_id,
    name: scenario.name,
    outcome: scenario.outcome,
    signal_source: scenario.signal_source,
    baseline_lead_time_days: scenario.baseline_lead_time_days ?? null,
    candidate_lead_time_days: scenario.candidate_lead_time_days ?? null,
    baseline_actionable_lead_time_days: scenario.baseline_actionable_lead_time_days ?? null,
    candidate_actionable_lead_time_days: scenario.candidate_actionable_lead_time_days ?? null,
    actionable_delta_days: scenario.actionable_delta_days ?? null
  }));

const focusRows = (review.scenario_focus || []).map((scenario) => {
  const dominantBlocks = scenario.dominant_runtime_blocks || {};
  const dominantContinuity = scenario.dominant_runtime_continuity_facets || {};
  const firstInteresting =
    (scenario.interesting_points || []).find((point) =>
      point.baseline_runtime_actionable_block_category != null ||
      point.candidate_runtime_actionable_block_category != null
    ) || null;
  return {
    scenario_id: scenario.scenario_id,
    name: scenario.name,
    outcome: scenario.outcome,
    baseline_primary_failure_mode: scenario.baseline_primary_failure_mode ?? null,
    candidate_primary_failure_mode: scenario.candidate_primary_failure_mode ?? null,
    baseline_actionable_point_count: scenario.baseline_actionable_point_count ?? null,
    candidate_actionable_point_count: scenario.candidate_actionable_point_count ?? null,
    baseline_runtime_floor_hit_point_count: scenario.baseline_runtime_floor_hit_point_count ?? null,
    candidate_runtime_floor_hit_point_count: scenario.candidate_runtime_floor_hit_point_count ?? null,
    baseline_dominant_runtime_block: (dominantBlocks.baseline_categories || []).join(" + "),
    baseline_dominant_runtime_block_count: dominantBlocks.baseline_count ?? 0,
    candidate_dominant_runtime_block: (dominantBlocks.candidate_categories || []).join(" + "),
    candidate_dominant_runtime_block_count: dominantBlocks.candidate_count ?? 0,
    baseline_dominant_continuity_facet:
      (dominantContinuity.baseline_categories || []).join(" + "),
    baseline_dominant_continuity_facet_count: dominantContinuity.baseline_count ?? 0,
    candidate_dominant_continuity_facet:
      (dominantContinuity.candidate_categories || []).join(" + "),
    candidate_dominant_continuity_facet_count: dominantContinuity.candidate_count ?? 0,
    baseline_first_runtime_floor_hit_without_l3_reason:
      scenario.baseline_first_runtime_floor_hit_without_l3_reason ?? null,
    candidate_first_runtime_floor_hit_without_l3_reason:
      scenario.candidate_first_runtime_floor_hit_without_l3_reason ?? null,
    first_block_date: firstInteresting?.as_of_date ?? null,
    first_baseline_block_category:
      firstInteresting?.baseline_runtime_actionable_block_category ?? null,
    first_candidate_block_category:
      firstInteresting?.candidate_runtime_actionable_block_category ?? null,
    first_baseline_block_reason:
      firstInteresting?.baseline_runtime_actionable_block_reason ?? null,
    first_candidate_block_reason:
      firstInteresting?.candidate_runtime_actionable_block_reason ?? null
  };
});

const blockMixRows = [];
const continuityFacetRows = [];
for (const scenario of review.scenario_focus || []) {
  for (const block of scenario.runtime_block_counts || []) {
    blockMixRows.push({
      scenario_id: scenario.scenario_id,
      name: scenario.name,
      category: block.category,
      baseline_count: block.baseline_count ?? 0,
      candidate_count: block.candidate_count ?? 0,
      delta: block.delta ?? 0
    });
  }
  for (const facet of scenario.runtime_continuity_facet_counts || []) {
    continuityFacetRows.push({
      scenario_id: scenario.scenario_id,
      name: scenario.name,
      category: facet.category,
      baseline_count: facet.baseline_count ?? 0,
      candidate_count: facet.candidate_count ?? 0,
      delta: facet.delta ?? 0
    });
  }
}

const workstreamRows = (review.historical_audit_workstreams || []).map((row) => ({
  workstream: row.workstream,
  scenario_count: row.scenario_count ?? 0,
  protected_count: row.protected_count ?? 0,
  scenarios: (row.scenarios || []).join(" | "),
  scenario_families: (row.scenario_families || []).join(" | "),
  training_roles: (row.training_roles || []).join(" | "),
  baseline_gate_gap_profiles: (row.baseline_gate_gap_profiles || []).join(" | "),
  candidate_gate_gap_profiles: (row.candidate_gate_gap_profiles || []).join(" | "),
  baseline_gate_gap_points: (row.gate_gap_point_counts || [])
    .filter((count) => (count.baseline_count ?? 0) > 0)
    .map((count) => `${count.category}=${count.baseline_count}`)
    .join(" | "),
  candidate_gate_gap_points: (row.gate_gap_point_counts || [])
    .filter((count) => (count.candidate_count ?? 0) > 0)
    .map((count) => `${count.category}=${count.candidate_count}`)
    .join(" | "),
  suggested_review: row.suggested_review ?? null
}));

const attributionRows = (review.historical_audit_attribution || []).map((row) => ({
  workstream: row.workstream,
  attribution: row.attribution,
  scenario_count: row.scenario_count ?? 0,
  protected_count: row.protected_count ?? 0,
  explanation: row.explanation ?? null
}));

const actionRows = (review.historical_audit_actions || []).map((row) => ({
  workstream: row.workstream,
  attribution: row.attribution,
  action_type: row.action_type,
  scenario_count: row.scenario_count ?? 0,
  protected_count: row.protected_count ?? 0,
  recommendation: row.recommendation ?? null
}));

console.log(
  JSON.stringify(
    {
      report_path: reportPath,
      reviewed_at: review.reviewed_at ?? null,
      market_scope: review.market_scope ?? null,
      history_mode: review.history_mode ?? null,
      baseline_release: review.baseline_release ?? null,
      candidate_release: review.candidate_release ?? null,
      comparison,
      metric_rows: metricRows,
      runtime_rows: runtimeRows,
      leadtime_gap_rows: leadtimeGapRows,
      focus_rows: focusRows,
      block_mix_rows: blockMixRows,
      continuity_facet_rows: continuityFacetRows,
      workstream_rows: workstreamRows,
      attribution_rows: attributionRows,
      action_rows: actionRows
    },
    null,
    2
  )
);
'@

    $json = $parser | node - $ReviewReportPath
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to parse release-review artifact: $ReviewReportPath"
    }

    $json | ConvertFrom-Json
}

function Add-Takeaway {
    param(
        [System.Collections.Generic.List[string]]$List,
        [string]$Text
    )

    if ($Text -and -not $List.Contains($Text)) {
        $List.Add($Text)
    }
}

function Get-LeadtimeTakeaways {
    param($Summary)

    $takeaways = New-Object System.Collections.Generic.List[string]
    $comparison = $Summary.comparison
    $runtimeRows = @($Summary.runtime_rows)
    $focusRows = @($Summary.focus_rows)
    $leadtimeGaps = @($Summary.leadtime_gap_rows)
    $blockRows = @($Summary.block_mix_rows)
    $continuityFacetRows = @($Summary.continuity_facet_rows)
    $workstreamRows = @($Summary.workstream_rows)

    if (
        $comparison.timely_warning_rate -and
        $comparison.strict_actionable_point_count -and
        $comparison.runtime_floor_hit_count -and
        [double]$comparison.timely_warning_rate.delta -eq 0.0 -and
        [double]$comparison.strict_actionable_point_count.delta -gt 0.0 -and
        [double]$comparison.runtime_floor_hit_count.delta -gt 0.0
    ) {
        Add-Takeaway -List $takeaways -Text 'candidate produced more strict actionable points and runtime floor hits, but timely warning rate did not improve; the blocker has shifted from signal presence to sustained L3 lead time.'
    }

    $candidate60 = $runtimeRows | Where-Object { $_.horizon_days -eq 60 } | Select-Object -First 1
    if ($candidate60) {
        if ($candidate60.candidate_diagnosis -eq "usable_early_warning_separation") {
            Add-Takeaway -List $takeaways -Text '60d already reached usable early-warning separation on the candidate, but that still did not translate into higher timely warning; the next fix target is the conversion chain from runtime floor to strict/actionable.'
        } elseif ($candidate60.candidate_diagnosis -eq "separated_but_below_runtime_floor") {
            Add-Takeaway -List $takeaways -Text '60d still sits at separated_but_below_runtime_floor; training separation exists, but runtime floor or threshold policy is still suppressing long-horizon actionability.'
        }
    }

    if ($comparison.longest_false_positive_episode_days -and [double]$comparison.longest_false_positive_episode_days.delta -gt 0.0) {
        Add-Takeaway -List $takeaways -Text 'candidate extended the longest pure false-positive episode, so lead-time recovery must be watched together with false-positive spillover.'
    }

    foreach ($row in $leadtimeGaps) {
        if ($row.candidate_lead_time_days -ne $null -and $row.candidate_actionable_lead_time_days -eq $null) {
            $message = '{0} still has only L2 lead time={1}d and no L3 actionable conversion; this scenario should be reviewed first for posture, gate, and sustained-hit continuity.' -f $row.name, $row.candidate_lead_time_days
            Add-Takeaway -List $takeaways -Text $message
        }
    }

    $candidateReviewGapRows = $blockRows | Where-Object {
        $_.category -eq "review_gate_gap" -and [int]$_.candidate_count -gt 0
    }
    if (($candidateReviewGapRows | Measure-Object).Count -gt 0) {
        $scenarioNames = ($candidateReviewGapRows | Select-Object -ExpandProperty name -Unique) -join ", "
        $message = 'review_gate_gap is still blocking scenarios such as {0}; strict review remains harder than runtime floor, so more runtime hits alone will not fix the problem.' -f $scenarioNames
        Add-Takeaway -List $takeaways -Text $message
    }

    $candidatePostureRows = $blockRows | Where-Object {
        $_.category -eq "posture_bucket_normal" -and [int]$_.candidate_count -gt 0
    }
    if (($candidatePostureRows | Measure-Object).Count -gt 0) {
        $scenarioNames = ($candidatePostureRows | Select-Object -ExpandProperty name -Unique) -join ", "
        $message = 'posture_bucket_normal still dominates scenarios such as {0}; the real missing piece is posture continuity, not another isolated probability-threshold relaxation.' -f $scenarioNames
        Add-Takeaway -List $takeaways -Text $message
    }

    $candidateP20OnlyRows = $continuityFacetRows | Where-Object {
        $_.category -eq "gate_gap:p20d_only" -and [int]$_.candidate_count -gt 0
    }
    if (($candidateP20OnlyRows | Measure-Object).Count -gt 0) {
        $scenarioNames = ($candidateP20OnlyRows | Select-Object -ExpandProperty name -Unique) -join ", "
        $message = 'gate_gap:p20d_only still appears in scenarios such as {0}; the next strict-gate audit should verify whether p20d review thresholds are the main blocker.' -f $scenarioNames
        Add-Takeaway -List $takeaways -Text $message
    }

    $candidateBothGapRows = $continuityFacetRows | Where-Object {
        $_.category -eq "gate_gap:p20d_and_p60d" -and [int]$_.candidate_count -gt 0
    }
    if (($candidateBothGapRows | Measure-Object).Count -gt 0) {
        $scenarioNames = ($candidateBothGapRows | Select-Object -ExpandProperty name -Unique) -join ", "
        $message = 'gate_gap:p20d_and_p60d remains active in scenarios such as {0}; both medium- and long-horizon strict gates are still suppressing L3 conversion.' -f $scenarioNames
        Add-Takeaway -List $takeaways -Text $message
    }

    $strictWorkstream = $workstreamRows |
        Where-Object { $_.workstream -eq "strict_review_vs_runtime_mapping" } |
        Select-Object -First 1
    if ($strictWorkstream) {
        $baselinePoints = [string]$strictWorkstream.baseline_gate_gap_points
        $candidatePoints = [string]$strictWorkstream.candidate_gate_gap_points
        if ($baselinePoints -or $candidatePoints) {
            $message = 'historical workstream strict_review_vs_runtime_mapping point counts: baseline [{0}] candidate [{1}].' -f `
                ($(if ($baselinePoints) { $baselinePoints } else { "—" })), `
                ($(if ($candidatePoints) { $candidatePoints } else { "—" }))
            Add-Takeaway -List $takeaways -Text $message
        }
        $candidateP20Matches = [regex]::Matches($candidatePoints, 'p20d_only=(\d+)')
        $candidateP60Matches = [regex]::Matches($candidatePoints, 'p60d_only=(\d+)')
        $candidateBothMatches = [regex]::Matches($candidatePoints, 'p20d_and_p60d=(\d+)')
        $candidateP20Count = 0
        $candidateP60Count = 0
        $candidateBothCount = 0
        if ($candidateP20Matches.Count -gt 0) {
            $candidateP20Count = [int]$candidateP20Matches[0].Groups[1].Value
        }
        if ($candidateP60Matches.Count -gt 0) {
            $candidateP60Count = [int]$candidateP60Matches[0].Groups[1].Value
        }
        if ($candidateBothMatches.Count -gt 0) {
            $candidateBothCount = [int]$candidateBothMatches[0].Groups[1].Value
        }
        if ($candidateP20Count -gt ($candidateP60Count + $candidateBothCount)) {
            Add-Takeaway -List $takeaways -Text 'workstream-level gate-gap point counts now lean toward p20d_only as the dominant strict mapping blocker.'
        } elseif ($candidateP60Count -gt ($candidateP20Count + $candidateBothCount)) {
            Add-Takeaway -List $takeaways -Text 'workstream-level gate-gap point counts now lean toward p60d_only as the dominant strict mapping blocker.'
        } elseif (($candidateP20Count + $candidateP60Count + $candidateBothCount) -gt 0) {
            Add-Takeaway -List $takeaways -Text 'workstream-level gate-gap point counts remain mixed, so strict mapping still competes with continuity as the next fix target.'
        }
    }

    foreach ($focus in $focusRows) {
        if ($focus.candidate_primary_failure_mode) {
            $message = '{0} has candidate primary failure mode {1}.' -f $focus.name, $focus.candidate_primary_failure_mode
            Add-Takeaway -List $takeaways -Text $message
        }
    }

    if ($takeaways.Count -eq 0) {
        Add-Takeaway -List $takeaways -Text 'the current release-review artifact did not expose a new lead-time blocker; the next step should fall back to day-level scenario slices.'
    }

    $takeaways
}

$reviewReportPath = Resolve-ReviewReportPath `
    -BaselineRelease $BaselineReleaseId `
    -CandidateRelease $CandidateReleaseId `
    -Mode $HistoryMode `
    -ExplicitPath $ReportPath
$summary = Load-LeadtimeAuditSummary -ReviewReportPath $reviewReportPath
$takeaways = Get-LeadtimeTakeaways -Summary $summary
$summary | Add-Member -NotePropertyName takeaways -NotePropertyValue $takeaways

Write-Host "Formal candidate lead-time audit"
Write-Host "  baseline : $BaselineReleaseId"
Write-Host "  candidate: $CandidateReleaseId"
Write-Host "  mode     : $HistoryMode"
Write-Host "  report   : $reviewReportPath"
Write-Host ""

Write-Host "Lead-time summary metrics"
$summary.metric_rows | Format-Table -AutoSize
Write-Host ""

Write-Host "Runtime separation summary"
$runtimeDisplayRows = @($summary.runtime_rows) | ForEach-Object {
    [pscustomobject]@{
        horizon = $_.horizon_days
        baseline_diag = $_.baseline_diagnosis
        candidate_diag = $_.candidate_diagnosis
        base_thr = $_.baseline_threshold
        cand_thr = $_.candidate_threshold
        base_gap = $_.baseline_floor_gap
        cand_gap = $_.candidate_floor_gap
        base_hit = $_.baseline_threshold_hit_rate
        cand_hit = $_.candidate_threshold_hit_rate
    }
}
$runtimeDisplayRows | Format-Table -AutoSize
Write-Host ""

Write-Host "Scenarios with L2 lead time but no L3 actionable"
if (($summary.leadtime_gap_rows | Measure-Object).Count -gt 0) {
    $summary.leadtime_gap_rows |
        Format-Table name, outcome, baseline_lead_time_days, candidate_lead_time_days, baseline_actionable_lead_time_days, candidate_actionable_lead_time_days -AutoSize
} else {
    Write-Host "  (none)"
}
Write-Host ""

Write-Host "Focus scenario failure modes"
if (($summary.focus_rows | Measure-Object).Count -gt 0) {
    $focusDisplayRows = @($summary.focus_rows) | ForEach-Object {
        [pscustomobject]@{
            name = $_.name
            outcome = $_.outcome
            baseline_failure = $_.baseline_primary_failure_mode
            candidate_failure = $_.candidate_primary_failure_mode
            baseline_block = $_.baseline_dominant_runtime_block
            candidate_block = $_.candidate_dominant_runtime_block
            baseline_facet = $_.baseline_dominant_continuity_facet
            candidate_facet = $_.candidate_dominant_continuity_facet
        }
    }
    $focusDisplayRows | Format-Table -AutoSize
} else {
    Write-Host "  (none)"
}
Write-Host ""

Write-Host "Focus scenario runtime block mix"
if (($summary.block_mix_rows | Measure-Object).Count -gt 0) {
    $summary.block_mix_rows |
        Format-Table name, category, baseline_count, candidate_count, delta -AutoSize
} else {
    Write-Host "  (none)"
}
Write-Host ""

Write-Host "Focus scenario continuity facets"
if (($summary.continuity_facet_rows | Measure-Object).Count -gt 0) {
    $summary.continuity_facet_rows |
        Format-Table name, category, baseline_count, candidate_count, delta -AutoSize
} else {
    Write-Host "  (none)"
}
Write-Host ""

Write-Host "Historical audit workstreams"
if (($summary.workstream_rows | Measure-Object).Count -gt 0) {
    $summary.workstream_rows |
        Format-Table workstream, scenario_count, protected_count, baseline_gate_gap_profiles, candidate_gate_gap_profiles, baseline_gate_gap_points, candidate_gate_gap_points, training_roles -AutoSize
} else {
    Write-Host "  (none)"
}
Write-Host ""

Write-Host "Historical audit actions"
if (($summary.action_rows | Measure-Object).Count -gt 0) {
    $summary.action_rows |
        Format-Table workstream, attribution, action_type, scenario_count, protected_count -AutoSize
} else {
    Write-Host "  (none)"
}
Write-Host ""

Write-Host "Key takeaways"
foreach ($takeaway in $takeaways) {
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
