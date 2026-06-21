import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";

const DEFAULT_MARKET_SCOPE = "financial_system";
const DEFAULT_OUTPUT_DIR = "artifacts/research/dataset-summary-check";
const DEFAULT_DATASET_IDS = [
  "formal_v1_main_1990_daily",
  "formal_v1_ext_stress_1990_daily",
  "formal_v1_ext_acute_pre1990"
];

const cliArgs = process.argv.slice(2);
const options = parseArgs(cliArgs);

function parseArgs(args) {
  const parsed = {
    marketScope: process.env.FC_MARKET_SCOPE ?? DEFAULT_MARKET_SCOPE,
    outputDir: process.env.FC_FORMAL_DATASET_SUMMARY_OUTPUT_DIR ?? DEFAULT_OUTPUT_DIR,
    datasetIds: [...DEFAULT_DATASET_IDS],
    workerBin: process.env.FC_WORKER_BIN ?? null,
    cargoBin: process.env.CARGO ?? "cargo",
    quiet: false
  };

  for (let index = 0; index < args.length; index += 1) {
    const arg = args[index];
    if (arg === "--market-scope") {
      parsed.marketScope = requireValue(args, ++index, arg);
    } else if (arg === "--output-dir") {
      parsed.outputDir = requireValue(args, ++index, arg);
    } else if (arg === "--dataset-id") {
      parsed.datasetIds.push(requireValue(args, ++index, arg));
    } else if (arg === "--only-dataset-ids") {
      parsed.datasetIds = requireValue(args, ++index, arg)
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean);
    } else if (arg === "--worker-bin") {
      parsed.workerBin = requireValue(args, ++index, arg);
    } else if (arg === "--cargo-bin") {
      parsed.cargoBin = requireValue(args, ++index, arg);
    } else if (arg === "--quiet") {
      parsed.quiet = true;
    } else if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    } else {
      throw new Error(`unknown formal dataset summary pack option: ${arg}`);
    }
  }

  parsed.datasetIds = [...new Set(parsed.datasetIds)];
  if (parsed.datasetIds.length === 0) {
    throw new Error("at least one --dataset-id or --only-dataset-ids value is required");
  }
  return parsed;
}

function requireValue(args, index, flag) {
  const value = args[index];
  if (typeof value !== "string" || value.length === 0 || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

function printHelp() {
  console.log(`Usage: node scripts/formal-dataset-summary-pack.mjs [options]

Exports the latest persisted formal dataset summaries for Go/No-Go evidence.

Options:
  --market-scope SCOPE          Market scope, default ${DEFAULT_MARKET_SCOPE}
  --output-dir DIR             Output directory, default ${DEFAULT_OUTPUT_DIR}
  --dataset-id ID              Add one dataset id to export
  --only-dataset-ids A,B,C     Replace the default dataset id set
  --worker-bin PATH            Use a built fc-worker binary instead of cargo run
  --cargo-bin PATH             Cargo binary when --worker-bin is not set
  --quiet                      Do not echo child command output
`);
}

function workerCommand(args) {
  if (options.workerBin) {
    return {
      command: options.workerBin,
      args
    };
  }
  return {
    command: options.cargoBin,
    args: ["run", "-q", "-p", "fc-worker", "--", ...args]
  };
}

async function runWorker(args) {
  const { command, args: commandArgs } = workerCommand(args);
  return new Promise((resolve, reject) => {
    const child = spawn(command, commandArgs, {
      cwd: process.cwd(),
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
      shell: false
    });
    let stdout = "";
    let stderr = "";
    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      stdout += chunk;
      if (!options.quiet) {
        process.stdout.write(chunk);
      }
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk;
      if (!options.quiet) {
        process.stderr.write(chunk);
      }
    });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve({ stdout, stderr });
      } else {
        reject(
          new Error(
            `fc-worker command failed with exit code ${code}: ${[command, ...commandArgs].join(
              " "
            )}`
          )
        );
      }
    });
  });
}

function parseDatasetList(stdout) {
  const datasets = [];
  for (const line of stdout.split(/\r?\n/)) {
    const match = line.match(
      /^\[(?<key>[^\]]+)\]\s+(?<marketScope>\S+)\s+rows=(?<rows>\d+)\s+feature_set=(?<featureSet>\S+)\s+label=(?<label>\S+)\s+pit=(?<pit>\S+)\s+range=(?<from>\S+)\s+->\s+(?<to>\S+)/
    );
    if (!match?.groups) {
      continue;
    }
    const [datasetId, datasetVersion] = match.groups.key.split(":", 2);
    if (!datasetId || !datasetVersion) {
      continue;
    }
    datasets.push({
      datasetKey: match.groups.key,
      datasetId,
      datasetVersion,
      marketScope: match.groups.marketScope,
      rowCount: Number(match.groups.rows),
      featureSet: match.groups.featureSet,
      label: match.groups.label,
      pit: match.groups.pit,
      from: match.groups.from,
      to: match.groups.to
    });
  }
  return datasets;
}

function chooseBestDatasets(datasets) {
  const selected = new Map();
  for (const dataset of datasets) {
    if (!options.datasetIds.includes(dataset.datasetId)) {
      continue;
    }
    const current = selected.get(dataset.datasetId);
    const shouldReplace =
      !current ||
      dataset.rowCount > current.rowCount ||
      (dataset.rowCount === current.rowCount && dataset.datasetVersion > current.datasetVersion);
    if (shouldReplace) {
      selected.set(dataset.datasetId, dataset);
    }
  }
  return selected;
}

async function main() {
  console.log(`Listing formal datasets for market scope ${options.marketScope} ...`);
  const { stdout } = await runWorker([
    "research",
    "dataset",
    "list-main",
    "--market-scope",
    options.marketScope
  ]);
  const datasets = parseDatasetList(stdout);
  const selected = chooseBestDatasets(datasets);
  const missing = options.datasetIds.filter((datasetId) => !selected.has(datasetId));
  if (missing.length > 0) {
    throw new Error(
      `missing dataset key(s) for ${missing.join(", ")}; run dataset build before exporting summaries`
    );
  }

  await mkdir(options.outputDir, { recursive: true });
  for (const datasetId of options.datasetIds) {
    const dataset = selected.get(datasetId);
    console.log(`Exporting summary for ${dataset.datasetKey} (${dataset.rowCount} rows) ...`);
    await runWorker([
      "research",
      "dataset",
      "summarize-main",
      "--market-scope",
      options.marketScope,
      "--dataset-key",
      dataset.datasetKey,
      "--output-dir",
      options.outputDir
    ]);
  }
  console.log(`Formal dataset summary pack exported to ${options.outputDir}.`);
}

main().catch((error) => {
  console.error(`Formal dataset summary pack failed: ${error?.message ?? error}`);
  process.exit(1);
});
