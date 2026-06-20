import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const apiBaseUrl = process.env.FC_API_BASE_URL ?? "http://127.0.0.1:18080";
const args = process.argv.slice(2);
const outputPath = parseOutputPath(args);
const failOnAlert = args.includes("--fail-on-alert");
const dryRun = args.includes("--dry-run") || process.env.FC_ALERT_DRY_RUN === "1";

function parseOutputPath(values) {
  const outputIndex = values.findIndex((arg) => arg === "--output" || arg === "-o");
  return outputIndex >= 0 ? values[outputIndex + 1] ?? null : null;
}

function numberEnv(name, fallback) {
  const value = Number.parseFloat(process.env[name] ?? "");
  return Number.isFinite(value) ? value : fallback;
}

function boolEnv(name, fallback = false) {
  const value = process.env[name];
  if (value === undefined) {
    return fallback;
  }
  return ["1", "true", "yes", "on"].includes(value.toLowerCase());
}

function postureRank(posture) {
  const ranks = {
    normal: 0,
    observe: 0,
    prepare: 1,
    hedge: 2,
    defend: 3
  };
  return ranks[String(posture ?? "").toLowerCase()] ?? 0;
}

function formatDate(value) {
  return typeof value === "string" && value.length > 0 ? value.slice(0, 10) : "-";
}

function formatNumber(value, digits = 1) {
  return typeof value === "number" && Number.isFinite(value) ? value.toFixed(digits) : "-";
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

function thresholds(apiConfig = null) {
  const apiThresholds = apiConfig ?? {};
  return {
    alertOnReferenceOnly: boolEnv(
      "FC_RISK_ALERT_ON_REFERENCE_ONLY",
      Boolean(apiThresholds.alert_on_reference_only ?? false)
    ),
    maxProductionSourceIssues: numberEnv(
      "FC_RISK_ALERT_MAX_SOURCE_ISSUES",
      Number(apiThresholds.max_production_source_issues ?? 0)
    ),
    overallScore: numberEnv(
      "FC_RISK_ALERT_OVERALL_SCORE",
      Number(apiThresholds.overall_score ?? 55)
    ),
    posture: process.env.FC_RISK_ALERT_MIN_POSTURE ?? apiThresholds.min_posture ?? "prepare",
    source: apiThresholds.source ?? "script_default",
    triggerScore: numberEnv(
      "FC_RISK_ALERT_TRIGGER_SCORE",
      Number(apiThresholds.trigger_score ?? 60)
    )
  };
}

async function fetchJson(path) {
  const response = await fetch(`${apiBaseUrl}${path}`);
  if (!response.ok) {
    throw new Error(`GET ${path} failed: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

function evaluate(snapshot, sources, config) {
  const alerts = [];
  const sourceIssues = productionSourceIssues(sources);
  const posture = snapshot?.mvp_risk_state?.code ?? snapshot?.posture;
  const probabilityStatus = snapshot?.mvp_risk_state?.probability_input_status ?? "unknown";

  if ((snapshot?.scores?.overall_score ?? 0) >= config.overallScore) {
    alerts.push(
      `总风险分 ${formatNumber(snapshot.scores.overall_score)} >= 阈值 ${formatNumber(
        config.overallScore
      )}`
    );
  }
  if ((snapshot?.scores?.trigger_score ?? 0) >= config.triggerScore) {
    alerts.push(
      `触发压力分 ${formatNumber(snapshot.scores.trigger_score)} >= 阈值 ${formatNumber(
        config.triggerScore
      )}`
    );
  }
  if (postureRank(posture) >= postureRank(config.posture)) {
    alerts.push(`MVP/动作档位 ${posture} >= 阈值 ${config.posture}`);
  }
  if (snapshot?.runtime?.stale_warning) {
    alerts.push(`runtime stale warning: ${snapshot.runtime.stale_warning}`);
  }
  if (sourceIssues.length > config.maxProductionSourceIssues) {
    alerts.push(`生产源降级 ${sourceIssues.length} > 阈值 ${config.maxProductionSourceIssues}`);
  }
  if (config.alertOnReferenceOnly && probabilityStatus === "reference_only") {
    alerts.push("正式概率仍为 reference_only，只能按规则层和关键数据解释主结论");
  }

  return {
    alerts,
    posture,
    probabilityStatus,
    sourceIssues
  };
}

function buildReport(snapshot, evaluation, config) {
  const status = evaluation.alerts.length > 0 ? "ATTENTION" : "OK";
  const lines = [
    "# Risk Threshold Alert",
    "",
    `- Status: ${status}`,
    `- API: ${apiBaseUrl}`,
    `- Generated: ${new Date().toISOString()}`,
    `- Assessment as-of: ${formatDate(snapshot?.as_of_date)}`,
    `- MVP state: ${snapshot?.mvp_risk_state?.label ?? evaluation.posture ?? "-"}`,
    `- Probability input: ${evaluation.probabilityStatus}`,
    `- Overall / trigger: ${formatNumber(snapshot?.scores?.overall_score)} / ${formatNumber(
      snapshot?.scores?.trigger_score
    )}`,
    `- Latest key indicator: ${formatDate(snapshot?.runtime?.latest_key_indicator_at)}`,
    "",
    "## Thresholds",
    "",
    `- Overall score >= ${formatNumber(config.overallScore)}`,
    `- Trigger score >= ${formatNumber(config.triggerScore)}`,
    `- Min posture: ${config.posture}`,
    `- Max production source issues: ${config.maxProductionSourceIssues}`,
    `- Alert on reference_only: ${config.alertOnReferenceOnly}`,
    `- Threshold source: ${config.source} + FC_RISK_ALERT_* overrides`,
    "",
    "## Alerts",
    ""
  ];

  if (evaluation.alerts.length === 0) {
    lines.push("- None");
  } else {
    lines.push(...evaluation.alerts.map((alert) => `- ${alert}`));
  }

  lines.push(
    "",
    "## Operator Guidance",
    "",
    "- This is a reminder only, not an automatic trading instruction.",
    "- Check data freshness, source health, liquidity, tax/account constraints, and the action playbook before acting."
  );

  return `${lines.join("\n")}\n`;
}

function sendAlert(reportPath, status) {
  const message =
    status === "attention"
      ? "Business risk threshold alert triggered."
      : "Business risk threshold check passed.";
  const argsForAlert = [
    "./scripts/operational-alert.mjs",
    "--mode",
    "risk-threshold",
    "--status",
    status,
    "--message",
    message,
    "--report",
    reportPath
  ];
  if (dryRun) {
    argsForAlert.push("--dry-run");
  }
  const result = spawnSync("node", argsForAlert, {
    encoding: "utf8",
    stdio: "inherit"
  });
  return result.status ?? 1;
}

try {
  const [snapshot, sources, apiThresholds] = await Promise.all([
    fetchJson("/api/assessment/current"),
    fetchJson("/api/sources"),
    fetchJson("/api/system/risk-thresholds").catch((error) => {
      console.warn(`Risk threshold config endpoint unavailable, using script defaults: ${error.message}`);
      return null;
    })
  ]);
  const config = thresholds(apiThresholds);
  const sourceRows = Array.isArray(sources) ? sources : [];
  const evaluation = evaluate(snapshot, sourceRows, config);
  const report = buildReport(snapshot, evaluation, config);
  const status = evaluation.alerts.length > 0 ? "attention" : "ok";

  let reportPath = outputPath;
  if (reportPath) {
    const absoluteOutputPath = resolve(reportPath);
    await mkdir(dirname(absoluteOutputPath), { recursive: true });
    await writeFile(absoluteOutputPath, report, "utf8");
    console.log(`Risk threshold report written to ${absoluteOutputPath}`);
    reportPath = absoluteOutputPath;
  } else {
    process.stdout.write(report);
  }

  if ((evaluation.alerts.length > 0 || process.env.FC_ALERT_ON_SUCCESS === "1") && reportPath) {
    const alertExitCode = sendAlert(reportPath, status);
    if (alertExitCode !== 0) {
      process.exitCode = alertExitCode;
    }
  }

  if (failOnAlert && evaluation.alerts.length > 0) {
    process.exitCode = process.exitCode || 2;
  }
} catch (error) {
  console.error(`Risk threshold alert failed: ${error?.message ?? error}`);
  process.exitCode = 1;
}
