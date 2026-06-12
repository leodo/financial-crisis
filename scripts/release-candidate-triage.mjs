import fs from "node:fs";
import path from "node:path";

const candidateScreenDir = path.join("artifacts", "research", "candidate-screen");
const releaseReviewDir = path.join("artifacts", "research", "release-review");
const bundleDir = path.join("artifacts", "research", "model-bundles", "generated");

const baselineFilter = process.argv[2] || null;
const limitArg = Number.parseInt(process.argv[3] ?? "6", 10);
const limit = Number.isFinite(limitArg) && limitArg > 0 ? limitArg : 6;

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8").replace(/^\uFEFF/, ""));
}

function percent(value, digits = 1) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return "n/a";
  }
  return `${(value * 100).toFixed(digits)}%`;
}

function days(value) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return "n/a";
  }
  return `${value}d`;
}

function shortList(values, maxItems = 3) {
  if (!Array.isArray(values) || values.length === 0) {
    return [];
  }
  return values.slice(0, maxItems).filter((value) => typeof value === "string" && value.trim());
}

function summarizeThresholdBlockers(blockers) {
  if (!Array.isArray(blockers) || blockers.length === 0) {
    return "none";
  }
  const hardCount = blockers.filter((blocker) => blocker?.severity === "hard").length;
  const warnCount = blockers.filter((blocker) => blocker?.severity === "warn").length;
  return `hard ${hardCount}, warn ${warnCount}`;
}

function summarizeCooldown(rows, horizon) {
  const row = Array.isArray(rows) ? rows.find((item) => item?.horizon === horizon) : null;
  if (!row) {
    return `${horizon}: n/a`;
  }
  return `${horizon}: ${row.candidate_diagnosis} (positive ${percent(row.candidate_positive)}, cooldown ${percent(row.candidate_cooldown)})`;
}

function summarizeThreshold(rows) {
  const thresholdRow = Array.isArray(rows)
    ? rows.find((item) => item?.metric === "threshold20")
    : null;
  const normalRow = Array.isArray(rows)
    ? rows.find((item) => item?.metric === "normal_avg_p20d")
    : null;
  const bufferRow = Array.isArray(rows)
    ? rows.find((item) => item?.metric === "buffer_avg_p20d")
    : null;
  return [
    `threshold20 ${percent(thresholdRow?.baseline, 1)} -> ${percent(thresholdRow?.candidate, 1)}`,
    `normal_avg_p20d ${percent(normalRow?.baseline, 1)} -> ${percent(normalRow?.candidate, 1)}`,
    `buffer_avg_p20d ${percent(bufferRow?.baseline, 1)} -> ${percent(bufferRow?.candidate, 1)}`
  ].join(" | ");
}

function loadBundleSummary(releaseId) {
  const bundlePath = path.join(bundleDir, `${releaseId}.json`);
  if (!fs.existsSync(bundlePath)) {
    return null;
  }
  const json = readJson(bundlePath);
  const horizons = Array.isArray(json.horizons) ? json.horizons : [];
  return horizons.map((horizon) => ({
    horizon_days: horizon.horizon_days,
    decision_threshold: horizon.decision_threshold ?? null,
    calibration_beta: horizon.calibration?.beta ?? null,
    calibration_alpha: horizon.calibration?.alpha ?? null,
    evaluation_diagnosis: horizon.evaluation?.regime_separation?.diagnosis ?? null,
    positive_window_gap_vs_normal:
      horizon.evaluation?.regime_separation?.positive_window_gap_vs_normal ?? null,
    post_crisis_cooldown_gap_vs_normal:
      horizon.evaluation?.regime_separation?.post_crisis_cooldown_gap_vs_normal ?? null
  }));
}

function summarizeBundle(bundleSummary, horizonDays) {
  const row = Array.isArray(bundleSummary)
    ? bundleSummary.find((item) => item?.horizon_days === horizonDays)
    : null;
  if (!row) {
    return `${horizonDays}d: n/a`;
  }
  return `${horizonDays}d: threshold ${percent(row.decision_threshold, 1)}, diagnosis ${row.evaluation_diagnosis ?? "n/a"}, pos-normal gap ${percent(row.positive_window_gap_vs_normal, 1)}, cooldown-normal gap ${percent(row.post_crisis_cooldown_gap_vs_normal, 1)}`;
}

function listJsonFiles(dirPath) {
  if (!fs.existsSync(dirPath)) {
    return [];
  }
  return fs
    .readdirSync(dirPath)
    .filter((name) => name.endsWith(".json"))
    .map((name) => {
      const fullPath = path.join(dirPath, name);
      return {
        name,
        fullPath,
        mtimeMs: fs.statSync(fullPath).mtimeMs
      };
    })
    .sort((left, right) => right.mtimeMs - left.mtimeMs);
}

const reviewIndex = new Map();
for (const file of listJsonFiles(releaseReviewDir)) {
  const json = readJson(file.fullPath);
  const baseline = json.baseline_release?.release_id ?? json.baseline_release_id ?? null;
  const candidate = json.candidate_release?.release_id ?? json.candidate_release_id ?? null;
  const historyMode = json.history_mode ?? "unknown";
  if (!baseline || !candidate) {
    continue;
  }
  const key = `${baseline}::${candidate}::${historyMode}`;
  if (!reviewIndex.has(key)) {
    reviewIndex.set(key, { file, json });
  }
}

const screens = [];
for (const file of listJsonFiles(candidateScreenDir)) {
  const json = readJson(file.fullPath);
  if (baselineFilter && json.baseline_release_id !== baselineFilter) {
    continue;
  }
  screens.push({ file, json });
}

const latestByCandidate = [];
const seenCandidates = new Set();
for (const screen of screens) {
  const candidate = screen.json.candidate_release_id;
  if (!candidate || seenCandidates.has(candidate)) {
    continue;
  }
  seenCandidates.add(candidate);
  latestByCandidate.push(screen);
  if (latestByCandidate.length >= limit) {
    break;
  }
}

if (latestByCandidate.length === 0) {
  console.log("No candidate-screen artifacts matched the requested baseline filter.");
  process.exit(1);
}

const baseline = latestByCandidate[0].json.baseline_release_id;
console.log(`Baseline: ${baseline}`);
console.log(`Candidates reviewed: ${latestByCandidate.length}`);
console.log("");

let closestCandidate = null;

for (const [index, entry] of latestByCandidate.entries()) {
  const screen = entry.json;
  const candidate = screen.candidate_release_id;
  const reviewKey = `${screen.baseline_release_id}::${candidate}::default`;
  const review = reviewIndex.get(reviewKey)?.json ?? null;
  const comparison = review?.comparison ?? null;
  const actionablePrecision = comparison?.actionable_precision ?? null;
  const timelyWarningRate = comparison?.timely_warning_rate ?? null;
  const longestFalsePositive = comparison?.longest_false_positive_episode_days ?? null;
  const runtimeFloorHits = comparison?.runtime_floor_hit_count ?? null;
  const hardBlockers = Array.isArray(screen.threshold_policy_blockers)
    ? screen.threshold_policy_blockers.filter((blocker) => blocker?.severity === "hard").length
    : 0;
  const bundleSummary = loadBundleSummary(candidate);

  if (
    !closestCandidate &&
    review &&
    actionablePrecision?.candidate > 0 &&
    timelyWarningRate?.candidate > 0
  ) {
    closestCandidate = {
      candidate,
      reasons: screen.reasons ?? [],
      comparison
    };
  }

  console.log(`${index + 1}. ${candidate}`);
  console.log(`   screen: ${screen.recommendation}`);
  console.log(`   threshold blockers: ${summarizeThresholdBlockers(screen.threshold_policy_blockers)}`);
  console.log(`   cooldown: ${summarizeCooldown(screen.cooldown_governance_rows, "20d")} | ${summarizeCooldown(screen.cooldown_governance_rows, "60d")}`);
  console.log(`   regime: ${summarizeThreshold(screen.regime_rows)}`);
  console.log(
    `   bundle: ${summarizeBundle(bundleSummary, 20)} | ${summarizeBundle(bundleSummary, 60)}`
  );
  if (review) {
    console.log(
      `   review: precision ${percent(actionablePrecision?.baseline)} -> ${percent(actionablePrecision?.candidate)} | timely ${percent(timelyWarningRate?.baseline)} -> ${percent(timelyWarningRate?.candidate)} | longest FP ${days(longestFalsePositive?.baseline)} -> ${days(longestFalsePositive?.candidate)} | runtime floor ${runtimeFloorHits?.baseline ?? "n/a"} -> ${runtimeFloorHits?.candidate ?? "n/a"}`
    );
  } else {
    console.log("   review: missing matching default release-review artifact");
  }
  for (const reason of shortList(screen.reasons, 4)) {
    console.log(`   - ${reason}`);
  }
  if (hardBlockers > 0) {
    const hardest = screen.threshold_policy_blockers.find((blocker) => blocker?.severity === "hard");
    if (hardest?.evidence) {
      console.log(`   - hard blocker: ${hardest.evidence}`);
    }
  }
  console.log("");
}

if (closestCandidate) {
  console.log("Conclusion:");
  console.log(
    `The least-bad recent candidate is ${closestCandidate.candidate}, but it is still not safe to activate. It improves some operational metrics while failing the core early-warning objective.`
  );
  for (const reason of shortList(closestCandidate.reasons, 3)) {
    console.log(`- ${reason}`);
  }
} else {
  console.log("Conclusion:");
  console.log("No recent reviewed candidate is safe to activate. Current work should stay on model repair, not release switching.");
}
