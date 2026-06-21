#!/usr/bin/env node
import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

import {
  asArray,
  countFrom,
  horizonRow,
  joinText,
  metric,
  releaseId,
  repoRelativePath,
  requireValue,
  resolveReviewReportPath,
  scenarioName,
  timestampForFile,
  writeJson,
  readJson
} from "./release-review-audit-common.mjs";

const DEFAULT_HISTORY_MODE = "strict_rebuild";
const DEFAULT_OUTPUT_DIR = "artifacts/research/leadtime-audit";

const options = parseArgs(process.argv.slice(2));

function parseArgs(args) {
  const parsed = {
    baselineReleaseId: null,
    candidateReleaseId: null,
    historyMode: DEFAULT_HISTORY_MODE,
    reportPath: "",
    outputPath: "",
    outputDir: DEFAULT_OUTPUT_DIR,
    root: process.cwd()
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--baseline-release-id") {
      parsed.baselineReleaseId = requireValue(args, ++index, arg);
    } else if (arg === "--candidate-release-id") {
      parsed.candidateReleaseId = requireValue(args, ++index, arg);
    } else if (arg === "--history-mode") {
      parsed.historyMode = requireValue(args, ++index, arg);
    } else if (arg === "--report-path") {
      parsed.reportPath = requireValue(args, ++index, arg);
    } else if (arg === "--output") {
      parsed.outputPath = requireValue(args, ++index, arg);
    } else if (arg === "--output-dir") {
      parsed.outputDir = requireValue(args, ++index, arg);
    } else if (arg === "--root") {
      parsed.root = requireValue(args, ++index, arg);
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown lead-time audit option: ${arg}`);
    }
  }

  if (!parsed.baselineReleaseId) {
    throw new Error("--baseline-release-id is required");
  }
  if (!parsed.candidateReleaseId) {
    throw new Error("--candidate-release-id is required");
  }
  parsed.root = resolve(parsed.root);
  return parsed;
}

function printHelp() {
  console.log(`Usage: node scripts/formal-candidate-leadtime-audit.mjs --baseline-release-id ID --candidate-release-id ID [options]

Exports a release-review-derived lead-time audit JSON artifact.

Options:
  --history-mode MODE   Release review history mode, default ${DEFAULT_HISTORY_MODE}
  --report-path PATH    Explicit release-review JSON path
  --output PATH         Explicit output JSON path
  --output-dir DIR      Output directory, default ${DEFAULT_OUTPUT_DIR}
  --root DIR            Repository/deployment root, default current directory
`);
}

function metricRows(comparison) {
  return [
    "timely_warning_rate",
    "strict_actionable_point_count",
    "runtime_floor_hit_count",
    "actionable_precision",
    "longest_false_positive_episode_days",
    "current_p_5d",
    "current_p_20d",
    "current_p_60d"
  ]
    .map((name) => {
      const row = metric(comparison, name);
      return row
        ? {
            metric: name,
            baseline: row.baseline ?? null,
            candidate: row.candidate ?? null,
            delta: row.delta ?? null
          }
        : null;
    })
    .filter(Boolean);
}

function runtimeRows(comparison) {
  return asArray(comparison?.runtime_separation_summary).map((row) => ({
    horizon_days: countFrom(row.horizon_days),
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
}

function leadtimeGapRows(comparison) {
  return asArray(comparison?.backtest_scenarios)
    .filter(
      (scenario) =>
        (scenario.baseline_lead_time_days != null &&
          scenario.baseline_actionable_lead_time_days == null) ||
        (scenario.candidate_lead_time_days != null &&
          scenario.candidate_actionable_lead_time_days == null)
    )
    .map((scenario) => ({
      scenario_id: scenario.scenario_id ?? "",
      name: scenarioName(scenario),
      outcome: scenario.outcome ?? null,
      signal_source: scenario.signal_source ?? null,
      baseline_lead_time_days: scenario.baseline_lead_time_days ?? null,
      candidate_lead_time_days: scenario.candidate_lead_time_days ?? null,
      baseline_actionable_lead_time_days: scenario.baseline_actionable_lead_time_days ?? null,
      candidate_actionable_lead_time_days: scenario.candidate_actionable_lead_time_days ?? null,
      actionable_delta_days: scenario.actionable_delta_days ?? null
    }));
}

function focusRows(review) {
  return asArray(review.scenario_focus).map((scenario) => {
    const dominantBlocks = scenario.dominant_runtime_blocks ?? {};
    const dominantContinuity = scenario.dominant_runtime_continuity_facets ?? {};
    const firstInteresting =
      asArray(scenario.interesting_points).find(
        (point) =>
          point.baseline_runtime_actionable_block_category != null ||
          point.candidate_runtime_actionable_block_category != null
      ) ?? null;
    return {
      scenario_id: scenario.scenario_id ?? "",
      name: scenarioName(scenario),
      outcome: scenario.outcome ?? null,
      baseline_primary_failure_mode: scenario.baseline_primary_failure_mode ?? null,
      candidate_primary_failure_mode: scenario.candidate_primary_failure_mode ?? null,
      baseline_actionable_point_count: scenario.baseline_actionable_point_count ?? null,
      candidate_actionable_point_count: scenario.candidate_actionable_point_count ?? null,
      baseline_runtime_floor_hit_point_count:
        scenario.baseline_runtime_floor_hit_point_count ?? null,
      candidate_runtime_floor_hit_point_count:
        scenario.candidate_runtime_floor_hit_point_count ?? null,
      baseline_dominant_runtime_block: joinText(dominantBlocks.baseline_categories),
      baseline_dominant_runtime_block_count: dominantBlocks.baseline_count ?? 0,
      candidate_dominant_runtime_block: joinText(dominantBlocks.candidate_categories),
      candidate_dominant_runtime_block_count: dominantBlocks.candidate_count ?? 0,
      baseline_dominant_continuity_facet: joinText(dominantContinuity.baseline_categories),
      baseline_dominant_continuity_facet_count: dominantContinuity.baseline_count ?? 0,
      candidate_dominant_continuity_facet: joinText(dominantContinuity.candidate_categories),
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
}

function countRows(review, fieldName) {
  const rows = [];
  for (const scenario of asArray(review.scenario_focus)) {
    for (const item of asArray(scenario[fieldName])) {
      rows.push({
        scenario_id: scenario.scenario_id ?? "",
        name: scenarioName(scenario),
        category: item.category ?? "",
        baseline_count: countFrom(item.baseline_count),
        candidate_count: countFrom(item.candidate_count),
        delta: countFrom(item.delta)
      });
    }
  }
  return rows;
}

function workstreamRows(review) {
  return asArray(review.historical_audit_workstreams).map((row) => ({
    workstream: row.workstream ?? "",
    scenario_count: countFrom(row.scenario_count),
    protected_count: countFrom(row.protected_count),
    scenarios: joinText(row.scenarios),
    scenario_families: joinText(row.scenario_families),
    training_roles: joinText(row.training_roles),
    baseline_gate_gap_profiles: joinText(row.baseline_gate_gap_profiles),
    candidate_gate_gap_profiles: joinText(row.candidate_gate_gap_profiles),
    baseline_gate_gap_points: asArray(row.gate_gap_point_counts)
      .filter((count) => countFrom(count.baseline_count) > 0)
      .map((count) => `${count.category}=${count.baseline_count}`)
      .join(" | "),
    candidate_gate_gap_points: asArray(row.gate_gap_point_counts)
      .filter((count) => countFrom(count.candidate_count) > 0)
      .map((count) => `${count.category}=${count.candidate_count}`)
      .join(" | "),
    suggested_review: row.suggested_review ?? null
  }));
}

function attributionRows(review) {
  return asArray(review.historical_audit_attribution).map((row) => ({
    workstream: row.workstream ?? "",
    attribution: row.attribution ?? "",
    scenario_count: countFrom(row.scenario_count),
    protected_count: countFrom(row.protected_count),
    explanation: row.explanation ?? null
  }));
}

function actionRows(review) {
  return asArray(review.historical_audit_actions).map((row) => ({
    workstream: row.workstream ?? "",
    attribution: row.attribution ?? "",
    action_type: row.action_type ?? row.action ?? "",
    scenario_count: countFrom(row.scenario_count),
    protected_count: countFrom(row.protected_count),
    recommendation: row.recommendation ?? null
  }));
}

function addTakeaway(takeaways, text) {
  if (text && !takeaways.includes(text)) {
    takeaways.push(text);
  }
}

function leadtimeTakeaways(summary) {
  const takeaways = [];
  const comparison = summary.comparison ?? {};
  const timely = metric(comparison, "timely_warning_rate");
  const strict = metric(comparison, "strict_actionable_point_count");
  const runtime = metric(comparison, "runtime_floor_hit_count");
  const longestFalsePositive = metric(comparison, "longest_false_positive_episode_days");
  if (
    timely?.delta === 0 &&
    Number(strict?.delta ?? 0) > 0 &&
    Number(runtime?.delta ?? 0) > 0
  ) {
    addTakeaway(
      takeaways,
      "candidate produced more strict actionable points and runtime floor hits, but timely warning rate did not improve; the blocker has shifted from signal presence to sustained L3 lead time."
    );
  }

  const candidate60 = horizonRow(summary.runtime_rows, 60);
  if (candidate60?.candidate_diagnosis === "usable_early_warning_separation") {
    addTakeaway(
      takeaways,
      "60d already reached usable early-warning separation on the candidate, but that still did not translate into higher timely warning; the next fix target is the conversion chain from runtime floor to strict/actionable."
    );
  } else if (candidate60?.candidate_diagnosis === "separated_but_below_runtime_floor") {
    addTakeaway(
      takeaways,
      "60d still sits at separated_but_below_runtime_floor; training separation exists, but runtime floor or threshold policy is still suppressing long-horizon actionability."
    );
  }

  if (Number(longestFalsePositive?.delta ?? 0) > 0) {
    addTakeaway(
      takeaways,
      "candidate extended the longest pure false-positive episode, so lead-time recovery must be watched together with false-positive spillover."
    );
  }

  for (const row of summary.leadtime_gap_rows) {
    if (row.candidate_lead_time_days != null && row.candidate_actionable_lead_time_days == null) {
      addTakeaway(
        takeaways,
        `${row.name} still has only L2 lead time=${row.candidate_lead_time_days}d and no L3 actionable conversion; this scenario should be reviewed first for posture, gate, and sustained-hit continuity.`
      );
    }
  }

  const reviewGateRows = summary.block_mix_rows.filter(
    (row) => row.category === "review_gate_gap" && row.candidate_count > 0
  );
  if (reviewGateRows.length > 0) {
    addTakeaway(
      takeaways,
      `review_gate_gap is still blocking scenarios such as ${[
        ...new Set(reviewGateRows.map((row) => row.name))
      ].join(", ")}; strict review remains harder than runtime floor, so more runtime hits alone will not fix the problem.`
    );
  }

  const postureRows = summary.block_mix_rows.filter(
    (row) => row.category === "posture_bucket_normal" && row.candidate_count > 0
  );
  if (postureRows.length > 0) {
    addTakeaway(
      takeaways,
      `posture_bucket_normal still dominates scenarios such as ${[
        ...new Set(postureRows.map((row) => row.name))
      ].join(", ")}; the real missing piece is posture continuity, not another isolated probability-threshold relaxation.`
    );
  }

  const p20OnlyRows = summary.continuity_facet_rows.filter(
    (row) => row.category === "gate_gap:p20d_only" && row.candidate_count > 0
  );
  if (p20OnlyRows.length > 0) {
    addTakeaway(
      takeaways,
      `gate_gap:p20d_only still appears in scenarios such as ${[
        ...new Set(p20OnlyRows.map((row) => row.name))
      ].join(", ")}; the next strict-gate audit should verify whether p20d review thresholds are the main blocker.`
    );
  }

  const strictWorkstream = summary.workstream_rows.find(
    (row) => row.workstream === "strict_review_vs_runtime_mapping"
  );
  if (strictWorkstream) {
    addTakeaway(
      takeaways,
      `historical workstream strict_review_vs_runtime_mapping point counts: baseline [${
        strictWorkstream.baseline_gate_gap_points || "-"
      }] candidate [${strictWorkstream.candidate_gate_gap_points || "-"}].`
    );
  }

  for (const focus of summary.focus_rows) {
    if (focus.candidate_primary_failure_mode) {
      addTakeaway(
        takeaways,
        `${focus.name} has candidate primary failure mode ${focus.candidate_primary_failure_mode}.`
      );
    }
  }

  if (takeaways.length === 0) {
    addTakeaway(
      takeaways,
      "the current release-review artifact did not expose a new lead-time blocker; the next step should fall back to day-level scenario slices."
    );
  }
  return takeaways;
}

async function outputPath() {
  if (options.outputPath) {
    return resolve(options.root, options.outputPath);
  }
  const dir = resolve(options.root, options.outputDir);
  await mkdir(dir, { recursive: true });
  return resolve(
    dir,
    `${timestampForFile()}-${options.baselineReleaseId}-vs-${options.candidateReleaseId}-${options.historyMode}-leadtime-audit.json`
  );
}

async function main() {
  const reviewPath = await resolveReviewReportPath(options);
  const review = await readJson(reviewPath);
  const comparison = review.comparison ?? {};
  const summary = {
    comparison,
    runtime_rows: runtimeRows(comparison),
    leadtime_gap_rows: leadtimeGapRows(comparison),
    focus_rows: focusRows(review),
    block_mix_rows: countRows(review, "runtime_block_counts"),
    continuity_facet_rows: countRows(review, "runtime_continuity_facet_counts"),
    workstream_rows: workstreamRows(review),
    attribution_rows: attributionRows(review),
    action_rows: actionRows(review)
  };
  const artifact = {
    generated_at: new Date().toISOString(),
    release_review_artifact: repoRelativePath(options.root, reviewPath),
    baseline_release_id: options.baselineReleaseId,
    candidate_release_id: options.candidateReleaseId,
    market_scope: review.market_scope ?? "financial_system",
    history_mode: review.history_mode ?? options.historyMode,
    reviewed_at: review.reviewed_at ?? null,
    comparison,
    metric_rows: metricRows(comparison),
    ...summary,
    takeaways: leadtimeTakeaways(summary),
    baseline_release: releaseId(review.baseline_release),
    candidate_release: releaseId(review.candidate_release)
  };
  const path = await outputPath();
  await writeJson(path, artifact);
  console.log("Formal candidate lead-time audit exported.");
  console.log(`  release review : ${reviewPath}`);
  console.log(`  output         : ${path}`);
  console.log(`  takeaways      : ${artifact.takeaways.length}`);
}

main().catch((error) => {
  console.error(`Formal candidate lead-time audit failed: ${error?.message ?? error}`);
  process.exit(1);
});
