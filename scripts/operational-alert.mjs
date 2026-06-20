import { readFile } from "node:fs/promises";
import { basename, resolve } from "node:path";

const args = process.argv.slice(2);
const options = parseArgs(args);
const dryRun = options.dryRun || process.env.FC_ALERT_DRY_RUN === "1";
const failOnSendError =
  options.failOnSendError || process.env.FC_ALERT_FAIL_ON_ERROR === "1";

function parseArgs(values) {
  const parsed = {
    dryRun: false,
    failOnSendError: false,
    message: "",
    mode: "operational",
    reports: [],
    status: "attention"
  };
  for (let index = 0; index < values.length; index += 1) {
    const arg = values[index];
    switch (arg) {
      case "--mode":
        parsed.mode = values[++index] ?? parsed.mode;
        break;
      case "--status":
        parsed.status = values[++index] ?? parsed.status;
        break;
      case "--message":
        parsed.message = values[++index] ?? parsed.message;
        break;
      case "--report":
        parsed.reports.push(values[++index]);
        break;
      case "--dry-run":
        parsed.dryRun = true;
        break;
      case "--fail-on-send-error":
        parsed.failOnSendError = true;
        break;
      default:
        throw new Error(`Unknown argument: ${arg}`);
    }
  }
  return parsed;
}

function configuredDestinations() {
  return [
    {
      kind: "generic",
      url: process.env.FC_ALERT_WEBHOOK_URL
    },
    {
      kind: "slack",
      url: process.env.FC_ALERT_SLACK_WEBHOOK_URL
    },
    {
      kind: "feishu",
      url: process.env.FC_ALERT_FEISHU_WEBHOOK_URL
    },
    {
      kind: "dingtalk",
      url: process.env.FC_ALERT_DINGTALK_WEBHOOK_URL
    }
  ].filter((destination) => Boolean(destination.url));
}

function trimReport(text) {
  const maxChars = Number.parseInt(process.env.FC_ALERT_REPORT_MAX_CHARS ?? "3500", 10);
  const limit = Number.isFinite(maxChars) && maxChars > 0 ? maxChars : 3500;
  if (text.length <= limit) {
    return text;
  }
  return `${text.slice(0, limit)}\n...`;
}

function titleFor(status, mode) {
  const normalized = String(status || "attention").toUpperCase();
  return `[financial-crisis] ${normalized} ${mode}`;
}

async function readReports(paths) {
  const reports = [];
  for (const reportPath of paths.filter(Boolean)) {
    const absolutePath = resolve(reportPath);
    const text = await readFile(absolutePath, "utf8");
    reports.push({
      name: basename(absolutePath),
      path: absolutePath,
      text,
      excerpt: trimReport(text)
    });
  }
  return reports;
}

function plainTextMessage(payload) {
  const reportLines = payload.reports
    .map((report) => [`Report: ${report.path}`, report.excerpt].join("\n"))
    .join("\n\n");
  return [
    payload.title,
    `Mode: ${payload.mode}`,
    `Status: ${payload.status}`,
    `Generated: ${payload.generated_at}`,
    payload.message ? `Message: ${payload.message}` : null,
    reportLines || "No report file was attached."
  ]
    .filter(Boolean)
    .join("\n\n");
}

function bodyFor(destination, payload) {
  const text = plainTextMessage(payload);
  switch (destination.kind) {
    case "slack":
      return { text };
    case "feishu":
      return {
        msg_type: "text",
        content: { text }
      };
    case "dingtalk":
      return {
        msgtype: "text",
        text: { content: text }
      };
    default:
      return payload;
  }
}

async function postJson(destination, payload) {
  const timeoutMs = Number.parseInt(process.env.FC_ALERT_TIMEOUT_MS ?? "10000", 10);
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), Number.isFinite(timeoutMs) ? timeoutMs : 10000);
  const headers = {
    "content-type": "application/json"
  };
  if (destination.kind === "generic" && process.env.FC_ALERT_WEBHOOK_BEARER_TOKEN) {
    headers.authorization = `Bearer ${process.env.FC_ALERT_WEBHOOK_BEARER_TOKEN}`;
  }
  try {
    const response = await fetch(destination.url, {
      body: JSON.stringify(bodyFor(destination, payload)),
      headers,
      method: "POST",
      signal: controller.signal
    });
    if (!response.ok) {
      throw new Error(`${destination.kind} webhook returned ${response.status} ${response.statusText}`);
    }
  } finally {
    clearTimeout(timer);
  }
}

try {
  const destinations = configuredDestinations();
  const reports = await readReports(options.reports);
  const payload = {
    generated_at: new Date().toISOString(),
    message: options.message,
    mode: options.mode,
    reports,
    service: "financial-crisis",
    status: options.status,
    title: titleFor(options.status, options.mode)
  };

  if (dryRun) {
    console.log(
      JSON.stringify(
        {
          dry_run: true,
          destinations: destinations.map((destination) => destination.kind),
          payload
        },
        null,
        2
      )
    );
    process.exit(0);
  }

  if (destinations.length === 0) {
    console.log("Operational alert skipped: no alert destination configured.");
    process.exit(0);
  }

  const failures = [];
  for (const destination of destinations) {
    try {
      await postJson(destination, payload);
      console.log(`Operational alert sent via ${destination.kind}.`);
    } catch (error) {
      failures.push(error?.message ?? String(error));
      console.error(`Operational alert failed via ${destination.kind}: ${error?.message ?? error}`);
    }
  }

  if (failures.length > 0 && failOnSendError) {
    process.exitCode = 2;
  }
} catch (error) {
  console.error(`Operational alert failed: ${error?.message ?? error}`);
  process.exitCode = 1;
}
