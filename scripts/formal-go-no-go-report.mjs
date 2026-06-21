import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const apiBaseUrl = process.env.FC_API_BASE_URL ?? "http://127.0.0.1:18080";
const cliArgs = process.argv.slice(2);
const outputPath = parseOutputPath(cliArgs);
const failOnNoGo = cliArgs.includes("--fail-on-no-go");

function parseOutputPath(args) {
  const outputIndex = args.findIndex((arg) => arg === "--output" || arg === "-o");
  return outputIndex >= 0 ? args[outputIndex + 1] ?? null : null;
}

function trimTrailingZeros(value) {
  return value.replace(/(?:\.0+|(\.\d*?[1-9])0+)$/, "$1");
}

function formatDate(value) {
  return typeof value === "string" && value.length > 0 ? value.slice(0, 10) : "-";
}

function formatDateTime(value) {
  if (typeof value !== "string" || value.length === 0) {
    return "-";
  }
  return value.replace("T", " ").replace(/\.\d+Z$/, " UTC").replace(/Z$/, " UTC");
}

function formatNumber(value, digits = 1) {
  return typeof value === "number" && Number.isFinite(value)
    ? trimTrailingZeros(value.toFixed(digits))
    : "-";
}

function formatPercent(value, digits = 1) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return "-";
  }
  return `${trimTrailingZeros((value * 100).toFixed(digits))}%`;
}

function asArray(value) {
  return Array.isArray(value) ? value : [];
}

function activeLikeReleaseStatus(value) {
  return ["active", "active_default", "active_experimental", "approved"].includes(
    String(value ?? "").toLowerCase()
  );
}

function productionSourceIssues(sources) {
  return sources.filter(
    (source) =>
      source?.production_allowed === true &&
      ["delayed", "partial_failure", "failed", "failing", "missing", "stale"].includes(
        source?.health?.status
      )
  );
}

function checkLine(check) {
  const marker = check.status === "pass" ? "x" : check.status === "warn" ? "~" : "!";
  return `- [${marker}] ${check.label}: ${check.detail}`;
}

function actionLine(action) {
  return `- ${action}`;
}

async function fetchJson(path) {
  const response = await fetch(`${apiBaseUrl}${path}`);
  if (!response.ok) {
    throw new Error(`GET ${path} failed: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

function addCheck(checks, status, label, detail, action) {
  checks.push({ action, detail, label, status });
}

async function collectState() {
  const [assessment, audit, sourcesResult] = await Promise.all([
    fetchJson("/api/assessment/current"),
    fetchJson("/api/research/audit"),
    fetchJson("/api/sources").catch((error) => ({ error: error?.message ?? String(error) }))
  ]);

  return {
    assessment,
    audit,
    sourceFetchError: sourcesResult?.error ?? null,
    sources: Array.isArray(sourcesResult) ? sourcesResult : []
  };
}

function evaluateGoNoGo(state) {
  const { assessment, audit, sourceFetchError, sources } = state;
  const checks = [];
  const sourceIssues = productionSourceIssues(sources);
  const probabilityMode = audit?.runtime_probability_mode ?? "unknown";
  const activeReleaseId = audit?.active_release_id ?? null;
  const servingStatus = audit?.runtime_release_status ?? "unknown";
  const snapshotAudit = audit?.prediction_snapshot_audit ?? {};
  const releaseReview = audit?.latest_release_review ?? null;
  const datasetSummaries = asArray(audit?.latest_dataset_summaries);
  const scenarioCoverageCatalog = releaseReview?.scenario_coverage_catalog ?? null;
  const probabilityInputStatus =
    assessment?.mvp_risk_state?.probability_input_status ?? "unknown";

  addCheck(
    checks,
    assessment?.runtime?.data_mode && assessment.runtime.data_mode !== "demo" ? "pass" : "fail",
    "Runtime data mode",
    `${assessment?.runtime?.data_mode ?? "unknown"}; latest key indicator ${formatDate(
      assessment?.runtime?.latest_key_indicator_at
    )}`,
    "Run the API on SQLite or Postgres production data before evaluating formal-model promotion."
  );

  addCheck(
    checks,
    assessment?.runtime?.stale_warning ? "fail" : "pass",
    "Runtime freshness",
    assessment?.runtime?.stale_warning ?? "clear",
    "Refresh or backfill key inputs until the runtime stale-warning guard is clear."
  );

  addCheck(
    checks,
    sourceFetchError ? "fail" : sourceIssues.length === 0 ? "pass" : "fail",
    "Production source health",
    sourceFetchError
      ? sourceFetchError
      : sourceIssues.length === 0
        ? "0 degraded production sources"
        : `${sourceIssues.length} degraded production sources`,
    "Resolve degraded production sources before relying on a formal release as active_default."
  );

  addCheck(
    checks,
    audit?.supported === true ? "pass" : "fail",
    "Research audit support",
    `supported=${audit?.supported === true}; storage=${audit?.storage_mode ?? "unknown"}`,
    "Use a runtime/storage mode that exposes release registry, snapshot, and research-audit evidence."
  );

  addCheck(
    checks,
    String(probabilityMode).includes("formal") ? "pass" : "fail",
    "Runtime probability mode",
    `${probabilityMode}; active_default requires a formal probability layer, not heuristic_mvp alone`,
    "Train, publish, review, and explicitly activate a formal release before treating probabilities as formal active_default."
  );

  addCheck(
    checks,
    activeReleaseId ? "pass" : "fail",
    "Active release id",
    activeReleaseId ?? "none",
    "Publish and activate a release in the registry so runtime output can be tied to a concrete artifact."
  );

  addCheck(
    checks,
    activeLikeReleaseStatus(servingStatus) ? "pass" : "fail",
    "Release serving status",
    servingStatus,
    "Do not promote a release unless the registry marks it active or approved for serving."
  );

  addCheck(
    checks,
    probabilityInputStatus === "reference_only" ? "fail" : "pass",
    "MVP probability input status",
    probabilityInputStatus,
    "Keep the main decision on rule-layer evidence while formal probabilities remain reference_only."
  );

  addCheck(
    checks,
    Number(snapshotAudit.active_release_snapshot_count ?? 0) > 0 &&
      Number(snapshotAudit.formal_probability_snapshot_count ?? 0) > 0
      ? "pass"
      : "fail",
    "Snapshot evidence for active formal release",
    `active=${snapshotAudit.active_release_snapshot_count ?? 0}; formal=${
      snapshotAudit.formal_probability_snapshot_count ?? 0
    }; heuristic=${snapshotAudit.heuristic_probability_snapshot_count ?? 0}`,
    "Generate and retain active-release formal prediction snapshots before relying on the release online."
  );

  addCheck(
    checks,
    datasetSummaries.length > 0 ? "pass" : "fail",
    "Formal dataset summary evidence",
    `${datasetSummaries.length} latest dataset summary artifact(s)`,
    "Run and retain a formal dataset summary so coverage, PIT mode, labels, and sample counts are inspectable."
  );

  addCheck(
    checks,
    releaseReview ? "pass" : "fail",
    "Latest release review evidence",
    releaseReview
      ? `${releaseReview.baseline_release_id} -> ${releaseReview.candidate_release_id}; history=${releaseReview.history_mode}`
      : "missing",
    "Run a strict release review for the candidate and retain the artifact before any active_default decision."
  );

  if (releaseReview) {
    addCheck(
      checks,
      releaseReview.overall_guard_passed === true ? "pass" : "fail",
      "Release review guard",
      `overall_guard_passed=${releaseReview.overall_guard_passed}; recommendation=${
        releaseReview.recommendation ?? "unknown"
      }`,
      "Fix release-review guard failures before promoting the candidate."
    );

    addCheck(
      checks,
      activeReleaseId && releaseReview.candidate_release_id === activeReleaseId ? "pass" : "fail",
      "Review proves current active release",
      activeReleaseId
        ? `active=${activeReleaseId}; reviewed_candidate=${releaseReview.candidate_release_id}`
        : `active=none; reviewed_candidate=${releaseReview.candidate_release_id}`,
      "Ensure the latest retained review proves the exact release currently proposed for active_default."
    );
  }

  if (scenarioCoverageCatalog) {
    const backtestCovered =
      Number(scenarioCoverageCatalog.covered_backtest_scenario_count ?? 0) >=
      Number(scenarioCoverageCatalog.backtest_scenario_count ?? 0);
    const focusCovered =
      Number(scenarioCoverageCatalog.covered_focus_scenario_count ?? 0) >=
      Number(scenarioCoverageCatalog.focus_scenario_count ?? 0);
    addCheck(
      checks,
      backtestCovered && focusCovered ? "pass" : "fail",
      "Scenario coverage for review",
      `backtest ${scenarioCoverageCatalog.covered_backtest_scenario_count ?? 0}/${
        scenarioCoverageCatalog.backtest_scenario_count ?? 0
      }; focus ${scenarioCoverageCatalog.covered_focus_scenario_count ?? 0}/${
        scenarioCoverageCatalog.focus_scenario_count ?? 0
      }`,
      "Close scenario coverage gaps before using release-review results as Go/No-Go evidence."
    );
  } else {
    addCheck(
      checks,
      "fail",
      "Scenario coverage for review",
      "missing",
      "Retain release-review scenario coverage so protected, main, and extension scenarios are auditable."
    );
  }

  addCheck(
    checks,
    audit?.latest_leadtime_audit ? "pass" : "fail",
    "Lead-time audit evidence",
    audit?.latest_leadtime_audit
      ? `${audit.latest_leadtime_audit.baseline_release_id} -> ${audit.latest_leadtime_audit.candidate_release_id}`
      : "missing",
    "Run a lead-time audit to prove the candidate does not lose actionable warning lead time."
  );

  addCheck(
    checks,
    audit?.latest_cooldown_audit ? "pass" : "warn",
    "Cooldown / false-positive audit evidence",
    audit?.latest_cooldown_audit
      ? `${audit.latest_cooldown_audit.baseline_release_id} -> ${audit.latest_cooldown_audit.candidate_release_id}`
      : "missing",
    "Run cooldown and false-positive audits before final human approval."
  );

  const hardFailures = checks.filter((check) => check.status === "fail");
  return {
    checks,
    hardFailures,
    ok: hardFailures.length === 0,
    sourceIssues
  };
}

function buildReport(state, evaluation) {
  const { assessment, audit } = state;
  const status = evaluation.ok ? "GO-FOR-HUMAN-APPROVAL" : "NO-GO";
  const uniqueActions = [
    ...new Set(evaluation.hardFailures.map((check) => check.action).filter(Boolean))
  ];
  const latestReview = audit?.latest_release_review ?? null;
  const lines = [
    "# Formal Model Go/No-Go Evidence Report",
    "",
    `- Status: ${status}`,
    `- API: ${apiBaseUrl}`,
    `- Generated: ${formatDateTime(new Date().toISOString())}`,
    `- Assessment as-of: ${formatDate(assessment?.as_of_date)}`,
    `- Runtime generated: ${formatDateTime(assessment?.runtime?.generated_at)}`,
    "",
    "## Current Runtime",
    "",
    `- Data mode: ${assessment?.runtime?.data_mode ?? "unknown"}`,
    `- Runtime probability mode: ${audit?.runtime_probability_mode ?? "unknown"}`,
    `- Active release: ${audit?.active_release_id ?? "none"}`,
    `- Release serving status: ${audit?.runtime_release_status ?? "unknown"}`,
    `- MVP probability input: ${
      assessment?.mvp_risk_state?.probability_input_status ?? "unknown"
    }`,
    `- Posture: ${assessment?.posture ?? "unknown"}`,
    `- Overall / structural / trigger / external: ${formatNumber(
      assessment?.scores?.overall_score
    )} / ${formatNumber(assessment?.scores?.structural_score)} / ${formatNumber(
      assessment?.scores?.trigger_score
    )} / ${formatNumber(assessment?.scores?.external_shock_score)}`,
    `- Probability reference values 5d / 20d / 60d: ${formatPercent(
      assessment?.probabilities?.p_5d,
      2
    )} / ${formatPercent(assessment?.probabilities?.p_20d, 2)} / ${formatPercent(
      assessment?.probabilities?.p_60d,
      2
    )}`,
    "",
    "## Evidence Summary",
    "",
    `- Releases in registry: ${asArray(audit?.releases).length}`,
    `- Replay runs: ${asArray(audit?.replay_runs).length}`,
    `- Formal dataset summaries: ${asArray(audit?.latest_dataset_summaries).length}`,
    `- Active-release snapshots: ${
      audit?.prediction_snapshot_audit?.active_release_snapshot_count ?? 0
    }`,
    `- Formal probability snapshots: ${
      audit?.prediction_snapshot_audit?.formal_probability_snapshot_count ?? 0
    }`,
    `- Latest release review: ${
      latestReview
        ? `${latestReview.baseline_release_id} -> ${latestReview.candidate_release_id}`
        : "missing"
    }`,
    "",
    "## Active-Default Checks",
    "",
    ...evaluation.checks.map(checkLine),
    "",
    "## Blocking Work",
    ""
  ];

  if (uniqueActions.length === 0) {
    lines.push(
      "- No automated hard blocker found. Human approval is still required before release activation."
    );
  } else {
    lines.push(...uniqueActions.map(actionLine));
  }

  lines.push(
    "",
    "## Interpretation",
    "",
    evaluation.ok
      ? "- The collected evidence is sufficient for a human Go/No-Go approval meeting. This script does not activate or roll back releases."
      : "- Current evidence does not justify treating formal probabilities as active_default. Keep the MVP decision on rule-layer evidence, data freshness, and operator review."
  );

  return {
    ok: evaluation.ok,
    text: `${lines.join("\n")}\n`
  };
}

try {
  const state = await collectState();
  const evaluation = evaluateGoNoGo(state);
  const report = buildReport(state, evaluation);
  if (outputPath) {
    const absoluteOutputPath = resolve(outputPath);
    await mkdir(dirname(absoluteOutputPath), { recursive: true });
    await writeFile(absoluteOutputPath, report.text, "utf8");
    console.log(`Formal Go/No-Go report written to ${absoluteOutputPath}`);
  } else {
    process.stdout.write(report.text);
  }
  if (failOnNoGo && !report.ok) {
    process.exitCode = 2;
  }
} catch (error) {
  console.error(`Formal Go/No-Go report failed: ${error?.message ?? error}`);
  process.exitCode = 1;
}
