import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const apiBaseUrl = process.env.FC_API_BASE_URL ?? "http://127.0.0.1:18080";
const cliArgs = process.argv.slice(2);
const outputPath = parseOutputPath(cliArgs);
const failOnIssues = cliArgs.includes("--fail-on-issues");

function parseOutputPath(args) {
  const outputIndex = args.findIndex((arg) => arg === "--output" || arg === "-o");
  if (outputIndex >= 0) {
    return args[outputIndex + 1] ?? null;
  }
  return process.env.FC_DAILY_HEALTH_REPORT_PATH ?? null;
}

function trimTrailingZeros(value) {
  return value.replace(/(?:\.0+|(\.\d*?[1-9])0+)$/, "$1");
}

function formatDate(value) {
  return typeof value === "string" && value.length > 0 ? value.slice(0, 10) : "—";
}

function formatDateTime(value) {
  if (typeof value !== "string" || value.length === 0) {
    return "—";
  }
  return value.replace("T", " ").replace(/\.\d+Z$/, " UTC").replace(/Z$/, " UTC");
}

function formatNumber(value, digits = 1) {
  return typeof value === "number" && Number.isFinite(value)
    ? trimTrailingZeros(value.toFixed(digits))
    : "—";
}

function formatPercent(value, digits = 0) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return "—";
  }
  return `${trimTrailingZeros((value * 100).toFixed(digits))}%`;
}

function formatBudgetPercent(value) {
  return typeof value === "number" && Number.isFinite(value)
    ? `${trimTrailingZeros(value.toFixed(1))}%`
    : "—";
}

function sourceStatusLabel(status) {
  const labels = {
    healthy: "健康",
    delayed: "延迟",
    stale: "陈旧",
    partial_failure: "部分失败",
    failed: "失败",
    failing: "失败",
    missing: "缺失",
    prototype: "原型",
    disabled: "停用"
  };
  return labels[status] ?? String(status ?? "未知");
}

function mvpStateLabel(snapshot) {
  const state = snapshot?.mvp_risk_state;
  return state?.display_label ?? state?.label ?? snapshot?.posture ?? "未知";
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

function sourceLastSuccessAt(source) {
  return source?.health?.last_success_at ?? null;
}

function minDateTime(values) {
  const timestamps = values
    .filter((value) => typeof value === "string" && value.length > 0)
    .map((value) => new Date(value))
    .filter((date) => Number.isFinite(date.getTime()));
  if (timestamps.length === 0) {
    return null;
  }
  return new Date(Math.min(...timestamps.map((date) => date.getTime()))).toISOString();
}

function maxDateTime(values) {
  const timestamps = values
    .filter((value) => typeof value === "string" && value.length > 0)
    .map((value) => new Date(value))
    .filter((date) => Number.isFinite(date.getTime()));
  if (timestamps.length === 0) {
    return null;
  }
  return new Date(Math.max(...timestamps.map((date) => date.getTime()))).toISOString();
}

async function fetchJson(path) {
  const response = await fetch(`${apiBaseUrl}${path}`);
  if (!response.ok) {
    throw new Error(`GET ${path} failed: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

function evaluateHealth(snapshot, sources) {
  const productionSources = sources.filter((source) => source?.production_allowed === true);
  const issues = productionSourceIssues(sources);
  const lastSuccessValues = productionSources.map(sourceLastSuccessAt);
  const oldestSuccess = minDateTime(lastSuccessValues);
  const newestSuccess = maxDateTime(lastSuccessValues);
  const probabilityStatus = snapshot?.mvp_risk_state?.probability_input_status ?? "unknown";
  const staleWarning = snapshot?.runtime?.stale_warning ?? null;
  const status = issues.length > 0 || staleWarning ? "attention" : "ok";
  return {
    issues,
    newestSuccess,
    oldestSuccess,
    probabilityStatus,
    staleWarning,
    status
  };
}

function buildReport(snapshot, sources, health) {
  const generatedAt = snapshot?.runtime?.generated_at;
  const lines = [
    "# Daily Health Report",
    "",
    `- Status: ${health.status === "ok" ? "OK" : "ATTENTION"}`,
    `- API: ${apiBaseUrl}`,
    `- Generated: ${formatDateTime(new Date().toISOString())}`,
    `- Assessment as-of: ${formatDate(snapshot?.as_of_date)}`,
    `- Runtime generated: ${formatDateTime(generatedAt)}`,
    `- Data mode: ${snapshot?.runtime?.data_mode ?? "—"}`,
    `- Latest key indicator: ${formatDate(snapshot?.runtime?.latest_key_indicator_at)}`,
    "",
    "## Decision Snapshot",
    "",
    `- MVP state: ${mvpStateLabel(snapshot)} (${health.probabilityStatus})`,
    `- Overall / structural / trigger / external: ${formatNumber(
      snapshot?.scores?.overall_score
    )} / ${formatNumber(snapshot?.scores?.structural_score)} / ${formatNumber(
      snapshot?.scores?.trigger_score
    )} / ${formatNumber(snapshot?.scores?.external_shock_score)}`,
    `- Probability reference values 5d / 20d / 60d: ${formatPercent(
      snapshot?.probabilities?.p_5d,
      2
    )} / ${formatPercent(snapshot?.probabilities?.p_20d, 2)} / ${formatPercent(
      snapshot?.probabilities?.p_60d,
      2
    )}`,
    `- Event confirmation / JPY carry: ${formatNumber(
      snapshot?.event_assessment?.confirmation_score
    )} / ${formatNumber(snapshot?.jpy_carry?.score)}`,
    `- Budget reference: risk assets ${formatBudgetPercent(
      snapshot?.position_guidance?.target_equity_exposure_pct
    )}, cash ${formatBudgetPercent(snapshot?.position_guidance?.target_cash_pct)}, hedge ${formatBudgetPercent(
      snapshot?.position_guidance?.hedge_ratio_pct
    )}, options ${formatBudgetPercent(snapshot?.position_guidance?.option_overlay_pct)}`,
    `- Manual confirmation items: ${
      Array.isArray(snapshot?.position_guidance?.manual_confirmation_items)
        ? snapshot.position_guidance.manual_confirmation_items.length
        : "—"
    }`,
    `- Inapplicable scenarios: ${
      Array.isArray(snapshot?.position_guidance?.inapplicable_scenarios)
        ? snapshot.position_guidance.inapplicable_scenarios.length
        : "—"
    }`,
    "",
    "## Data Health",
    "",
    `- Coverage grade: ${(snapshot?.data_trust?.quality_grade ?? "—").toString().toUpperCase()} (${formatPercent(
      snapshot?.data_trust?.coverage_score,
      1
    )} coverage)`,
    `- Production source issues: ${health.issues.length}`,
    `- Oldest production source success: ${formatDateTime(health.oldestSuccess)}`,
    `- Newest production source success: ${formatDateTime(health.newestSuccess)}`,
    `- API stale warning: ${health.staleWarning ?? "none"}`,
    "",
    "## Source Issues",
    ""
  ];

  if (health.issues.length === 0) {
    lines.push("- None");
  } else {
    for (const source of health.issues) {
      lines.push(
        `- ${source.display_name ?? source.source_id}: ${sourceStatusLabel(
          source?.health?.status
        )}; last success ${formatDateTime(sourceLastSuccessAt(source))}; failures ${
          source?.health?.consecutive_failures ?? "—"
        }; ${source?.health?.message ?? ""}`
      );
    }
  }

  lines.push(
    "",
    "## Operator Guidance",
    "",
    health.probabilityStatus === "reference_only"
      ? "- Formal probabilities are reference-only. Keep the main decision on rule-layer scores, key data freshness, event confirmation, and JPY carry."
      : "- Formal probabilities are usable, but still require data freshness and event confirmation before action escalation.",
    health.issues.length > 0
      ? "- Resolve degraded production sources before raising confidence or action posture."
      : "- No production source degradation detected. Continue daily refresh and monitor key indicators.",
    health.staleWarning
      ? "- Runtime stale warning is active; refresh or backfill data before relying on the panel."
      : "- Runtime freshness guard is clear."
  );

  return `${lines.join("\n")}\n`;
}

try {
  const [snapshot, sources] = await Promise.all([
    fetchJson("/api/assessment/current"),
    fetchJson("/api/sources")
  ]);
  const sourceRows = Array.isArray(sources) ? sources : [];
  const health = evaluateHealth(snapshot, sourceRows);
  const report = buildReport(snapshot, sourceRows, health);
  if (outputPath) {
    const absoluteOutputPath = resolve(outputPath);
    await mkdir(dirname(absoluteOutputPath), { recursive: true });
    await writeFile(absoluteOutputPath, report, "utf8");
    console.log(`Daily health report written to ${absoluteOutputPath}`);
  } else {
    process.stdout.write(report);
  }
  if (failOnIssues && health.status !== "ok") {
    process.exitCode = 2;
  }
} catch (error) {
  console.error(`Daily health report failed: ${error?.message ?? error}`);
  process.exitCode = 1;
}
