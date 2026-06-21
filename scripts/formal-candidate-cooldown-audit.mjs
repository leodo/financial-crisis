#!/usr/bin/env node
import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

import {
  asArray,
  countFrom,
  horizonRow,
  metric,
  maybeRound,
  repoRelativePath,
  requireValue,
  resolveReviewReportPath,
  scenarioName,
  timestampForFile,
  writeJson,
  readJson
} from "./release-review-audit-common.mjs";

const DEFAULT_HISTORY_MODE = "strict_rebuild";
const DEFAULT_OUTPUT_DIR = "artifacts/research/cooldown-audit";

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
      throw new Error(`unknown cooldown audit option: ${arg}`);
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
  console.log(`Usage: node scripts/formal-candidate-cooldown-audit.mjs --baseline-release-id ID --candidate-release-id ID [options]

Exports a release-review-derived cooldown / false-positive audit JSON artifact.

Options:
  --history-mode MODE   Release review history mode, default ${DEFAULT_HISTORY_MODE}
  --report-path PATH    Explicit release-review JSON path
  --output PATH         Explicit output JSON path
  --output-dir DIR      Output directory, default ${DEFAULT_OUTPUT_DIR}
  --root DIR            Repository/deployment root, default current directory
`);
}

function runtimeRows(review, comparison) {
  const baselineRows = asArray(review.baseline_runtime_review?.regime_separation_summaries);
  const candidateRows = asArray(review.candidate_runtime_review?.regime_separation_summaries);
  const comparisonRows = asArray(comparison.runtime_separation_summary);
  const horizons = [...new Set([...baselineRows, ...candidateRows, ...comparisonRows].map((row) => countFrom(row.horizon_days)))].sort(
    (left, right) => left - right
  );

  return horizons.map((horizonDays) => {
    const baseline = horizonRow(baselineRows, horizonDays);
    const candidate = horizonRow(candidateRows, horizonDays);
    const candidateCooldown = candidate?.post_crisis_cooldown_avg_probability;
    const candidatePositive = candidate?.positive_window_avg_probability;
    const candidateNormal = candidate?.normal_avg_probability;
    return {
      horizon_days: horizonDays,
      baseline_diagnosis: baseline?.diagnosis ?? null,
      candidate_diagnosis: candidate?.diagnosis ?? null,
      baseline: baseline ?? null,
      candidate: candidate ?? null,
      comparison: horizonRow(comparisonRows, horizonDays) ?? {},
      candidate_cooldown_minus_positive: maybeRound(
        typeof candidateCooldown === "number" && typeof candidatePositive === "number"
          ? candidateCooldown - candidatePositive
          : null
      ),
      candidate_cooldown_minus_normal: maybeRound(
        typeof candidateCooldown === "number" && typeof candidateNormal === "number"
          ? candidateCooldown - candidateNormal
          : null
      )
    };
  });
}

function falsePositiveEpisodes(assessment) {
  return asArray(assessment?.backtest_summary?.rolling_audit?.classified_episodes)
    .filter((episode) => episode?.classification === "false_positive")
    .map((episode) => ({
      start_date: episode.start_date ?? "",
      end_date: episode.end_date ?? "",
      duration_days: countFrom(episode.duration_days),
      signal_count: countFrom(episode.signal_count),
      classification: episode.classification ?? "false_positive",
      note: episode.note ?? ""
    }))
    .filter((episode) => episode.start_date && episode.end_date);
}

function parseDate(value) {
  const date = new Date(`${value}T00:00:00Z`);
  if (Number.isNaN(date.getTime())) {
    return null;
  }
  return date;
}

function overlaps(left, right) {
  const leftStart = parseDate(left.start_date);
  const leftEnd = parseDate(left.end_date);
  const rightStart = parseDate(right.start_date);
  const rightEnd = parseDate(right.end_date);
  if (!leftStart || !leftEnd || !rightStart || !rightEnd) {
    return false;
  }
  return leftStart <= rightEnd && rightStart <= leftEnd;
}

function topEpisodes(episodes, limit = 10) {
  return [...episodes]
    .sort(
      (left, right) =>
        countFrom(right.duration_days) - countFrom(left.duration_days) ||
        String(right.start_date).localeCompare(String(left.start_date))
    )
    .slice(0, limit);
}

function episodeRegression(candidateEpisode, baselineEpisodes) {
  const overlapping = baselineEpisodes.filter((episode) => overlaps(candidateEpisode, episode));
  if (overlapping.length === 0) {
    return {
      kind: "candidate_only",
      episode: candidateEpisode,
      overlapping_baseline_episodes: []
    };
  }
  const maxBaselineDuration = Math.max(
    ...overlapping.map((episode) => countFrom(episode.duration_days))
  );
  if (countFrom(candidateEpisode.duration_days) > maxBaselineDuration) {
    return {
      kind: "extended_candidate_episode",
      episode: candidateEpisode,
      overlapping_baseline_episodes: overlapping
    };
  }
  return null;
}

function scenarioFalsePositiveDeltas(comparison) {
  return asArray(comparison.backtest_scenarios)
    .map((scenario) => {
      const baseline = countFrom(scenario.baseline_false_positive_count);
      const candidate = countFrom(scenario.candidate_false_positive_count);
      const delta = candidate - baseline;
      return delta === 0
        ? null
        : {
            scenario_id: scenario.scenario_id ?? "",
            name: scenarioName(scenario),
            baseline_false_positive_count: baseline,
            candidate_false_positive_count: candidate,
            delta,
            outcome: scenario.outcome ?? null
          };
    })
    .filter(Boolean)
    .sort(
      (left, right) =>
        right.delta - left.delta || String(left.scenario_id).localeCompare(String(right.scenario_id))
    );
}

function addReason(reasons, code, summary, evidence) {
  reasons.push({ code, summary, evidence });
}

function noGoReasons(review, comparison, cooldownRows) {
  const reasons = [];
  const precision = metric(comparison, "actionable_precision");
  if (
    precision &&
    (Number(precision.candidate ?? 1) < 0.7 || Number(precision.delta ?? 0) <= -0.05)
  ) {
    addReason(
      reasons,
      "actionable_precision_regression",
      "Candidate actionable precision is too weak for promotion.",
      precision
    );
  }

  const longestFalsePositive = metric(comparison, "longest_false_positive_episode_days");
  if (
    longestFalsePositive &&
    (Number(longestFalsePositive.delta ?? 0) >= 7 ||
      Number(longestFalsePositive.candidate ?? 0) > 30)
  ) {
    addReason(
      reasons,
      "longest_false_positive_episode_regression",
      "Candidate materially lengthens the longest pure false-positive episode.",
      longestFalsePositive
    );
  }

  const runtimeFloor = metric(comparison, "runtime_floor_hit_count");
  if (runtimeFloor && Number(runtimeFloor.delta ?? 0) <= -5) {
    addReason(
      reasons,
      "runtime_floor_hit_count_regression",
      "Candidate loses too many runtime floor hits.",
      runtimeFloor
    );
  }

  for (const row of cooldownRows) {
    if (row.horizon_days === 20 && row.candidate_diagnosis === "cooldown_bleed") {
      addReason(
        reasons,
        "candidate_20d_cooldown_bleed",
        "Candidate 20d runtime regime diagnosis is cooldown_bleed.",
        row.candidate ?? {}
      );
    }
    if (row.horizon_days === 60 && row.candidate_diagnosis === "cooldown_bleed") {
      addReason(
        reasons,
        "candidate_60d_cooldown_bleed",
        "Candidate 60d runtime regime diagnosis is cooldown_bleed.",
        row.candidate ?? {}
      );
    }
    if (
      row.horizon_days === 20 &&
      typeof row.candidate_cooldown_minus_positive === "number" &&
      row.candidate_cooldown_minus_positive >= 0
    ) {
      addReason(
        reasons,
        "candidate_20d_cooldown_not_below_positive",
        "Candidate 20d cooldown average is not below positive-window average.",
        {
          cooldown_minus_positive: row.candidate_cooldown_minus_positive,
          candidate: row.candidate ?? {}
        }
      );
    }
    if (
      row.horizon_days === 60 &&
      typeof row.candidate_cooldown_minus_positive === "number" &&
      row.candidate_cooldown_minus_positive >= 0
    ) {
      addReason(
        reasons,
        "candidate_60d_cooldown_not_below_positive",
        "Candidate 60d cooldown average is not below positive-window average.",
        {
          cooldown_minus_positive: row.candidate_cooldown_minus_positive,
          candidate: row.candidate ?? {}
        }
      );
    }
  }

  for (const regression of asArray(review.probability_guard_regressions)) {
    if (String(regression).includes("cooldown")) {
      addReason(
        reasons,
        "probability_guard_cooldown_regression",
        String(regression),
        { regression }
      );
    }
  }

  return reasons;
}

function recommendation(noGoReasonsValue, candidateRegressions) {
  if (noGoReasonsValue.length > 0) {
    return "no_go_cooldown_false_positive";
  }
  if (candidateRegressions.length > 0) {
    return "manual_review_false_positive_episode_changes";
  }
  return "cooldown_false_positive_clean";
}

async function outputPath() {
  if (options.outputPath) {
    return resolve(options.root, options.outputPath);
  }
  const dir = resolve(options.root, options.outputDir);
  await mkdir(dir, { recursive: true });
  return resolve(
    dir,
    `${timestampForFile()}-${options.baselineReleaseId}-vs-${options.candidateReleaseId}-${options.historyMode}-cooldown-audit.json`
  );
}

async function main() {
  const reviewPath = await resolveReviewReportPath(options);
  const review = await readJson(reviewPath);
  const comparison = review.comparison ?? {};
  const cooldownRows = runtimeRows(review, comparison);
  const baselineFp = falsePositiveEpisodes(review.baseline_assessment);
  const candidateFp = falsePositiveEpisodes(review.candidate_assessment);
  const candidateRegressions = topEpisodes(candidateFp, 25)
    .map((episode) => episodeRegression(episode, baselineFp))
    .filter(Boolean);
  const reasons = noGoReasons(review, comparison, cooldownRows);
  const artifact = {
    audit_type: "formal_candidate_cooldown_false_positive_audit",
    generated_at: new Date().toISOString(),
    baseline_release_id: options.baselineReleaseId,
    candidate_release_id: options.candidateReleaseId,
    market_scope: review.market_scope ?? "financial_system",
    history_mode: review.history_mode ?? options.historyMode,
    release_review_artifact: repoRelativePath(options.root, reviewPath),
    reviewed_at: review.reviewed_at ?? null,
    overall_guard_passed: review.overall_guard_passed ?? null,
    probability_guard_passed: review.probability_guard_passed ?? null,
    actionability_guard_passed: review.actionability_guard_passed ?? null,
    operational_guard_passed: review.operational_guard_passed ?? null,
    review_recommendation: review.recommendation ?? null,
    comparison_metrics: {
      timely_warning_rate: metric(comparison, "timely_warning_rate"),
      strict_actionable_point_count: metric(comparison, "strict_actionable_point_count"),
      runtime_floor_hit_count: metric(comparison, "runtime_floor_hit_count"),
      actionable_precision: metric(comparison, "actionable_precision"),
      longest_false_positive_episode_days: metric(
        comparison,
        "longest_false_positive_episode_days"
      )
    },
    runtime_cooldown_rows: cooldownRows,
    false_positive_episodes: {
      baseline_top: topEpisodes(baselineFp),
      candidate_top: topEpisodes(candidateFp),
      candidate_regressions: candidateRegressions
    },
    scenario_false_positive_deltas: scenarioFalsePositiveDeltas(comparison),
    no_go_reasons: reasons,
    recommendation: recommendation(reasons, candidateRegressions)
  };
  const path = await outputPath();
  await writeJson(path, artifact);
  console.log("Cooldown / false-positive audit exported.");
  console.log(`  release review : ${reviewPath}`);
  console.log(`  output         : ${path}`);
  console.log(`  recommendation : ${artifact.recommendation}`);
  console.log(`  no-go reasons  : ${artifact.no_go_reasons.length}`);
  console.log(`  episode changes: ${artifact.false_positive_episodes.candidate_regressions.length}`);
}

main().catch((error) => {
  console.error(`Cooldown / false-positive audit failed: ${error?.message ?? error}`);
  process.exit(1);
});
