#!/usr/bin/env node
// Post-hoc consumer for the prepare-head adjudication policy.
//
// Reads a formal-probability-slice CSV alongside a human-filled adjudication
// CSV (produced by scripts/formal-highscore-adjudication.mjs) and recomputes
// the prepare-head counts for the no-scenario high-score subset, applying the
// adjudicated_class overrides defined in ADJUDICATION-POLICY.md.
//
// This is deliberately a post-hoc Node script, NOT a hook in the Rust training
// pipeline. The pipeline consumes in-memory ProbabilityTrainingRow objects from
// SQLite, not the slice CSV, and threading adjudication into the pipeline would
// both risk contaminating training labels and have nothing to consume until
// rows are adjudicated. See ADJUDICATION-POLICY.md "Flow-back into evaluation".
//
// For no-scenario rows the action-episode phase is always Outside, so every
// no-scenario predicted-positive row is currently counted as a prepare false
// positive. This script partitions those rows into:
//   - manual-review scope (provisional_class "requires_*"): the adjudication
//     target. The human-filled adjudicated_class reclassifies them (TP /
//     cooldown / excluded / stays FP / pending).
//   - auto-classified (provisional_class "catalog_*"): catalog-window overlap
//     handled by the adjudication script's own rules, out of manual-adjudication
//     scope. Reported separately, not in the FP swing.
//   - orphaned: no cluster at all. Flagged for attention.
// The consumer never infers a class from model output.

import { createReadStream, existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { basename, resolve } from "node:path";
import readline from "node:readline";

const DEFAULT_OUTPUT_DIR = "artifacts/research/leadtime-audit/adjudication";

const options = parseArgs(process.argv.slice(2));

function parseArgs(args) {
  const parsed = {
    sliceCsv: "",
    adjudicationCsv: "",
    clustersCsv: "",
    outputDir: DEFAULT_OUTPUT_DIR,
    horizonDays: 60,
    minScore: 0.9,
    split: "evaluation",
    root: process.cwd()
  };
  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--slice-csv") {
      parsed.sliceCsv = requireValue(args, ++index, arg);
    } else if (arg === "--adjudication-csv") {
      parsed.adjudicationCsv = requireValue(args, ++index, arg);
    } else if (arg === "--clusters-csv") {
      parsed.clustersCsv = requireValue(args, ++index, arg);
    } else if (arg === "--output-dir") {
      parsed.outputDir = requireValue(args, ++index, arg);
    } else if (arg === "--horizon-days") {
      parsed.horizonDays = Number(requireValue(args, ++index, arg));
    } else if (arg === "--min-score") {
      parsed.minScore = Number(requireValue(args, ++index, arg));
    } else if (arg === "--split") {
      parsed.split = requireValue(args, ++index, arg);
    } else if (arg === "--root") {
      parsed.root = requireValue(args, ++index, arg);
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown prepare-eval-adjudicated option: ${arg}`);
    }
  }
  if (!parsed.sliceCsv) {
    throw new Error("--slice-csv is required");
  }
  if (!parsed.adjudicationCsv) {
    throw new Error("--adjudication-csv is required");
  }
  parsed.root = resolve(parsed.root);
  parsed.sliceCsv = resolve(parsed.root, parsed.sliceCsv);
  parsed.adjudicationCsv = resolve(parsed.root, parsed.adjudicationCsv);
  parsed.outputDir = resolve(parsed.root, parsed.outputDir);
  if (parsed.clustersCsv) {
    parsed.clustersCsv = resolve(parsed.root, parsed.clustersCsv);
  } else {
    // Auto-derive the full clusters CSV from the manual-review path so rows can
    // be partitioned into manual-review vs auto-classified vs orphaned. The two
    // files are co-produced by formal-highscore-adjudication.mjs.
    parsed.clustersCsv = parsed.adjudicationCsv.replace(
      "-manual-review.csv",
      "-clusters.csv"
    );
  }
  return parsed;
}

function requireValue(args, index, flag) {
  const value = args[index];
  if (!value || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function printHelp() {
  console.log(`Usage: node scripts/formal-prepare-eval-adjudicated.mjs --slice-csv PATH --adjudication-csv PATH [options]

Recomputes prepare-head FP/TP/cooldown/excluded counts for the no-scenario
high-score subset, applying human-filled adjudicated_class overrides.

Options:
  --clusters-csv P   Full clusters CSV (all 21 clusters). Auto-derived from the
                     adjudication-csv path if omitted; used to partition rows
                     into manual-review vs auto-classified vs orphaned.
  --horizon-days N   Probability horizon, default 60
  --min-score X      Minimum final probability for the no-scenario subset, default 0.90
  --split NAME       split_name to restrict to (e.g. "evaluation"); "all" disables filtering
  --output-dir DIR   Output directory, default ${DEFAULT_OUTPUT_DIR}
  --root DIR         Repository root, default current directory
`);
}

function parseCsvLine(line) {
  const fields = [];
  let current = "";
  let quoted = false;
  for (let index = 0; index < line.length; index += 1) {
    const char = line[index];
    if (quoted) {
      if (char === '"') {
        if (line[index + 1] === '"') {
          current += '"';
          index += 1;
        } else {
          quoted = false;
        }
      } else {
        current += char;
      }
    } else if (char === '"') {
      quoted = true;
    } else if (char === ",") {
      fields.push(current);
      current = "";
    } else {
      current += char;
    }
  }
  fields.push(current);
  return fields;
}

function dayNumber(dateText) {
  const [year, month, day] = dateText.split("-").map(Number);
  return Math.floor(Date.UTC(year, month - 1, day) / 86_400_000);
}

// Map an adjudicated class to its prepare-head reclassification bucket.
//   "tp"       -> reclassified from FP to true positive (near_miss / protected_context)
//   "cooldown" -> moved to cooldown_hit, leaves the FP denominator
//   "excluded" -> removed from the denominator entirely (current_stress_watch / ignore)
//   "fp"       -> stays a false positive (true_false_positive)
//   "pending"  -> adjudicated_class blank or unrecognized; row left as-is with a warning
function reclassify(adjudicatedClass) {
  switch ((adjudicatedClass || "").trim()) {
    case "near_miss_prepare_positive":
    case "protected_context_positive":
      return { bucket: "tp" };
    case "true_false_positive":
      return { bucket: "fp" };
    case "post_crisis_cooldown":
      return { bucket: "cooldown" };
    case "current_stress_watch":
    case "ignore_for_prepare_eval":
      return { bucket: "excluded" };
    default:
      return { bucket: "pending" };
  }
}

// Reads a cluster-shaped CSV (either the manual-review CSV or the full clusters
// CSV). Returns rows with whatever of {adjudicated_class, provisional_class}
// is present; missing columns come back as "".
async function loadClusterCsv(path, { requireAdjudicatedClass = false } = {}) {
  const rows = [];
  let header = null;
  let indexes = null;
  const stream = createReadStream(path, { encoding: "utf8" });
  const lines = readline.createInterface({ input: stream, crlfDelay: Infinity });
  for await (const line of lines) {
    const fields = parseCsvLine(line);
    if (!header) {
      header = fields;
      indexes = {
        cluster_id: header.indexOf("cluster_id"),
        start_date: header.indexOf("start_date"),
        end_date: header.indexOf("end_date"),
        peak_date: header.indexOf("peak_date"),
        provisional_class: header.indexOf("provisional_class"),
        adjudicated_class: header.indexOf("adjudicated_class")
      };
      if (indexes.cluster_id < 0 || indexes.start_date < 0 || indexes.end_date < 0) {
        throw new Error(`cluster CSV ${path} is missing required columns (cluster_id/start_date/end_date)`);
      }
      if (requireAdjudicatedClass && indexes.adjudicated_class < 0) {
        throw new Error(`adjudication CSV ${path} is missing the adjudicated_class column`);
      }
      continue;
    }
    rows.push({
      cluster_id: fields[indexes.cluster_id],
      start_date: fields[indexes.start_date],
      end_date: fields[indexes.end_date],
      start_day: dayNumber(fields[indexes.start_date]),
      end_day: dayNumber(fields[indexes.end_date]),
      peak_date: indexes.peak_date >= 0 ? fields[indexes.peak_date] : "",
      provisional_class: indexes.provisional_class >= 0 ? fields[indexes.provisional_class] : "",
      adjudicated_class:
        indexes.adjudicated_class >= 0 ? (fields[indexes.adjudicated_class] || "").trim() : ""
    });
  }
  rows.sort((left, right) => left.start_day - right.start_day);
  return rows;
}

// Merge the full clusters CSV (all clusters, with provisional_class) and the
// manual-review CSV (the subset with human-filled adjudicated_class). The
// merged list carries provisional_class for every cluster and adjudicated_class
// overlaid from the manual-review rows. If the clusters CSV is absent, falls
// back to the manual-review rows alone (auto-classified clusters invisible).
async function loadMergedClusters(adjudicationCsv, clustersCsv) {
  const manual = await loadClusterCsv(adjudicationCsv, { requireAdjudicatedClass: true });
  if (!clustersCsv || !existsSync(clustersCsv)) {
    if (clustersCsv) {
      console.warn(
        `clusters CSV not found at ${clustersCsv}; falling back to manual-review clusters only (auto-classified clusters invisible)`
      );
    }
    return manual;
  }
  const all = await loadClusterCsv(clustersCsv);
  const adjudicatedById = new Map(manual.map((row) => [row.cluster_id, row.adjudicated_class]));
  return all.map((cluster) => ({
    ...cluster,
    adjudicated_class: adjudicatedById.has(cluster.cluster_id)
      ? adjudicatedById.get(cluster.cluster_id)
      : cluster.adjudicated_class
  }));
}

function clusterForDay(day, clusters) {
  for (const cluster of clusters) {
    if (day >= cluster.start_day && day <= cluster.end_day) {
      return cluster;
    }
  }
  return null;
}

async function loadEligibleRows(sliceCsv, horizonDays, minScore, split) {
  const scoreColumn = `final_p_${horizonDays}d`;
  const rows = [];
  let header = null;
  let indexes = null;
  const stream = createReadStream(sliceCsv, { encoding: "utf8" });
  const lines = readline.createInterface({ input: stream, crlfDelay: Infinity });
  for await (const line of lines) {
    const fields = parseCsvLine(line);
    if (!header) {
      header = fields;
      indexes = {
        as_of_date: header.indexOf("as_of_date"),
        split_name: header.indexOf("split_name"),
        scenario_id: header.indexOf("primary_scenario_id"),
        score: header.indexOf(scoreColumn),
        prepare_episode_label: header.indexOf("prepare_episode_label")
      };
      for (const [name, index] of Object.entries(indexes)) {
        if (index < 0 && name !== "prepare_episode_label") {
          const lookedFor = name === "score" ? scoreColumn : name;
          throw new Error(`slice CSV is missing required column for ${name} (looked for ${lookedFor})`);
        }
      }
      continue;
    }
    if (fields[indexes.scenario_id]) {
      continue; // only no-scenario rows are adjudication-eligible
    }
    if (split !== "all" && (fields[indexes.split_name] || "") !== split) {
      continue;
    }
    const score = Number(fields[indexes.score]);
    if (!Number.isFinite(score) || score < minScore) {
      continue;
    }
    rows.push({
      as_of_date: fields[indexes.as_of_date],
      day: dayNumber(fields[indexes.as_of_date]),
      split_name: fields[indexes.split_name],
      score,
      prepare_episode_label:
        indexes.prepare_episode_label >= 0 ? Number(fields[indexes.prepare_episode_label]) : null
    });
  }
  rows.sort((left, right) => left.day - right.day);
  return rows;
}

function isManualReview(cluster) {
  return Boolean(cluster.provisional_class && cluster.provisional_class.startsWith("requires_"));
}

function isAutoClassified(cluster) {
  return Boolean(cluster.provisional_class && cluster.provisional_class.startsWith("catalog_"));
}

function aggregate(rows, clusters) {
  const byCluster = new Map();
  const totals = {
    eligible: rows.length,
    orphaned: 0,
    tp: 0,
    cooldown: 0,
    excluded: 0,
    fp_stays: 0,
    pending: 0,
    auto: 0,
    label_nonzero_warn: 0
  };

  for (const row of rows) {
    const cluster = clusterForDay(row.day, clusters);
    if (!cluster) {
      totals.orphaned += 1;
      continue;
    }
    // No-scenario rows should have prepare_episode_label == 0. A nonzero value
    // would mean the row is already a positive (not an FP) and adjudication is
    // mis-targeted; surface it rather than silently double-counting.
    if (row.prepare_episode_label != null && row.prepare_episode_label >= 0.5) {
      totals.label_nonzero_warn += 1;
    }

    let bucket;
    if (cluster.adjudicated_class) {
      bucket = reclassify(cluster.adjudicated_class).bucket;
    } else if (isAutoClassified(cluster)) {
      bucket = "auto";
    } else {
      bucket = "pending";
    }
    const key = bucket === "fp" ? "fp_stays" : bucket;

    let acc = byCluster.get(cluster.cluster_id);
    if (!acc) {
      acc = {
        cluster_id: cluster.cluster_id,
        start_date: cluster.start_date,
        end_date: cluster.end_date,
        peak_date: cluster.peak_date,
        adjudicated_class: cluster.adjudicated_class || "(blank)",
        provisional_class: cluster.provisional_class,
        scope: isManualReview(cluster) ? "manual" : isAutoClassified(cluster) ? "auto" : "unknown",
        row_count: 0,
        tp: 0,
        cooldown: 0,
        excluded: 0,
        fp_stays: 0,
        pending: 0,
        auto: 0
      };
      byCluster.set(cluster.cluster_id, acc);
    }
    acc.row_count += 1;
    acc[key] += 1;
    totals[key] += 1;
  }

  // Manual-review scope = rows in requires_* clusters (the adjudication target).
  const inManual = totals.tp + totals.cooldown + totals.excluded + totals.fp_stays + totals.pending;
  return {
    clusters: [...byCluster.values()].sort((left, right) => {
      if (left.start_date < right.start_date) return -1;
      if (left.start_date > right.start_date) return 1;
      return 0;
    }),
    totals,
    in_manual: inManual
  };
}

function csvEscape(value) {
  if (value == null) {
    return "";
  }
  const text = String(value);
  return /[",\n]/.test(text) ? `"${text.replaceAll('"', '""')}"` : text;
}

function renderCsv(rows) {
  if (rows.length === 0) {
    return "";
  }
  const header = Object.keys(rows[0]);
  return [
    header.join(","),
    ...rows.map((row) => header.map((name) => csvEscape(row[name])).join(","))
  ].join("\n");
}

async function main() {
  const clusters = await loadMergedClusters(options.adjudicationCsv, options.clustersCsv);
  const rows = await loadEligibleRows(
    options.sliceCsv,
    options.horizonDays,
    options.minScore,
    options.split
  );
  const { clusters: clusterAggs, totals, in_manual: inManual } = aggregate(rows, clusters);

  const manualClusterCount = clusterAggs.filter((acc) => acc.scope === "manual").length;
  const autoClusterCount = clusterAggs.filter((acc) => acc.scope === "auto").length;

  await mkdir(options.outputDir, { recursive: true });
  const stem = basename(options.sliceCsv, ".csv").replace(
    "formal-probability-slice",
    `prepare-eval-adjudicated-${options.horizonDays}d`
  );
  const outPath = resolve(options.outputDir, `${stem}.csv`);
  const outRows = clusterAggs.map((acc) => {
    const inManualScope = acc.scope === "manual";
    // FP accounting only applies to manual-review scope. Auto-classified rows
    // are out of scope (catalog-window overlap, phase unresolved here).
    const currentFp = inManualScope ? acc.row_count : "";
    const postFp = inManualScope ? acc.fp_stays + acc.pending : "";
    const fpDelta = inManualScope ? acc.fp_stays + acc.pending - acc.row_count : "";
    return {
      cluster_id: acc.cluster_id,
      start_date: acc.start_date,
      end_date: acc.end_date,
      peak_date: acc.peak_date,
      scope: acc.scope,
      adjudicated_class: acc.adjudicated_class,
      provisional_class: acc.provisional_class,
      row_count: acc.row_count,
      current_fp: currentFp,
      post_tp: acc.tp,
      post_cooldown: acc.cooldown,
      post_excluded: acc.excluded,
      post_fp: postFp,
      pending: acc.pending,
      auto: acc.auto,
      fp_delta: fpDelta
    };
  });
  await writeFile(outPath, renderCsv(outRows));

  console.log("Prepare-head adjudication recompute exported.");
  console.log(`  slice          : ${options.sliceCsv}`);
  console.log(`  adjudication   : ${options.adjudicationCsv}`);
  console.log(`  clusters       : ${options.clustersCsv}`);
  console.log(`  split          : ${options.split}`);
  console.log(`  eligible rows  : ${totals.eligible} (no-scenario, score >= ${options.minScore})`);
  console.log(
    `  manual-review  : ${inManual} rows across ${manualClusterCount} clusters (adjudication target)`
  );
  console.log(`    -> TP        : ${totals.tp} (near_miss + protected_context)`);
  console.log(`    -> cooldown  : ${totals.cooldown} (post_crisis_cooldown)`);
  console.log(`    -> excluded  : ${totals.excluded} (current_stress_watch + ignore)`);
  console.log(
    `    -> stays FP  : ${totals.fp_stays + totals.pending} (true_false_positive ${totals.fp_stays} + pending ${totals.pending})`
  );
  console.log(`    pending      : ${totals.pending} (adjudicated_class blank or unrecognized)`);
  console.log(
    `  auto-classified: ${totals.auto} rows across ${autoClusterCount} clusters (catalog-window overlap, out of manual-adjudication scope)`
  );
  console.log(`  orphaned       : ${totals.orphaned} (no cluster at all)`);
  const fromFp = inManual;
  const toFp = totals.fp_stays + totals.pending; // undecided rows remain FP
  console.log(
    `  FP swing       : ${toFp - fromFp} (from ${fromFp} to ${toFp}; manual-review scope, assumes pending remain FP)`
  );
  console.log(
    `  scope note     : swing covers only the high-score subset (score >= ${options.minScore}); the full prepare FP rate also includes lower-score no-scenario FPs not in any adjudication cluster`
  );
  if (totals.label_nonzero_warn) {
    console.log(
      `  WARN           : ${totals.label_nonzero_warn} no-scenario rows had prepare_episode_label >= 0.5 (expected 0); check adjudication targeting`
    );
  }
  console.log(`  csv            : ${outPath}`);

  // When running across all splits, also emit a per-split breakdown so the
  // reviewer sees which split each pending cluster lands in without re-running.
  // Rows are already in memory; this is a second pass over the same data with a
  // split_name filter, no extra I/O.
  if (options.split === "all") {
    const splitNames = ["train", "calibration", "evaluation"];
    const breakdown = [];
    for (const splitName of splitNames) {
      const splitRows = rows.filter((row) => row.split_name === splitName);
      if (splitRows.length === 0) {
        continue;
      }
      const { totals: splitTotals, in_manual: splitInManual } = aggregate(splitRows, clusters);
      breakdown.push({
        split: splitName,
        eligible: splitTotals.eligible,
        manual: splitInManual,
        auto: splitTotals.auto,
        orphaned: splitTotals.orphaned,
        tp: splitTotals.tp,
        cooldown: splitTotals.cooldown,
        excluded: splitTotals.excluded,
        fp_stays: splitTotals.fp_stays,
        pending: splitTotals.pending,
        fp_pre: splitInManual,
        fp_post: splitTotals.fp_stays + splitTotals.pending
      });
    }
    console.log("  per-split      : eligible | manual(auto/orphan) | TP cool excl FP-stay pending | FP pre->post");
    for (const row of breakdown) {
      console.log(
        `    ${row.split.padEnd(12)} ${String(row.eligible).padStart(3)} | ${String(row.manual).padStart(3)} (${row.auto}/${row.orphaned}) | TP ${String(row.tp).padStart(2)} cool ${String(row.cooldown).padStart(2)} excl ${String(row.excluded).padStart(2)} stay ${String(row.fp_stays).padStart(2)} pend ${String(row.pending).padStart(2)} | ${row.fp_pre}->${row.fp_post}`
      );
    }
    const breakdownPath = resolve(
      options.outputDir,
      `${stem}-by-split.csv`
    );
    await writeFile(breakdownPath, renderCsv(breakdown));
    console.log(`  by-split csv   : ${breakdownPath}`);
  }
}

main().catch((error) => {
  console.error(`Prepare-head adjudication recompute failed: ${error?.message ?? error}`);
  process.exit(1);
});
