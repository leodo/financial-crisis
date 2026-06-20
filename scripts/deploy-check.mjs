import { mkdir, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";

const apiBaseUrl = process.env.FC_API_BASE_URL ?? "http://127.0.0.1:18080";
const webBaseUrl = process.env.FC_WEB_BASE_URL ?? "http://127.0.0.1:5173";
const cliArgs = process.argv.slice(2);
const outputPath = parseOutputPath(cliArgs);
const failOnIssues = cliArgs.includes("--fail-on-issues");
const skipWeb = cliArgs.includes("--skip-web");
const allowDemo = process.env.FC_ALLOW_DEMO === "1" || cliArgs.includes("--allow-demo");

function parseOutputPath(args) {
  const outputIndex = args.findIndex((arg) => arg === "--output" || arg === "-o");
  return outputIndex >= 0 ? args[outputIndex + 1] ?? null : null;
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

function trimTrailingZeros(value) {
  return value.replace(/(?:\.0+|(\.\d*?[1-9])0+)$/, "$1");
}

function formatNumber(value, digits = 1) {
  return typeof value === "number" && Number.isFinite(value)
    ? trimTrailingZeros(value.toFixed(digits))
    : "—";
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
  const mark = check.ok ? "x" : "!";
  return `- [${mark}] ${check.label}: ${check.detail}`;
}

async function fetchJson(url, label) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${label} failed: ${response.status} ${response.statusText}`);
  }
  return response.json();
}

async function fetchText(url, label) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${label} failed: ${response.status} ${response.statusText}`);
  }
  return response.text();
}

async function collectDeploymentState() {
  const checks = [];

  const health = await fetchJson(`${apiBaseUrl}/health`, "API health");
  checks.push({
    ok: true,
    label: "API /health",
    detail: `responded with ${JSON.stringify(health)}`
  });

  let webTitle = "skipped";
  if (!skipWeb) {
    const webHtml = await fetchText(webBaseUrl, "Web root");
    webTitle = webHtml.match(/<title>(.*?)<\/title>/i)?.[1] ?? "HTML returned";
    checks.push({
      ok: webHtml.includes("<html") || webHtml.includes("<!doctype html"),
      label: "Web root",
      detail: `${webBaseUrl} served ${webTitle}`
    });
  } else {
    checks.push({
      ok: true,
      label: "Web root",
      detail: "skipped by --skip-web"
    });
  }

  const [assessment, sources] = await Promise.all([
    fetchJson(`${apiBaseUrl}/api/assessment/current`, "Assessment"),
    fetchJson(`${apiBaseUrl}/api/sources`, "Sources")
  ]);
  const sourceRows = Array.isArray(sources) ? sources : [];
  const sourceIssues = productionSourceIssues(sourceRows);
  const dataMode = assessment?.runtime?.data_mode ?? "unknown";
  const staleWarning = assessment?.runtime?.stale_warning ?? null;
  const latestKey = assessment?.runtime?.latest_key_indicator_at ?? null;
  const mvpStatus = assessment?.mvp_risk_state?.probability_input_status ?? "unknown";

  checks.push({
    ok: allowDemo || dataMode !== "demo",
    label: "Data mode",
    detail: `${dataMode}${allowDemo ? " (demo allowed)" : ""}`
  });
  checks.push({
    ok: typeof latestKey === "string" && latestKey.length > 0,
    label: "Latest key indicator",
    detail: formatDate(latestKey)
  });
  checks.push({
    ok: !staleWarning,
    label: "Runtime freshness",
    detail: staleWarning ?? "clear"
  });
  checks.push({
    ok: sourceIssues.length === 0,
    label: "Production source health",
    detail:
      sourceIssues.length === 0
        ? "0 degraded production sources"
        : `${sourceIssues.length} degraded: ${sourceIssues
            .map((source) => source.display_name ?? source.source_id)
            .join(", ")}`
  });

  return {
    assessment,
    checks,
    dataMode,
    latestKey,
    mvpStatus,
    sourceIssues,
    staleWarning,
    webTitle
  };
}

function buildReport(state) {
  const hardFailures = state.checks.filter((check) => !check.ok);
  const status = hardFailures.length === 0 ? "OK" : "ATTENTION";
  const report = [
    "# Deployment Check",
    "",
    `- Status: ${status}`,
    `- API: ${apiBaseUrl}`,
    `- Web: ${skipWeb ? "skipped" : webBaseUrl}`,
    `- Generated: ${formatDateTime(new Date().toISOString())}`,
    `- Assessment as-of: ${formatDate(state.assessment?.as_of_date)}`,
    `- Runtime generated: ${formatDateTime(state.assessment?.runtime?.generated_at)}`,
    "",
    "## Runtime Summary",
    "",
    `- Data mode: ${state.dataMode}`,
    `- Latest key indicator: ${formatDate(state.latestKey)}`,
    `- MVP probability input: ${state.mvpStatus}`,
    `- Overall score: ${formatNumber(state.assessment?.scores?.overall_score)}`,
    `- Production source issues: ${state.sourceIssues.length}`,
    "",
    "## Checks",
    "",
    ...state.checks.map(checkLine)
  ];

  if (state.sourceIssues.length > 0) {
    report.push("", "## Source Issues", "");
    for (const source of state.sourceIssues) {
      report.push(
        `- ${source.display_name ?? source.source_id}: ${source?.health?.status ?? "unknown"}; ${
          source?.health?.message ?? ""
        }`
      );
    }
  }

  report.push(
    "",
    "## Next Action",
    "",
    hardFailures.length === 0
      ? "- Deployment check passed. Continue daily refresh monitoring and keep the latest report for audit trail."
      : "- Deployment check needs attention. Fix failed checks before treating the panel as operational."
  );

  return {
    ok: hardFailures.length === 0,
    text: `${report.join("\n")}\n`
  };
}

try {
  const state = await collectDeploymentState();
  const report = buildReport(state);
  if (outputPath) {
    const absoluteOutputPath = resolve(outputPath);
    await mkdir(dirname(absoluteOutputPath), { recursive: true });
    await writeFile(absoluteOutputPath, report.text, "utf8");
    console.log(`Deployment check written to ${absoluteOutputPath}`);
  } else {
    process.stdout.write(report.text);
  }
  if (failOnIssues && !report.ok) {
    process.exitCode = 2;
  }
} catch (error) {
  console.error(`Deployment check failed: ${error?.message ?? error}`);
  process.exitCode = 1;
}
