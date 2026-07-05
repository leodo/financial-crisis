#!/usr/bin/env node
import { createReadStream } from "node:fs";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { basename, resolve } from "node:path";
import readline from "node:readline";
import { createHash } from "node:crypto";

const DEFAULT_OUTPUT_DIR = "artifacts/research/leadtime-audit/adjudication";
const DEFAULT_SCENARIO_CATALOG = "config/research_crisis_scenarios.us.json";

const options = parseArgs(process.argv.slice(2));

function parseArgs(args) {
  const parsed = {
    sliceCsv: "",
    scenarioCatalog: DEFAULT_SCENARIO_CATALOG,
    outputDir: DEFAULT_OUTPUT_DIR,
    horizonDays: 60,
    minScore: 0.9,
    maxGapDays: 3,
    root: process.cwd()
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--slice-csv") {
      parsed.sliceCsv = requireValue(args, ++index, arg);
    } else if (arg === "--scenario-catalog") {
      parsed.scenarioCatalog = requireValue(args, ++index, arg);
    } else if (arg === "--output-dir") {
      parsed.outputDir = requireValue(args, ++index, arg);
    } else if (arg === "--horizon-days") {
      parsed.horizonDays = Number(requireValue(args, ++index, arg));
    } else if (arg === "--min-score") {
      parsed.minScore = Number(requireValue(args, ++index, arg));
    } else if (arg === "--max-gap-days") {
      parsed.maxGapDays = Number(requireValue(args, ++index, arg));
    } else if (arg === "--root") {
      parsed.root = requireValue(args, ++index, arg);
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown high-score adjudication option: ${arg}`);
    }
  }

  if (!parsed.sliceCsv) {
    throw new Error("--slice-csv is required");
  }
  parsed.root = resolve(parsed.root);
  parsed.sliceCsv = resolve(parsed.root, parsed.sliceCsv);
  parsed.scenarioCatalog = resolve(parsed.root, parsed.scenarioCatalog);
  parsed.outputDir = resolve(parsed.root, parsed.outputDir);
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
  console.log(`Usage: node scripts/formal-highscore-adjudication.mjs --slice-csv PATH [options]

Clusters high-scoring no_scenario rows from a scored formal-probability-slice CSV.

Options:
  --horizon-days N       Probability horizon, default 60
  --min-score X          Minimum final probability for no_scenario rows, default 0.90
  --max-gap-days N       Maximum calendar gap inside a cluster, default 3
  --scenario-catalog P   Scenario catalog, default ${DEFAULT_SCENARIO_CATALOG}
  --output-dir DIR       Output directory, default ${DEFAULT_OUTPUT_DIR}
  --root DIR             Repository root, default current directory
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

function addDays(day, offset) {
  return new Date((day + offset) * 86_400_000).toISOString().slice(0, 10);
}

// Stable handle for a cluster, so the manual-review CSV can be joined back to
// slice rows across re-runs without relying on row order. The hash covers the
// full date span + peak so any membership change produces a different id.
function clusterId(start_date, end_date, peak_date) {
  const digest = createHash("sha1")
    .update(`${start_date}|${end_date}|${peak_date}`)
    .digest("hex");
  return `cl-${peak_date}-${digest.slice(0, 6)}`;
}

function round(value, digits = 4) {
  if (value == null || !Number.isFinite(value)) {
    return null;
  }
  const scale = 10 ** digits;
  return Math.round(value * scale) / scale;
}

async function loadScenarios(path) {
  const catalog = JSON.parse(await readFile(path, "utf8"));
  return (catalog.scenarios ?? []).map((scenario) => ({
    scenario_id: scenario.scenario_id,
    family: scenario.family ?? null,
    training_role: scenario.training_role ?? null,
    protected_window: Boolean(scenario.protected_window),
    pre_warning_start: scenario.pre_warning_start,
    crisis_start: scenario.crisis_start,
    crisis_end: scenario.crisis_end,
    pre_warning_day: dayNumber(scenario.pre_warning_start),
    crisis_start_day: dayNumber(scenario.crisis_start),
    crisis_end_day: dayNumber(scenario.crisis_end)
  }));
}

function scenarioContext(day, scenarios) {
  const containing = scenarios
    .filter((scenario) => day >= scenario.pre_warning_day && day <= scenario.crisis_end_day)
    .sort((left, right) => Math.abs(day - left.crisis_start_day) - Math.abs(day - right.crisis_start_day));
  if (containing.length > 0) {
    const scenario = containing[0];
    return {
      catalog_relation: day < scenario.crisis_start_day ? "inside_catalog_prewarning" : "inside_catalog_crisis_or_cooldown",
      nearest_scenario_id: scenario.scenario_id,
      nearest_scenario_family: scenario.family,
      nearest_scenario_training_role: scenario.training_role,
      nearest_scenario_protected_window: scenario.protected_window,
      nearest_days_to_crisis_start: scenario.crisis_start_day - day
    };
  }

  let nearest = null;
  for (const scenario of scenarios) {
    const distance = Math.min(
      Math.abs(day - scenario.pre_warning_day),
      Math.abs(day - scenario.crisis_start_day),
      Math.abs(day - scenario.crisis_end_day)
    );
    if (!nearest || distance < nearest.distance) {
      nearest = { scenario, distance };
    }
  }
  return {
    catalog_relation: "outside_catalog_windows",
    nearest_scenario_id: nearest?.scenario.scenario_id ?? null,
    nearest_scenario_family: nearest?.scenario.family ?? null,
    nearest_scenario_training_role: nearest?.scenario.training_role ?? null,
    nearest_scenario_protected_window: nearest?.scenario.protected_window ?? null,
    nearest_days_to_crisis_start: nearest ? nearest.scenario.crisis_start_day - day : null
  };
}

function safeJsonArray(text) {
  if (!text) {
    return [];
  }
  try {
    const parsed = JSON.parse(text);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function topContributionNames(text, count = 5) {
  return safeJsonArray(text)
    .slice(0, count)
    .map((item) => `${item.name}=${round(item.contribution, 3)}`)
    .join("|");
}

// Overlay contributions have a different shape than base: they are per-family
// gate rows ({family_id, gate, overlay_probability, contribution, ...}) in a
// fixed family order, not pre-sorted by contribution. Surface the families
// that actually moved the final probability, and include overlay_probability
// so the reviewer can tell whether a family gate is genuinely firing.
function topOverlayFamilyContributions(text, count = 5) {
  return safeJsonArray(text)
    .map((item) => ({
      family_id: item.family_id,
      contribution: Number(item.contribution),
      overlay_probability: Number(item.overlay_probability)
    }))
    .filter((item) => item.family_id && Number.isFinite(item.contribution))
    .sort((left, right) => Math.abs(right.contribution) - Math.abs(left.contribution))
    .slice(0, count)
    .map((item) => `${item.family_id}[p=${round(item.overlay_probability, 3)}]=${round(item.contribution, 4)}`)
    .join("|");
}

async function loadHighRows() {
  const scoreColumn = `final_p_${options.horizonDays}d`;
  const baseColumn = `base_contributions_${options.horizonDays}d_json`;
  const overlayColumn = `contributions_${options.horizonDays}d_json`;
  const rows = [];
  let header = null;
  let indexes = null;

  const stream = createReadStream(options.sliceCsv, { encoding: "utf8" });
  const lines = readline.createInterface({ input: stream, crlfDelay: Infinity });

  for await (const line of lines) {
    if (!header) {
      header = parseCsvLine(line);
      indexes = {
        asOfDate: header.indexOf("as_of_date"),
        splitName: header.indexOf("split_name"),
        scenarioId: header.indexOf("primary_scenario_id"),
        scenarioFamily: header.indexOf("scenario_family"),
        regime60d: header.indexOf("regime_60d"),
        score: header.indexOf(scoreColumn),
        baseContributions: header.indexOf(baseColumn),
        overlayContributions: header.indexOf(overlayColumn)
      };
      for (const [name, index] of Object.entries(indexes)) {
        if (index < 0 && !["baseContributions", "overlayContributions"].includes(name)) {
          throw new Error(`slice CSV is missing required column for ${name}`);
        }
      }
      continue;
    }

    const fields = parseCsvLine(line);
    const score = Number(fields[indexes.score]);
    const scenarioId = fields[indexes.scenarioId] || null;
    if (scenarioId || !Number.isFinite(score) || score < options.minScore) {
      continue;
    }

    rows.push({
      as_of_date: fields[indexes.asOfDate],
      day: dayNumber(fields[indexes.asOfDate]),
      split_name: fields[indexes.splitName],
      regime_60d: fields[indexes.regime60d],
      score,
      base_contributions_json:
        indexes.baseContributions >= 0 ? fields[indexes.baseContributions] : "",
      overlay_contributions_json:
        indexes.overlayContributions >= 0 ? fields[indexes.overlayContributions] : ""
    });
  }

  rows.sort((left, right) => left.day - right.day);
  return rows;
}

function clusterRows(rows, scenarios) {
  const clusters = [];
  let current = null;

  for (const row of rows) {
    if (!current || row.day - current.end_day > options.maxGapDays) {
      if (current) {
        clusters.push(finalizeCluster(current, scenarios));
      }
      current = {
        start_day: row.day,
        end_day: row.day,
        rows: [],
        peak: row
      };
    }
    current.rows.push(row);
    current.end_day = row.day;
    if (row.score > current.peak.score) {
      current.peak = row;
    }
  }

  if (current) {
    clusters.push(finalizeCluster(current, scenarios));
  }
  clusters.sort((left, right) => right.peak_score - left.peak_score);
  return clusters;
}

function finalizeCluster(cluster, scenarios) {
  const context = scenarioContext(cluster.peak.day, scenarios);
  const avgScore =
    cluster.rows.reduce((sum, row) => sum + row.score, 0) / Math.max(1, cluster.rows.length);
  const provisional = provisionalAdjudication(context, cluster);
  return {
    cluster_id: clusterId(
      addDays(cluster.start_day, 0),
      addDays(cluster.end_day, 0),
      cluster.peak.as_of_date
    ),
    start_date: addDays(cluster.start_day, 0),
    end_date: addDays(cluster.end_day, 0),
    day_count: cluster.rows.length,
    calendar_span_days: cluster.end_day - cluster.start_day + 1,
    peak_date: cluster.peak.as_of_date,
    peak_score: round(cluster.peak.score),
    avg_score: round(avgScore),
    split_names: [...new Set(cluster.rows.map((row) => row.split_name))].sort().join("|"),
    regime_60d_values: [...new Set(cluster.rows.map((row) => row.regime_60d))].sort().join("|"),
    ...context,
    provisional_class: provisional.provisional_class,
    provisional_action: provisional.provisional_action,
    provisional_rationale: provisional.provisional_rationale,
    peak_top_base_contributions: topContributionNames(cluster.peak.base_contributions_json),
    peak_top_overlay_contributions: topOverlayFamilyContributions(cluster.peak.overlay_contributions_json)
  };
}

function provisionalAdjudication(context, cluster) {
  if (context.catalog_relation === "inside_catalog_prewarning") {
    return {
      provisional_class: "catalog_prewarning_leakage",
      provisional_action:
        "inspect scenario action-window rules; likely should not be counted as no_scenario false positive",
      provisional_rationale:
        "peak date falls inside the scenario catalog pre-warning window, but the scored formal row has no primary_scenario_id"
    };
  }

  if (context.catalog_relation === "inside_catalog_crisis_or_cooldown") {
    const isPostStart = Number(context.nearest_days_to_crisis_start ?? 1) <= 0;
    return {
      provisional_class: isPostStart
        ? "catalog_context_not_actionable"
        : "catalog_prewarning_leakage",
      provisional_action:
        "exclude from prepare-head false-positive pressure or route through post-start/context diagnostics",
      provisional_rationale:
        "peak date falls inside a known scenario window, so this is a scenario coverage/window-mapping issue rather than ordinary background"
    };
  }

  const startYear = Number(addDays(cluster.start_day, 0).slice(0, 4));
  const isRecent = startYear >= 2023;
  return {
    provisional_class: isRecent
      ? "requires_current_stress_or_near_miss_review"
      : "requires_historical_near_miss_review",
    provisional_action:
      "human adjudication required before using this row as either a negative or prepare-positive example",
    provisional_rationale:
      "peak date is outside configured scenario windows; model score is high enough that treating it as an ordinary negative would encode a policy decision"
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

function summarize(clusters) {
  const byRelation = new Map();
  for (const cluster of clusters) {
    const key = `${cluster.catalog_relation}:${cluster.provisional_class}`;
    const existing = byRelation.get(key) ?? {
      catalog_relation: cluster.catalog_relation,
      provisional_class: cluster.provisional_class,
      cluster_count: 0,
      day_count: 0,
      max_peak_score: 0,
      scenarios: new Set()
    };
    existing.cluster_count += 1;
    existing.day_count += cluster.day_count;
    existing.max_peak_score = Math.max(existing.max_peak_score, cluster.peak_score);
    if (cluster.nearest_scenario_id) {
      existing.scenarios.add(cluster.nearest_scenario_id);
    }
    byRelation.set(key, existing);
  }
  return [...byRelation.values()]
    .map((row) => ({
      catalog_relation: row.catalog_relation,
      provisional_class: row.provisional_class,
      cluster_count: row.cluster_count,
      day_count: row.day_count,
      max_peak_score: round(row.max_peak_score),
      nearest_scenarios: [...row.scenarios].sort().join("|")
    }))
    .sort((left, right) => right.day_count - left.day_count);
}

function manualReviewRows(clusters) {
  const allowedClasses = [
    "near_miss_prepare_positive",
    "true_false_positive",
    "current_stress_watch",
    "ignore_for_prepare_eval",
    "protected_context_positive",
    "post_crisis_cooldown"
  ].join("|");
  return clusters
    .filter((cluster) => cluster.provisional_class.startsWith("requires_"))
    .map((cluster) => ({
      cluster_id: cluster.cluster_id,
      adjudicated_class: "",
      allowed_classes: allowedClasses,
      start_date: cluster.start_date,
      end_date: cluster.end_date,
      day_count: cluster.day_count,
      peak_date: cluster.peak_date,
      peak_score: cluster.peak_score,
      split_names: cluster.split_names,
      regime_60d_values: cluster.regime_60d_values,
      provisional_class: cluster.provisional_class,
      nearest_scenario_id: cluster.nearest_scenario_id,
      nearest_days_to_crisis_start: cluster.nearest_days_to_crisis_start,
      nearest_scenario_training_role: cluster.nearest_scenario_training_role,
      peak_top_base_contributions: cluster.peak_top_base_contributions,
      peak_top_overlay_contributions: cluster.peak_top_overlay_contributions,
      adjudication_notes: ""
    }));
}

async function main() {
  const scenarios = await loadScenarios(options.scenarioCatalog);
  const rows = await loadHighRows();
  const clusters = clusterRows(rows, scenarios);
  const summary = summarize(clusters);
  const reviewRows = manualReviewRows(clusters);

  await mkdir(options.outputDir, { recursive: true });
  const stem = basename(options.sliceCsv, ".csv").replace(
    "formal-probability-slice",
    `highscore-no-scenario-${options.horizonDays}d`
  );
  const jsonPath = resolve(options.outputDir, `${stem}.json`);
  const clustersCsvPath = resolve(options.outputDir, `${stem}-clusters.csv`);
  const summaryCsvPath = resolve(options.outputDir, `${stem}-summary.csv`);
  const manualReviewCsvPath = resolve(options.outputDir, `${stem}-manual-review.csv`);

  await writeFile(
    jsonPath,
    JSON.stringify(
      {
        generated_at: new Date().toISOString(),
        slice_csv: options.sliceCsv,
        scenario_catalog: options.scenarioCatalog,
        horizon_days: options.horizonDays,
        min_score: options.minScore,
        max_gap_days: options.maxGapDays,
        high_score_no_scenario_row_count: rows.length,
        cluster_count: clusters.length,
        manual_review_cluster_count: reviewRows.length,
        summary,
        manual_review_rows: reviewRows,
        clusters
      },
      null,
      2
    )
  );
  await writeFile(clustersCsvPath, renderCsv(clusters));
  await writeFile(summaryCsvPath, renderCsv(summary));
  await writeFile(manualReviewCsvPath, renderCsv(reviewRows));

  console.log("Formal high-score adjudication exported.");
  console.log(`  high rows : ${rows.length}`);
  console.log(`  clusters  : ${clusters.length}`);
  console.log(`  manual    : ${reviewRows.length}`);
  console.log(`  json      : ${jsonPath}`);
  console.log(`  clusters  : ${clustersCsvPath}`);
  console.log(`  summary   : ${summaryCsvPath}`);
  console.log(`  review    : ${manualReviewCsvPath}`);
}

main().catch((error) => {
  console.error(`Formal high-score adjudication failed: ${error?.message ?? error}`);
  process.exit(1);
});
