import { mkdir, readdir, readFile, stat, writeFile } from "node:fs/promises";
import { basename, dirname, join, resolve } from "node:path";

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

function formatInteger(value) {
  return typeof value === "number" && Number.isFinite(value)
    ? String(Math.round(value))
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

function unique(values) {
  return [...new Set(values.filter(Boolean))];
}

function healthyServingStatus(value) {
  return String(value ?? "").toLowerCase() === "healthy";
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

function auditMatchesReleaseReview(auditArtifact, releaseReview) {
  if (!auditArtifact || !releaseReview) {
    return false;
  }
  return (
    auditArtifact.baseline_release_id === releaseReview.baseline_release_id &&
    auditArtifact.candidate_release_id === releaseReview.candidate_release_id &&
    auditArtifact.history_mode === releaseReview.history_mode
  );
}

function auditTargetDetail(auditArtifact, releaseReview, extraParts = []) {
  if (!auditArtifact) {
    return "missing";
  }

  const parts = [
    `${auditArtifact.baseline_release_id} -> ${auditArtifact.candidate_release_id}`,
    `history=${auditArtifact.history_mode ?? "unknown"}`,
    ...extraParts
  ];

  if (releaseReview && !auditMatchesReleaseReview(auditArtifact, releaseReview)) {
    parts.push(
      `does not match latest release review ${releaseReview.baseline_release_id} -> ${releaseReview.candidate_release_id}; history=${releaseReview.history_mode ?? "unknown"}`
    );
  }

  return parts.join("; ");
}

function leadtimeAuditDecision(leadtimeAudit, releaseReview) {
  if (!leadtimeAudit) {
    return {
      action: "Run a lead-time audit to prove the candidate does not lose actionable warning lead time.",
      detail: "missing",
      status: "fail"
    };
  }

  if (releaseReview && !auditMatchesReleaseReview(leadtimeAudit, releaseReview)) {
    return {
      action:
        "Regenerate lead-time audit evidence for the exact latest release-review baseline, candidate, and history mode.",
      detail: auditTargetDetail(leadtimeAudit, releaseReview),
      status: "fail"
    };
  }

  return {
    action: "Run a lead-time audit to prove the candidate does not lose actionable warning lead time.",
    detail: auditTargetDetail(leadtimeAudit, releaseReview),
    status: "pass"
  };
}

function cooldownAuditDecision(cooldownAudit, releaseReview) {
  if (!cooldownAudit) {
    return {
      action: "Run cooldown and false-positive audits before final human approval.",
      detail: "missing",
      status: "warn"
    };
  }

  const recommendation = String(cooldownAudit.recommendation ?? "unknown");
  const noGoReasons = asArray(cooldownAudit.no_go_reasons);
  const reasonCodes = unique(noGoReasons.map((reason) => reason?.code));
  const detail = auditTargetDetail(cooldownAudit, releaseReview, [
    `recommendation=${recommendation}`,
    reasonCodes.length > 0
      ? `no_go_reasons=${reasonCodes.length} (${reasonCodes.join(", ")})`
      : "no_go_reasons=0"
  ]);

  if (releaseReview && !auditMatchesReleaseReview(cooldownAudit, releaseReview)) {
    return {
      action:
        "Regenerate cooldown / false-positive audit evidence for the exact latest release-review baseline, candidate, and history mode.",
      detail,
      status: "fail"
    };
  }

  if (recommendation.startsWith("no_go") || noGoReasons.length > 0) {
    return {
      action: "Fix cooldown / false-positive no-go reasons before promoting the candidate.",
      detail,
      status: "fail"
    };
  }

  if (recommendation.startsWith("manual_review")) {
    return {
      action: "Resolve manual false-positive episode review before final human approval.",
      detail,
      status: "warn"
    };
  }

  return {
    action: "Run cooldown and false-positive audits before final human approval.",
    detail,
    status: "pass"
  };
}

function checkLine(check) {
  const marker = check.status === "pass" ? "x" : check.status === "warn" ? "~" : "!";
  return `- [${marker}] ${check.label}: ${check.detail}`;
}

function actionLine(action) {
  return `- ${action}`;
}

function thresholdHitSummary(summary) {
  if (!summary) {
    return "-";
  }

  return [
    `pred=${formatInteger(summary.predicted_positive_count)}`,
    `tp=${formatInteger(summary.true_positive_count)}`,
    `early=${formatInteger(summary.early_warning_hit_count)}/${formatInteger(
      summary.early_warning_row_count
    )} (${formatPercent(summary.early_warning_hit_rate, 1)})`,
    `normal=${formatInteger(summary.normal_hit_count)}/${formatInteger(
      summary.normal_row_count
    )} (${formatPercent(summary.normal_hit_rate, 1)})`,
    `positive=${formatInteger(summary.positive_window_hit_count)}/${formatInteger(
      summary.positive_window_row_count
    )} (${formatPercent(summary.positive_window_hit_rate, 1)})`,
    `cooldown=${formatInteger(summary.cooldown_hit_count)}/${formatInteger(
      summary.cooldown_row_count
    )} (${formatPercent(summary.cooldown_hit_rate, 1)})`
  ].join("; ");
}

function repairCandidateSummary(diagnostics) {
  if (!diagnostics) {
    return "-";
  }

  const bestRejected = diagnostics.best_rejected_reason
    ? `; best_rejected=${diagnostics.best_rejected_reason}@${formatPercent(
        diagnostics.best_rejected_threshold,
        1
      )}, early=${formatPercent(
        diagnostics.best_rejected_early_warning_hit_rate,
        1
      )}, positive=${formatPercent(
        diagnostics.best_rejected_positive_window_hit_rate,
        1
      )}, normal=${formatPercent(
        diagnostics.best_rejected_normal_hit_rate,
        1
      )}, cooldown=${formatPercent(
        diagnostics.best_rejected_cooldown_hit_rate,
        1
      )}, pred=${formatInteger(diagnostics.best_rejected_predicted_positive_count)}`
    : "";

  return [
    `candidates=${formatInteger(diagnostics.candidate_count)}`,
    `accepted=${formatInteger(diagnostics.accepted_candidate_count)}`,
    `no_early=${formatInteger(diagnostics.rejected_no_early_warning_hit_count)}`,
    `regime_reject=${formatInteger(diagnostics.rejected_regime_support_count)}`,
    `no_positive=${formatInteger(diagnostics.rejected_no_positive_support_count)}`,
    `ceiling=${formatInteger(diagnostics.rejected_prediction_ceiling_count)}${bestRejected}`
  ].join("; ");
}

function regimeLiftSummary(regimeSummary) {
  if (!regimeSummary) {
    return "-";
  }

  return [
    `early=${formatNumber(regimeSummary.early_warning_lift_vs_normal, 3)}x`,
    `positive=${formatNumber(regimeSummary.positive_window_lift_vs_normal, 3)}x`,
    `cooldown=${formatNumber(regimeSummary.post_crisis_cooldown_lift_vs_normal, 3)}x`
  ].join("; ");
}

function thresholdNextStep(row) {
  if (!row.thresholdDiagnostic) {
    return "Regenerate the evaluation artifact with threshold diagnostics enabled.";
  }

  const candidates = row.repairCandidateDiagnostics;
  const earlyWarningLift = Number(row.regimeSummary?.early_warning_lift_vs_normal);
  if (row.splitMismatchDetail) {
    return "Inspect calibration/evaluation family and episode coverage before threshold tuning.";
  }

  if (row.repaired && Number.isFinite(earlyWarningLift) && earlyWarningLift < 1.5) {
    return "Early-warning lift is below the safety guardrail; improve model ranking or family coverage before relaxing thresholds.";
  }

  if (
    row.repaired &&
    candidates &&
    Number(candidates.accepted_candidate_count ?? 0) > 0
  ) {
    return "Accepted repair candidates exist; inspect guard ordering only after confirming lift and false-positive safety.";
  }

  if (
    row.repaired &&
    candidates &&
    Number(candidates.rejected_regime_support_count ?? 0) > 0
  ) {
    return "Improve model ranking or family coverage so early-warning hits beat normal/cooldown hits.";
  }

  if (
    row.repaired &&
    candidates &&
    Number(candidates.rejected_prediction_ceiling_count ?? 0) > 0
  ) {
    return "Reduce broad normal hits before considering any prediction-ceiling policy change.";
  }

  if (row.repaired) {
    return "Treat as fail-closed; improve training signal before promotion review.";
  }

  return "No threshold-specific blocker detected by generated candidate diagnostics.";
}

function generatedCandidateReportLines(candidate, evaluation) {
  if (!candidate) {
    return [
      "- Latest generated candidate: not found.",
      "- Interpretation: no research-only candidate evaluation artifact is available for this report."
    ];
  }

  const metrics = candidate.doc?.summary ?? {};
  const lines = [
    `- Release: ${candidate.releaseId}`,
    `- Evaluation artifact: ${candidate.path}`,
    `- Dataset: ${candidate.doc?.dataset_label ?? "unknown"}`,
    `- Model shape: ${candidate.doc?.model_family ?? "unknown"} / ${
      candidate.doc?.feature_transform ?? "unknown"
    }`,
    `- Status: ${evaluation.status}`,
    `- Metrics: brier=${formatNumber(metrics.brier_score, 4)}, log_loss=${formatNumber(
      metrics.log_loss,
      4
    )}, ece=${formatNumber(metrics.ece, 4)}`,
    `- Usable early-warning horizons: ${
      metrics.usable_early_warning_horizon_count ??
      metrics.usable_early_warning_horizons ??
      "unknown"
    }`,
    "",
    "| Horizon | Regime diagnosis | Base threshold | Final threshold | Repair | Reason |",
    "| --- | --- | ---: | ---: | --- | --- |",
    ...evaluation.horizonRows.map(
      (row) =>
        `| ${row.horizonDays}d | ${
          row.splitMismatchDetail ? `${row.diagnosis}; split mismatch` : row.diagnosis
        } | ${formatPercent(
          row.baseThreshold,
          1
        )} | ${formatPercent(row.finalThreshold, 1)} | ${
          row.repaired ? "yes" : "no"
        } | ${row.repairReason} |`
    )
  ];

  lines.push(
    "",
    "Threshold diagnostics:",
    "",
    "| Horizon | Base hits | Final hits | Regime lift | Repair candidates | Next step |",
    "| --- | --- | --- | --- | --- | --- |",
    ...evaluation.horizonRows.map(
      (row) =>
        `| ${row.horizonDays}d | ${thresholdHitSummary(
          row.baseSummary
        )} | ${thresholdHitSummary(row.finalSummary)} | ${regimeLiftSummary(
          row.regimeSummary
        )} | ${repairCandidateSummary(
          row.repairCandidateDiagnostics
        )} | ${row.nextStep} |`
    )
  );

  if (evaluation.blockers.length > 0) {
    lines.push("", "Candidate blockers:");
    lines.push(...evaluation.blockers.map(actionLine));
  } else {
    lines.push(
      "",
      "Candidate blockers: none detected by generated-evaluation checks. Release review and human approval are still required."
    );
  }

  if (evaluation.warnings?.length > 0) {
    lines.push("", "Candidate warnings:");
    lines.push(...evaluation.warnings.map(actionLine));
  }

  return lines;
}

async function fetchJson(path) {
  const response = await fetch(`${apiBaseUrl}${path}`);
  if (!response.ok) {
    throw new Error(`GET ${path} failed: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

async function pathExists(path) {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

function generatedCandidateDirs() {
  const cwd = process.cwd();
  const deployRoot =
    process.env.FC_DEPLOY_ROOT ??
    (basename(cwd) === "current" ? dirname(cwd) : null);
  return unique([
    process.env.FC_FORMAL_GENERATED_BUNDLE_DIR
      ? resolve(process.env.FC_FORMAL_GENERATED_BUNDLE_DIR)
      : null,
    resolve("artifacts/research/model-bundles/generated"),
    resolve("config/model-bundles/generated"),
    deployRoot ? resolve(deployRoot, "artifacts/research/model-bundles/generated") : null,
    deployRoot ? resolve(deployRoot, "config/model-bundles/generated") : null
  ]);
}

function releaseTimestamp(releaseId) {
  const match = String(releaseId ?? "").match(/(\d{8}T\d{6})/);
  if (!match) {
    return null;
  }
  return match[1];
}

async function loadLatestGeneratedCandidate() {
  const candidates = [];
  for (const directory of generatedCandidateDirs()) {
    if (!(await pathExists(directory))) {
      continue;
    }
    const entries = await readdir(directory);
    for (const entry of entries) {
      if (!entry.endsWith("-evaluation.json")) {
        continue;
      }
      const path = join(directory, entry);
      try {
        const [fileStat, raw] = await Promise.all([stat(path), readFile(path, "utf8")]);
        const doc = JSON.parse(raw);
        candidates.push({
          doc,
          mtimeMs: fileStat.mtimeMs,
          path,
          releaseId: doc?.release_id ?? entry.replace(/-evaluation\.json$/, ""),
          timestamp: releaseTimestamp(doc?.release_id ?? entry)
        });
      } catch {
        // Ignore partial or legacy generated files; the report should keep running.
      }
    }
  }

  return (
    candidates.sort((left, right) => {
      if (left.timestamp && right.timestamp && left.timestamp !== right.timestamp) {
        return right.timestamp.localeCompare(left.timestamp);
      }
      if (left.timestamp && !right.timestamp) {
        return -1;
      }
      if (!left.timestamp && right.timestamp) {
        return 1;
      }
      return right.mtimeMs - left.mtimeMs;
    })[0] ?? null
  );
}

function thresholdDiagnosticForHorizon(candidate, horizonDays) {
  return asArray(candidate?.doc?.horizons).find(
    (horizon) => Number(horizon?.horizon_days) === horizonDays
  )?.threshold_diagnostics;
}

function regimeSummaryForHorizon(candidate, horizonDays) {
  return asArray(candidate?.doc?.summary?.regime_separation_summaries).find(
    (summary) => Number(summary?.horizon_days) === horizonDays
  );
}

function thresholdSplitMismatchDetail(thresholdDiagnostic, regimeSummary) {
  if (!thresholdDiagnostic || !regimeSummary) {
    return null;
  }

  const calibrationEarlyHitRate = Number(
    thresholdDiagnostic.base_summary?.early_warning_hit_rate ?? 0
  );
  const evaluationEarlyAverage = Number(
    regimeSummary.early_warning_avg_probability ??
      regimeSummary.pre_warning_buffer_avg_probability ??
      0
  );
  const evaluationEarlyCount = Number(
    regimeSummary.early_warning_sample_count ??
      regimeSummary.pre_warning_buffer_sample_count ??
      0
  );
  const calibrationEarlyCount = Number(
    thresholdDiagnostic.base_summary?.early_warning_row_count ?? 0
  );

  if (
    calibrationEarlyCount > 0 &&
    evaluationEarlyCount > 0 &&
    calibrationEarlyHitRate === 0 &&
    evaluationEarlyAverage >= 0.8
  ) {
    return `calibration early-warning hit rate is ${formatPercent(
      calibrationEarlyHitRate,
      1
    )}, while evaluation early-warning average is ${formatPercent(
      evaluationEarlyAverage,
      1
    )}; inspect split/family coverage before tuning thresholds`;
  }

  return null;
}

function candidateThresholdWasRepaired(thresholdDiagnostic) {
  if (!thresholdDiagnostic) {
    return false;
  }
  return (
    thresholdDiagnostic.repair_applied === true ||
    thresholdDiagnostic.threshold_repaired === true ||
    Number(thresholdDiagnostic.final_threshold ?? 0) >= 0.99
  );
}

function usableGeneratedCandidateDiagnosis(diagnosis) {
  return (
    diagnosis === "usable_early_warning_separation" ||
    diagnosis === "usable_runtime_separation"
  );
}

function generatedCandidateHorizonIsCoreGate(horizonDays) {
  // 5d remains a conservative short-term/defend layer until there is enough
  // acute-crash coverage to make it a reliable formal promotion gate.
  return horizonDays === 20 || horizonDays === 60;
}

function evaluateLatestGeneratedCandidate(candidate, latestReview) {
  if (!candidate) {
    return {
      blockers: [],
      horizonRows: [],
      warnings: [],
      status: "missing"
    };
  }

  const blockers = [];
  const warnings = [];
  const latestReviewedTimestamp = releaseTimestamp(latestReview?.candidate_release_id);
  if (
    candidate.timestamp &&
    latestReviewedTimestamp &&
    candidate.timestamp < latestReviewedTimestamp
  ) {
    blockers.push(
      `Generated candidate ${candidate.releaseId} is older than latest reviewed candidate ${latestReview.candidate_release_id}.`
    );
  }

  const horizonRows = [5, 20, 60].map((horizonDays) => {
    const thresholdDiagnostic = thresholdDiagnosticForHorizon(candidate, horizonDays);
    const regimeSummary = regimeSummaryForHorizon(candidate, horizonDays) ?? {};
    const repaired = candidateThresholdWasRepaired(thresholdDiagnostic ?? {});
    const diagnosis = regimeSummary.diagnosis ?? "unknown";
    const repairReason = thresholdDiagnostic?.repair_reason ?? "missing";
    const splitMismatchDetail = thresholdSplitMismatchDetail(
      thresholdDiagnostic,
      regimeSummary
    );

    if (!thresholdDiagnostic) {
      blockers.push(`${horizonDays}d generated evaluation lacks threshold diagnostics.`);
    }
    if (repaired) {
      const message =
        `${horizonDays}d threshold remains fail-closed at ${formatPercent(
          thresholdDiagnostic?.final_threshold,
          1
        )}; reason=${repairReason}.`;
      if (generatedCandidateHorizonIsCoreGate(horizonDays)) {
        blockers.push(message);
      } else {
        warnings.push(`${message} Treated as conservative short-term advisory.`);
      }
    }
    if (diagnosis === "unknown") {
      blockers.push(`${horizonDays}d generated evaluation lacks regime separation diagnostics.`);
    } else if (!usableGeneratedCandidateDiagnosis(diagnosis)) {
      const message = `${horizonDays}d regime separation is ${diagnosis}.`;
      if (generatedCandidateHorizonIsCoreGate(horizonDays)) {
        blockers.push(message);
      } else {
        warnings.push(`${message} Treated as conservative short-term advisory.`);
      }
    }
    if (splitMismatchDetail) {
      blockers.push(`${horizonDays}d calibration/evaluation split mismatch: ${splitMismatchDetail}.`);
    }

    return {
      baseThreshold: thresholdDiagnostic?.base_threshold,
      baseSummary: thresholdDiagnostic?.base_summary,
      diagnosis,
      finalThreshold: thresholdDiagnostic?.final_threshold,
      finalSummary: thresholdDiagnostic?.final_summary,
      horizonDays,
      repaired,
      repairCandidateDiagnostics: thresholdDiagnostic?.repair_candidate_diagnostics,
      repairReason,
      regimeSummary,
      splitMismatchDetail,
      thresholdDiagnostic,
      nextStep: thresholdNextStep({
        repairCandidateDiagnostics: thresholdDiagnostic?.repair_candidate_diagnostics,
        regimeSummary,
        repaired,
        splitMismatchDetail,
        thresholdDiagnostic
      })
    };
  });

  const usableCount =
    candidate.doc?.summary?.usable_early_warning_horizon_count ??
    candidate.doc?.summary?.usable_early_warning_horizons;
  if (typeof usableCount === "number" && usableCount < 2) {
    blockers.push(
      `Only ${usableCount} early-warning horizon(s) are usable; require at least 2 before promotion review.`
    );
  }

  return {
    blockers: unique(blockers),
    horizonRows,
    warnings: unique(warnings),
    status: blockers.length === 0 ? "reviewable" : "blocked"
  };
}

function addCheck(checks, status, label, detail, action) {
  checks.push({ action, detail, label, status });
}

async function collectState() {
  const latestGeneratedCandidate = await loadLatestGeneratedCandidate();
  const [assessment, audit, sourcesResult] = await Promise.all([
    fetchJson("/api/assessment/current"),
    fetchJson("/api/research/audit"),
    fetchJson("/api/sources").catch((error) => ({ error: error?.message ?? String(error) }))
  ]);

  return {
    assessment,
    audit,
    latestGeneratedCandidate,
    sourceFetchError: sourcesResult?.error ?? null,
    sources: Array.isArray(sourcesResult) ? sourcesResult : []
  };
}

function evaluateGoNoGo(state) {
  const { assessment, audit, latestGeneratedCandidate, sourceFetchError, sources } = state;
  const checks = [];
  const latestReview = audit?.latest_release_review ?? null;
  const latestCandidateEvaluation = evaluateLatestGeneratedCandidate(
    latestGeneratedCandidate,
    latestReview
  );
  const sourceIssues = productionSourceIssues(sources);
  const probabilityMode = audit?.runtime_probability_mode ?? "unknown";
  const activeReleaseId = audit?.active_release_id ?? null;
  const servingStatus = audit?.runtime_release_status ?? "unknown";
  const snapshotAudit = audit?.prediction_snapshot_audit ?? {};
  const releaseReview = latestReview;
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
    healthyServingStatus(servingStatus) ? "pass" : "fail",
    "Release serving status",
    servingStatus,
    "Do not promote a release unless the runtime serving status is healthy."
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

  const leadtimeDecision = leadtimeAuditDecision(audit?.latest_leadtime_audit, releaseReview);
  addCheck(
    checks,
    leadtimeDecision.status,
    "Lead-time audit evidence",
    leadtimeDecision.detail,
    leadtimeDecision.action
  );

  const cooldownDecision = cooldownAuditDecision(audit?.latest_cooldown_audit, releaseReview);
  addCheck(
    checks,
    cooldownDecision.status,
    "Cooldown / false-positive audit evidence",
    cooldownDecision.detail,
    cooldownDecision.action
  );

  if (latestGeneratedCandidate) {
    addCheck(
      checks,
      latestCandidateEvaluation.blockers.length === 0 ? "pass" : "fail",
      "Latest generated formal candidate",
      `${latestGeneratedCandidate.releaseId}; status=${latestCandidateEvaluation.status}; blockers=${latestCandidateEvaluation.blockers.length}`,
      "Keep the generated formal candidate research-only until threshold repair, regime separation, release review, lead-time, and cooldown audits all pass."
    );
  } else {
    addCheck(
      checks,
      "warn",
      "Latest generated formal candidate",
      "not found in generated bundle directories",
      "Run research-only formal training and retain the generated evaluation artifact before candidate promotion review."
    );
  }

  const hardFailures = checks.filter((check) => check.status === "fail");
  return {
    checks,
    hardFailures,
    latestCandidateEvaluation,
    ok: hardFailures.length === 0,
    sourceIssues
  };
}

function buildReport(state, evaluation) {
  const { assessment, audit, latestGeneratedCandidate } = state;
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
    `- Latest generated candidate: ${
      latestGeneratedCandidate
        ? `${latestGeneratedCandidate.releaseId} (${latestGeneratedCandidate.path})`
        : "not found"
    }`,
    "",
    "## Active-Default Checks",
    "",
    ...evaluation.checks.map(checkLine),
    "",
    "## Latest Generated Candidate",
    "",
    ...generatedCandidateReportLines(latestGeneratedCandidate, evaluation.latestCandidateEvaluation),
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
