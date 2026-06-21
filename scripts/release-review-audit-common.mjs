import { readdir, readFile, stat, mkdir, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";

export function requireValue(args, index, flag) {
  const value = args[index];
  if (typeof value !== "string" || value.length === 0 || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

export function asArray(value) {
  return Array.isArray(value) ? value : [];
}

export function asNumber(value, fallback = null) {
  return typeof value === "number" && Number.isFinite(value) ? value : fallback;
}

export function asString(value, fallback = "") {
  return typeof value === "string" ? value : fallback;
}

export function timestampForFile(date = new Date()) {
  return date.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
}

export async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

export async function writeJson(path, value) {
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

export function repoRelativePath(root, path) {
  const absoluteRoot = resolve(root);
  const absolutePath = resolve(path);
  const rel = relative(absoluteRoot, absolutePath);
  if (!rel.startsWith("..") && !isAbsolute(rel)) {
    return rel.split(sep).join("/");
  }
  return absolutePath.split(sep).join("/");
}

export async function resolveReviewReportPath({
  root,
  baselineReleaseId,
  candidateReleaseId,
  historyMode,
  reportPath
}) {
  if (reportPath) {
    const explicitPath = isAbsolute(reportPath) ? reportPath : resolve(root, reportPath);
    await stat(explicitPath);
    return explicitPath;
  }

  const reportDirectory = resolve(root, "artifacts/research/release-review");
  const expectedSuffix = `${baselineReleaseId}-vs-${candidateReleaseId}-${historyMode}-release-review.json`;
  let entries = [];
  try {
    entries = await readdir(reportDirectory, { withFileTypes: true });
  } catch {
    throw new Error(`release-review artifact directory was not found: ${reportDirectory}`);
  }

  const matches = [];
  for (const entry of entries) {
    if (!entry.isFile() || !entry.name.endsWith(expectedSuffix)) {
      continue;
    }
    const path = join(reportDirectory, entry.name);
    matches.push({ path, mtimeMs: (await stat(path)).mtimeMs });
  }
  matches.sort((left, right) => right.mtimeMs - left.mtimeMs || right.path.localeCompare(left.path));
  if (matches.length === 0) {
    throw new Error(
      `no release-review artifact matched baseline=${baselineReleaseId} candidate=${candidateReleaseId} history_mode=${historyMode}`
    );
  }
  return matches[0].path;
}

export function releaseId(release) {
  return release?.release_id ?? release?.manifest?.release_id ?? null;
}

export function metric(comparison, name) {
  const value = comparison?.[name];
  return value && typeof value === "object" && !Array.isArray(value) ? value : null;
}

export function horizonRow(rows, horizonDays) {
  return asArray(rows).find((row) => Number(row?.horizon_days) === horizonDays) ?? null;
}

export function scenarioName(scenario) {
  return (
    scenario?.name ??
    scenario?.label ??
    scenario?.scenario_label ??
    scenario?.scenario_id ??
    "unknown"
  );
}

export function joinText(values) {
  return asArray(values)
    .map((value) => String(value))
    .filter(Boolean)
    .join(" | ");
}

export function countFrom(value) {
  const number = Number(value);
  return Number.isFinite(number) ? number : 0;
}

export function maybeRound(value, digits = 6) {
  return typeof value === "number" && Number.isFinite(value)
    ? Number(value.toFixed(digits))
    : null;
}
