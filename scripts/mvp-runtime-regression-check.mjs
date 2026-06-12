import { execFile } from "node:child_process";
import { access, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const apiBaseUrl = process.env.FC_API_BASE_URL ?? "http://127.0.0.1:18080";
const webBaseUrl = process.env.FC_WEB_BASE_URL ?? "http://127.0.0.1:5173";
const allowDemoMode = process.env.FC_ALLOW_DEMO === "1";
const tailSuppressorFeature = "tail_pos__us_usdjpy_level__145";
const execFileAsync = promisify(execFile);

const failures = [];

function fail(message) {
  failures.push(message);
}

function assert(condition, message) {
  if (!condition) {
    fail(message);
  }
}

function numberOrNull(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function stringValue(value) {
  return typeof value === "string" ? value : "";
}

function formatDate(value) {
  return typeof value === "string" && value ? value.slice(0, 10) : "—";
}

function levelLabel(level) {
  const labels = {
    normal: "L0 正常",
    watch: "L1 观察",
    stress: "L2 压力",
    warning: "L3 预警",
    crisis: "L4 危机态"
  };
  return labels[level] ?? stringValue(level);
}

function trimTrailingZeros(value) {
  return value.replace(/(?:\.0+|(\.\d*?[1-9])0+)$/, "$1");
}

function formatPercentPrecise(value) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  if (absolute === 0) {
    return "0%";
  }
  if (absolute < 0.0001) {
    return "<0.01%";
  }
  if (absolute < 0.01) {
    return `${trimTrailingZeros((value * 100).toFixed(2))}%`;
  }
  if (absolute < 1) {
    return `${trimTrailingZeros((value * 100).toFixed(1))}%`;
  }
  return `${trimTrailingZeros((value * 100).toFixed(0))}%`;
}

function formatNumber(value, suffix = "") {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  if (absolute === 0) {
    return `0.0${suffix}`;
  }
  if (absolute < 0.1) {
    const digits = absolute < 0.01 ? 4 : 3;
    return `${trimTrailingZeros(value.toFixed(digits))}${suffix}`;
  }
  return `${value.toFixed(1)}${suffix}`;
}

function formatPercent(value, digits = 0) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `${(value * 100).toFixed(digits)}%`;
}

function formatSignedNumber(value, digits = 1, suffix = "") {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${value.toFixed(digits)}${suffix}`;
}

function formatProbabilityPercentExact(value) {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const percent = value * 100;
  const absolutePercent = Math.abs(percent);
  if (absolutePercent === 0) {
    return "0%";
  }
  if (absolutePercent < 0.0001) {
    return "<0.0001%";
  }
  const digits =
    absolutePercent < 0.01 ? 4 : absolutePercent < 0.1 ? 3 : absolutePercent < 1 ? 2 : 1;
  return `${trimTrailingZeros(percent.toFixed(digits))}%`;
}

function probabilityDiagnosticByHorizon(snapshot, horizonDays) {
  return snapshot.probability_diagnostics?.horizon_overlays?.find(
    (horizon) => horizon.horizon_days === horizonDays
  );
}

async function fileExists(path) {
  if (!path) {
    return false;
  }
  try {
    await access(path);
    return true;
  } catch {
    return false;
  }
}

async function findBrowserExecutable() {
  if (await fileExists(process.env.FC_MVP_BROWSER_PATH)) {
    return process.env.FC_MVP_BROWSER_PATH;
  }

  const candidates =
    process.platform === "win32"
      ? [
          join(process.env.ProgramFiles ?? "", "Google/Chrome/Application/chrome.exe"),
          join(process.env["ProgramFiles(x86)"] ?? "", "Google/Chrome/Application/chrome.exe"),
          join(process.env.ProgramFiles ?? "", "Microsoft/Edge/Application/msedge.exe"),
          join(process.env["ProgramFiles(x86)"] ?? "", "Microsoft/Edge/Application/msedge.exe")
        ]
      : [
          "/usr/bin/google-chrome",
          "/usr/bin/google-chrome-stable",
          "/usr/bin/chromium",
          "/usr/bin/chromium-browser",
          "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
          "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
        ];

  for (const candidate of candidates) {
    if (await fileExists(candidate)) {
      return candidate;
    }
  }
  return null;
}

function renderedUrlForView(viewId) {
  const url = new URL(webBaseUrl);
  if (viewId && viewId !== "decision") {
    url.searchParams.set("view", viewId);
  }
  return url.toString();
}

async function dumpRenderedDom(browserPath, targetUrl = webBaseUrl) {
  const userDataDir = await mkdtemp(join(tmpdir(), "fc-mvp-browser-"));
  try {
    const { stdout } = await execFileAsync(
      browserPath,
      [
        "--headless=new",
        "--disable-gpu",
        "--disable-extensions",
        "--no-first-run",
        "--disable-background-networking",
        `--user-data-dir=${userDataDir}`,
        "--virtual-time-budget=10000",
        "--dump-dom",
        targetUrl
      ],
      { maxBuffer: 10 * 1024 * 1024, timeout: 20000 }
    );
    return stdout;
  } finally {
    await rm(userDataDir, { force: true, recursive: true });
  }
}

function nearlyEqual(left, right, tolerance = 1e-9) {
  return (
    typeof left === "number" &&
    Number.isFinite(left) &&
    typeof right === "number" &&
    Number.isFinite(right) &&
    Math.abs(left - right) <= tolerance
  );
}

async function fetchJson(path) {
  const url = `${apiBaseUrl}${path}`;
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url} returned ${response.status}`);
  }
  return response.json();
}

function keyIndicatorById(snapshot, indicatorId) {
  return snapshot.key_indicators?.find((indicator) => indicator.indicator_id === indicatorId);
}

function hasUsdJpyTailSuppressorAnomaly(snapshot) {
  return snapshot.probability_diagnostics?.horizon_overlays?.some((horizon) =>
    horizon.base_contributions?.some(
      (contribution) =>
        contribution.name === tailSuppressorFeature &&
        numberOrNull(contribution.raw_value) !== null &&
        contribution.raw_value > 0 &&
        numberOrNull(contribution.contribution) !== null &&
        contribution.contribution <= -1
    )
  );
}

function collectUserFacingMvpCopy(snapshot) {
  const mvp = snapshot.mvp_risk_state ?? {};
  return [
    snapshot.summary,
    mvp.label,
    mvp.summary,
    ...(Array.isArray(mvp.primary_evidence) ? mvp.primary_evidence : []),
    ...(Array.isArray(mvp.blockers) ? mvp.blockers : []),
    ...(Array.isArray(mvp.next_actions) ? mvp.next_actions : [])
  ]
    .filter((value) => typeof value === "string")
    .join("\n");
}

function validateRuntimeMode(snapshot) {
  assert(snapshot.runtime && typeof snapshot.runtime === "object", "runtime block is missing");
  if (!allowDemoMode) {
    assert(
      snapshot.runtime?.demo_mode === false,
      "MVP regression expects SQLite/live local data; set FC_ALLOW_DEMO=1 only for shape-only checks"
    );
    assert(
      snapshot.runtime?.data_mode === "sqlite",
      `expected data_mode=sqlite, got ${snapshot.runtime?.data_mode ?? "<missing>"}`
    );
  }
}

function validateKeyIndicators(snapshot) {
  const requiredIndicators = [
    ["us_external_usdjpy_level", "USDJPY", "boj", "boj_fx_daily"],
    ["us_market_vix_close", "VIX", "fred", "fred_series_observations"],
    ["us_liquidity_effr", "EFFR", "fred", "fred_series_observations"],
    ["jp_rates_call_rate", "JPY call rate", "boj", "boj_money_market_rates"]
  ];

  for (const [indicatorId, label, expectedSourceId, expectedDatasetId] of requiredIndicators) {
    const indicator = keyIndicatorById(snapshot, indicatorId);
    assert(indicator, `${label} key indicator is missing`);
    if (!indicator) {
      continue;
    }
    assert(numberOrNull(indicator.latest_value) !== null, `${label} latest_value is not numeric`);
    assert(stringValue(indicator.latest_as_of_date), `${label} latest_as_of_date is missing`);
    assert(stringValue(indicator.source_id), `${label} source_id is missing`);
    assert(
      indicator.source_id === expectedSourceId,
      `${label} should use source ${expectedSourceId}, got ${indicator.source_id ?? "<missing>"}`
    );
    assert(
      indicator.dataset_id === expectedDatasetId,
      `${label} should use dataset ${expectedDatasetId}, got ${indicator.dataset_id ?? "<missing>"}`
    );
    assert(
      indicator.status === "fresh" || indicator.status === "stale",
      `${label} status should be fresh/stale, got ${indicator.status ?? "<missing>"}`
    );
  }

  const usdJpyIndicator = keyIndicatorById(snapshot, "us_external_usdjpy_level");
  const indicatorValue = numberOrNull(usdJpyIndicator?.latest_value);
  const carryValue = numberOrNull(snapshot.jpy_carry?.usdjpy_level);
  assert(carryValue !== null, "jpy_carry.usdjpy_level is missing");
  if (indicatorValue !== null && carryValue !== null) {
    assert(
      Math.abs(indicatorValue - carryValue) < 1e-9,
      `USDJPY key indicator ${indicatorValue} does not match jpy_carry.usdjpy_level ${carryValue}`
    );
  }
}

function sourceById(sources, sourceId) {
  return sources?.find((source) => source.source_id === sourceId);
}

function validateSourceCatalogRuntime(sources) {
  assert(Array.isArray(sources) && sources.length > 0, "source catalog endpoint returned no sources");

  for (const sourceId of ["fred", "boj", "sec_edgar", "world_bank"]) {
    const source = sourceById(sources, sourceId);
    assert(source, `${sourceId} source catalog entry is missing`);
    if (!source) {
      continue;
    }
    assert(source.production_allowed === true, `${sourceId} should be production allowed`);
    assert(stringValue(source.license_note), `${sourceId} license note is missing`);
    assert(source.health && typeof source.health === "object", `${sourceId} health block is missing`);
    assert(stringValue(source.health?.status), `${sourceId} health status is missing`);
  }

  for (const source of sources ?? []) {
    if (source.production_allowed !== true || source.health?.status === "disabled") {
      continue;
    }
    const healthMessage = stringValue(source.health?.message);
    const lacksRunEvidence =
      source.health?.last_success_at === null ||
      healthMessage.includes("no ingest_runs evidence is available");
    if (lacksRunEvidence) {
      assert(
        source.health?.status !== "healthy",
        `${source.source_id} has no ingestion-run success evidence but is still marked healthy`
      );
    }
  }

  const fred = sourceById(sources, "fred");
  assert(
    stringValue(fred?.license_note).toLowerCase().includes("no-key"),
    "FRED source catalog should document the no-key free CSV path"
  );
}

function validateMvpAuditState(snapshot) {
  const anomalyActive = hasUsdJpyTailSuppressorAnomaly(snapshot);
  const mvp = snapshot.mvp_risk_state ?? {};
  const summary = stringValue(snapshot.summary);
  const userFacingMvpCopy = collectUserFacingMvpCopy(snapshot);
  const probabilities = snapshot.probabilities ?? {};
  const p5d = numberOrNull(probabilities.p_5d);
  const p20d = numberOrNull(probabilities.p_20d);
  const p60d = numberOrNull(probabilities.p_60d);
  const twentyDayCold =
    p5d !== null &&
    p20d !== null &&
    p60d !== null &&
    p20d > 0 &&
    p20d < p5d * 0.25 &&
    p20d < p60d * 0.25;

  assert(typeof mvp.code === "string", "mvp_risk_state.code is missing");
  assert(typeof mvp.label === "string", "mvp_risk_state.label is missing");
  assert(
    mvp.probability_input_status === "usable" || mvp.probability_input_status === "reference_only",
    "mvp_risk_state.probability_input_status is not usable/reference_only"
  );
  assert(
    !/日元套息\s+(Quiet|Building|Stress|Unwind)\b/.test(userFacingMvpCopy),
    "MVP user-facing evidence should not leak raw Rust JPY carry enum names"
  );

  if (anomalyActive) {
    assert(
      mvp.probability_input_status === "reference_only",
      "USDJPY tail suppressor anomaly is active, but MVP probability input is not reference_only"
    );
    assert(
      mvp.label?.includes("概率参考"),
      "reference_only MVP label should tell the user it is a reference-state reading"
    );
    assert(
      summary.includes("MVP 风险状态"),
      "reference_only API summary should lead with MVP risk state"
    );
    assert(
      summary.includes("不参与主结论"),
      "reference_only API summary should state formal probabilities do not drive the main conclusion"
    );
    assert(
      summary.includes("概率参考值"),
      "reference_only API summary should explicitly call formal probabilities reference readings"
    );
    assert(
      summary.includes("不能解释成风险已经远离"),
      "reference_only API summary should prevent low formal probabilities from being read as risk is far away"
    );
    assert(
      !summary.includes("当前仍偏常态区间"),
      "reference_only API summary must not say the current state is simply normal"
    );
    assert(
      !userFacingMvpCopy.includes("低 formal") &&
        !userFacingMvpCopy.includes("formal 读数") &&
        !userFacingMvpCopy.includes("formal 风险"),
      "user-facing MVP copy still contains old formal wording"
    );
  }

  if (twentyDayCold) {
    assert(
      mvp.probability_input_status === "reference_only",
      "20d is materially colder than 5d/60d, but MVP did not downgrade formal probabilities to reference_only"
    );
    assert(
      Array.isArray(mvp.blockers) &&
        mvp.blockers.some((blocker) => stringValue(blocker).includes("语义异常")),
      "20d cold state should expose a model semantic anomaly blocker"
    );
    assert(
      summary.includes("不能解释成风险已经远离"),
      "20d cold audit state must prevent low probabilities from being read as risk is far away"
    );
  }
}

function validateProbabilityDisplayContract(snapshot) {
  const probabilities = snapshot.probabilities ?? {};
  const mvp = snapshot.mvp_risk_state ?? {};
  const summary = stringValue(snapshot.summary);
  const currentValues = [
    [5, numberOrNull(probabilities.p_5d)],
    [20, numberOrNull(probabilities.p_20d)],
    [60, numberOrNull(probabilities.p_60d)]
  ];

  for (const [horizonDays, probability] of currentValues) {
    assert(probability !== null, `${horizonDays}d page probability is missing`);
    const diagnostic = probabilityDiagnosticByHorizon(snapshot, horizonDays);
    assert(diagnostic, `${horizonDays}d probability diagnostic is missing`);
    if (!diagnostic || probability === null) {
      continue;
    }

    const modelFinal = numberOrNull(diagnostic.final_probability);
    const runtimeFinal = numberOrNull(diagnostic.runtime_final_probability ?? diagnostic.final_probability);
    assert(modelFinal !== null, `${horizonDays}d model final probability is missing`);
    assert(runtimeFinal !== null, `${horizonDays}d runtime final probability is missing`);
    assert(
      nearlyEqual(probability, runtimeFinal),
      `${horizonDays}d displayed probability ${probability} should match runtime_final_probability ${runtimeFinal}`
    );
  }

  const formattedProbabilitySummary = currentValues
    .map(([, probability]) => formatProbabilityPercentExact(probability))
    .join(" / ");
  assert(
    !formattedProbabilitySummary.includes("—") && !formattedProbabilitySummary.includes("NaN"),
    `probability display summary is not renderable: ${formattedProbabilitySummary}`
  );

  if (mvp.probability_input_status === "reference_only") {
    assert(
      summary.includes("概率参考值") && summary.includes("不参与主结论"),
      "reference_only state should explain that displayed probabilities are reference values outside the main conclusion"
    );

    for (const horizonDays of [20, 60]) {
      const diagnostic = probabilityDiagnosticByHorizon(snapshot, horizonDays);
      const modelFinal = numberOrNull(diagnostic?.final_probability);
      const runtimeFinal = numberOrNull(diagnostic?.runtime_final_probability);
      if (modelFinal === null || runtimeFinal === null) {
        continue;
      }
      assert(
        runtimeFinal + 1e-9 >= modelFinal,
        `${horizonDays}d runtime reference probability ${runtimeFinal} should not be below model final ${modelFinal}`
      );
      if (modelFinal < 0.001) {
        assert(
          runtimeFinal >= 0.01,
          `${horizonDays}d model final is below 0.1%, but runtime reference value ${runtimeFinal} is not visibly separated from the cold model output`
        );
      }
    }
  }

  return formattedProbabilitySummary;
}

function validateRuleLayerRiskScoreContract(snapshot) {
  const scores = snapshot.scores ?? {};
  const scoreValues = [
    ["overall_score", "overall rule-layer score"],
    ["structural_score", "structural rule-layer score"],
    ["trigger_score", "trigger rule-layer score"],
    ["external_shock_score", "external rule-layer score"]
  ];

  for (const [field, label] of scoreValues) {
    assert(numberOrNull(scores[field]) !== null, `${label} is missing`);
  }

  return `总风险 ${formatNumber(numberOrNull(scores.overall_score))} / 结构 ${formatNumber(
    numberOrNull(scores.structural_score)
  )} / 触发 ${formatNumber(numberOrNull(scores.trigger_score))} / 外部 ${formatNumber(
    numberOrNull(scores.external_shock_score)
  )}`;
}

function validateEventAndCarryNumberAuditContract(snapshot) {
  const eventAssessment = snapshot.event_assessment ?? {};
  const jpyCarry = snapshot.jpy_carry ?? {};
  const recentEvents = Array.isArray(eventAssessment.recent_events)
    ? eventAssessment.recent_events
    : [];

  assert(
    numberOrNull(eventAssessment.confirmation_score) !== null,
    "event confirmation score is missing"
  );
  assert(
    typeof eventAssessment.recent_event_count === "number",
    "recent event count is missing"
  );
  if (eventAssessment.recent_event_count > 0) {
    assert(recentEvents.length > 0, "recent event count is non-zero but recent_events is missing");
    assert(stringValue(recentEvents[0]?.triggered_as_of_date), "first recent event date is missing");
    assert(stringValue(recentEvents[0]?.level), "first recent event level is missing");
  }
  assert(numberOrNull(jpyCarry.score) !== null, "JPY carry score is missing");
  assert(
    numberOrNull(jpyCarry.funding_pressure_score) !== null,
    "JPY carry funding pressure score is missing"
  );
  assert(
    numberOrNull(jpyCarry.vix_coupling_score) !== null,
    "JPY carry VIX coupling score is missing"
  );
  assert(numberOrNull(jpyCarry.realized_vol_20d) !== null, "JPY carry 20d realized volatility is missing");

  return [
    eventSignalListLabel(eventAssessment.state),
    recentEvents.length > 0
      ? `${formatDate(recentEvents[0].triggered_as_of_date)} · ${levelLabel(recentEvents[0].level)}`
      : "",
    `确认分 ${formatNumber(numberOrNull(eventAssessment.confirmation_score))} / 近期事件 ${
      eventAssessment.recent_event_count ?? "—"
    } 条`,
    `放大器 ${formatNumber(numberOrNull(jpyCarry.score))} / 融资压力 ${formatNumber(
      numberOrNull(jpyCarry.funding_pressure_score)
    )} / VIX 联动 ${formatNumber(numberOrNull(jpyCarry.vix_coupling_score))}`,
    `美日短端利差 ${formatSignedNumber(numberOrNull(jpyCarry.us_jp_short_rate_diff), 2, "%")}`,
    `20d 日收益波动 ${formatPercentPrecise(numberOrNull(jpyCarry.realized_vol_20d))}`
  ];
}

function eventSignalListLabel(state) {
  return state === "confirmed" || state === "escalating" ? "已确认信号" : "近期观察信号";
}

function clamp01(value) {
  return Math.max(0, Math.min(1, value));
}

function modelReliabilityComponent(snapshot) {
  if (snapshot.runtime?.demo_mode) {
    return 0.1;
  }
  if (snapshot.method?.release_status === "degraded") {
    return 0.25;
  }
  if (snapshot.mvp_risk_state?.probability_input_status === "reference_only") {
    return 0.35;
  }
  if (snapshot.method?.release_status === "healthy") {
    return 0.9;
  }
  return 0.65;
}

function modelReliabilityLabel(snapshot) {
  const score = modelReliabilityComponent(snapshot);
  if (snapshot.runtime?.demo_mode) {
    return `演示 ${formatPercent(score)}`;
  }
  if (snapshot.method?.release_status === "degraded") {
    return `降级 ${formatPercent(score)}`;
  }
  if (snapshot.mvp_risk_state?.probability_input_status === "reference_only") {
    return `参考 ${formatPercent(score)}`;
  }
  if (snapshot.method?.release_status === "healthy") {
    return `健康 ${formatPercent(score)}`;
  }
  return `需复核 ${formatPercent(score)}`;
}

function freshnessReliabilityComponent(snapshot) {
  if (snapshot.runtime?.stale_warning) {
    return 0.35;
  }
  const businessLag =
    snapshot.runtime?.latest_key_indicator_lag_business_days ??
    snapshot.runtime?.latest_observation_lag_business_days;
  if (businessLag === null || businessLag === undefined) {
    return 0.55;
  }
  if (businessLag <= 2) {
    return 1;
  }
  if (businessLag <= 5) {
    return 0.75;
  }
  if (businessLag <= 10) {
    return 0.45;
  }
  return 0.25;
}

function freshnessReliabilityLabel(snapshot) {
  const score = freshnessReliabilityComponent(snapshot);
  if (snapshot.runtime?.stale_warning) {
    return `滞后 ${formatPercent(score)}`;
  }
  if (score >= 0.95) {
    return `新鲜 ${formatPercent(score)}`;
  }
  if (score >= 0.7) {
    return `可用 ${formatPercent(score)}`;
  }
  if (score >= 0.45) {
    return `需复核 ${formatPercent(score)}`;
  }
  return `陈旧 ${formatPercent(score)}`;
}

function historicalAnalogComponent(snapshot) {
  const analogs = Array.isArray(snapshot.historical_analogs) ? snapshot.historical_analogs : [];
  const maxSimilarity = Math.max(0, ...analogs.map((analog) => numberOrNull(analog.similarity_score) ?? 0));
  return clamp01(maxSimilarity / 100);
}

function decisionReliabilityScore(snapshot) {
  const rawScore =
    clamp01(numberOrNull(snapshot.data_trust?.coverage_score) ?? 0) * 0.35 +
    modelReliabilityComponent(snapshot) * 0.25 +
    clamp01((numberOrNull(snapshot.event_assessment?.confirmation_score) ?? 0) / 100) * 0.2 +
    historicalAnalogComponent(snapshot) * 0.1 +
    freshnessReliabilityComponent(snapshot) * 0.1;

  if (snapshot.runtime?.demo_mode) {
    return Math.min(rawScore, 0.4);
  }
  if (snapshot.method?.release_status === "degraded") {
    return Math.min(rawScore, 0.5);
  }
  if (snapshot.mvp_risk_state?.probability_input_status === "reference_only") {
    return Math.min(rawScore, 0.45);
  }
  if (snapshot.runtime?.stale_warning) {
    return Math.min(rawScore, 0.65);
  }
  return rawScore;
}

function decisionReliabilityLabel(snapshot) {
  const score = decisionReliabilityScore(snapshot);
  if (snapshot.runtime?.demo_mode) {
    return `演示 ${formatPercent(score)}`;
  }
  if (snapshot.method?.release_status === "degraded") {
    return `降级 ${formatPercent(score)}`;
  }
  if (snapshot.mvp_risk_state?.probability_input_status === "reference_only") {
    return `参考上限 ${formatPercent(score)}`;
  }
  if (score >= 0.8) {
    return `高可信 ${formatPercent(score)}`;
  }
  if (score >= 0.65) {
    return `可用 ${formatPercent(score)}`;
  }
  if (score >= 0.45) {
    return `需复核 ${formatPercent(score)}`;
  }
  return `低可信 ${formatPercent(score)}`;
}

function actionEvidenceScore(snapshot) {
  return numberOrNull(snapshot.action_evidence?.score) ?? numberOrNull(snapshot.conviction_score) ?? 0;
}

function actionEvidenceStatus(score) {
  if (score >= 0.82) {
    return "强升级证据";
  }
  if (score >= 0.68) {
    return "可升级证据";
  }
  if (score >= 0.42) {
    return "接近观察线";
  }
  if (score >= 0.18) {
    return "初步观察证据";
  }
  return "仅数据底座";
}

function validateReliabilityNumberAuditContract(snapshot) {
  const eventScore = (numberOrNull(snapshot.event_assessment?.confirmation_score) ?? 0) / 100;
  const evidenceScore = actionEvidenceScore(snapshot);

  assert(numberOrNull(snapshot.data_trust?.coverage_score) !== null, "data trust coverage score is missing");
  assert(numberOrNull(snapshot.conviction_score) !== null, "conviction score is missing");

  return [
    `结论可信度 · ${decisionReliabilityLabel(snapshot)}`,
    `模型 ${modelReliabilityLabel(snapshot)} / 数据 ${freshnessReliabilityLabel(snapshot)} / 事件 ${formatPercent(
      eventScore
    )}`,
    `动作升级证据 · ${actionEvidenceStatus(evidenceScore)}`,
    `总分 ${formatPercent(evidenceScore)} / 数据底座 ${formatPercent(
      numberOrNull(snapshot.action_evidence?.data_quality_component) ?? 0
    )} / 风险广度 ${formatPercent(
      numberOrNull(snapshot.action_evidence?.breadth_component) ?? 0
    )} / 压力 ${formatPercent(numberOrNull(snapshot.action_evidence?.risk_pressure_component) ?? 0)}`
  ];
}

function validateTopRiskDriverCopyContract(snapshot) {
  const drivers = Array.isArray(snapshot.top_risk_drivers) ? snapshot.top_risk_drivers : [];
  const requiredPhrases = [];
  const contracts = [
    {
      indicatorId: "us_liquidity_sofr",
      levelCopy: "不是 SOFR 当前水平",
      forbiddenCopies: ["当前读数 0.03", "当前信号 0.03"],
      errorLabel: "SOFR",
      requireRendered: true
    },
    {
      indicatorId: "us_real_estate_home_price",
      levelCopy: "不是 Case-Shiller 房价指数 当前水平",
      forbiddenCopies: ["当前读数 0.66", "当前信号 0.66"],
      errorLabel: "Case-Shiller",
      requireRendered: false
    }
  ];

  for (const contract of contracts) {
    const driver = drivers.find((item) => item?.indicator_id === contract.indicatorId);
    if (!driver) {
      continue;
    }

    const explanation = stringValue(driver.explanation);
    assert(
      explanation.includes("评分输入") &&
        explanation.includes(contract.levelCopy) &&
        explanation.includes("最新水平"),
      `${contract.errorLabel} derived driver explanation should distinguish the score input from the latest level`
    );
    for (const forbiddenCopy of contract.forbiddenCopies) {
      assert(
        !explanation.includes(forbiddenCopy),
        `${contract.errorLabel} derived driver explanation must not present the score input as the current level`
      );
    }

    if (contract.requireRendered) {
      requiredPhrases.push(contract.levelCopy);
    }
  }

  return requiredPhrases;
}

function validateHistoricalAnalogDisplayContract(snapshot) {
  const analogs = Array.isArray(snapshot.historical_analogs) ? snapshot.historical_analogs : [];
  if (analogs.length === 0) {
    return [];
  }

  const firstSimilarity = numberOrNull(analogs[0]?.similarity_score);
  assert(firstSimilarity !== null, "historical analog similarity score is missing");
  return [
    "相似度（0-100）",
    "不代表当前还剩多少天",
    `${formatNumber(firstSimilarity)} /100`
  ];
}

function validatePositionGuidance(snapshot) {
  const guidance = snapshot.position_guidance ?? {};
  const governance = guidance.governance ?? {};
  const preferences = snapshot.user_preferences ?? {};

  for (const [field, label] of [
    ["target_equity_exposure_pct", "risk asset cap"],
    ["target_cash_pct", "cash target"],
    ["hedge_ratio_pct", "hedge ratio"],
    ["leverage_cap_pct", "leverage cap"],
    ["option_overlay_pct", "option overlay"]
  ]) {
    assert(numberOrNull(guidance[field]) !== null, `position guidance ${label} is missing`);
  }

  const equityCap = numberOrNull(guidance.target_equity_exposure_pct);
  const maxEquity = numberOrNull(preferences.max_equity_cap_pct);
  if (equityCap !== null && maxEquity !== null) {
    assert(equityCap <= maxEquity, `risk asset cap ${equityCap} exceeds user max equity ${maxEquity}`);
  }

  const cashTarget = numberOrNull(guidance.target_cash_pct);
  const cashFloor = numberOrNull(preferences.cash_floor_pct);
  if (cashTarget !== null && cashFloor !== null) {
    assert(cashTarget >= cashFloor, `cash target ${cashTarget} is below user cash floor ${cashFloor}`);
  }

  const leverageCap = numberOrNull(guidance.leverage_cap_pct);
  const maxLeverage = numberOrNull(preferences.max_leverage_pct);
  if (leverageCap !== null && maxLeverage !== null) {
    assert(leverageCap <= maxLeverage, `leverage cap ${leverageCap} exceeds user max leverage ${maxLeverage}`);
  }

  const optionOverlay = numberOrNull(guidance.option_overlay_pct);
  const optionPreference = numberOrNull(preferences.option_overlay_preference_pct);
  if (optionOverlay !== null && optionPreference !== null) {
    assert(
      optionOverlay >= optionPreference,
      `option overlay ${optionOverlay} is below user preference ${optionPreference}`
    );
  }

  assert(guidance.action_summary, "position guidance action_summary is missing");
  assert(Array.isArray(guidance.actions) && guidance.actions.length > 0, "position guidance actions are missing");
  assert(
    Array.isArray(guidance.forbidden_actions) && guidance.forbidden_actions.length > 0,
    "position guidance forbidden_actions are missing"
  );
  assert(governance.system_budget_only === true, "position guidance should be marked as system budget only");
  assert(governance.auto_execution_allowed === false, "MVP must not allow automatic execution");
  assert(governance.manual_confirmation_required === true, "MVP should require manual confirmation");
  assert(
    governance.policy_change_requires_release_review === true,
    "policy changes should require release review"
  );
  assert(governance.policy_change_requires_go_no_go === true, "policy changes should require Go/No-Go");
}

async function validateUserFacingUiCopy() {
  const uiFiles = [
    "../apps/web/src/views/decision/DecisionView.tsx",
    "../apps/web/src/views/decision/sections.tsx",
    "../apps/web/src/views/decision/components.tsx",
    "../apps/web/src/views/decision/numberAudit.ts"
  ];
  const forbiddenPhrases = [
    "机械完成度",
    "触线仍需",
    "触线所需放大",
    "还差多少倍",
    "机械触线",
    "20d 当前是",
    "20d 只有",
    "当前 20日窗口明显低于"
  ];

  for (const file of uiFiles) {
    const text = await readFile(new URL(file, import.meta.url), "utf8");
    for (const phrase of forbiddenPhrases) {
      assert(!text.includes(phrase), `${file} still contains misleading UI copy: ${phrase}`);
    }
  }

  const decisionView = await readFile(
    new URL("../apps/web/src/views/decision/DecisionView.tsx", import.meta.url),
    "utf8"
  );
  assert(
    decisionView.includes("当前数字说明"),
    "decision dashboard should include the current number explanation checklist"
  );
  assert(
    decisionView.includes("关键免费数据源是否可信"),
    "decision dashboard should include the key free data reliability section"
  );
  assert(
    decisionView.includes("mvpProbabilityInputIsAuditOnly"),
    "probability trajectory should use the shared MVP audit-only predicate"
  );

  const mvpRiskState = await readFile(
    new URL("../apps/web/src/views/decision/mvpRiskState.ts", import.meta.url),
    "utf8"
  );
  assert(
    mvpRiskState.includes("mvpProbabilityInputIsAuditOnly"),
    "decision UI should expose one shared audit-only predicate"
  );

  const decisionReliability = await readFile(
    new URL("../apps/web/src/views/decision/decisionReliability.ts", import.meta.url),
    "utf8"
  );
  assert(
    decisionReliability.includes("参考上限") && decisionReliability.includes("Math.min(score, 0.45)"),
    "decision reliability should be visibly capped when formal probabilities are reference-only"
  );

  const decisionSections = await readFile(
    new URL("../apps/web/src/views/decision/sections.tsx", import.meta.url),
    "utf8"
  );
  assert(
    decisionSections.includes("mvpProbabilityInputIsAuditOnly") &&
      decisionSections.includes("当前不计算阈值占比、放大倍数或仓位时距"),
    "risk horizon section should hide mechanical distance math in audit-only mode"
  );
  assert(
    decisionSections.includes("首屏四问摘要") &&
      decisionSections.includes("当前是否危险") &&
      decisionSections.includes("离风险多远") &&
      decisionSections.includes("未进入动作窗口") &&
      decisionSections.includes("为什么") &&
      decisionSections.includes("现在做什么"),
    "decision hero should answer the four MVP decision questions on the first screen"
  );

  const signalLayerBuilders = await readFile(
    new URL("../apps/web/src/views/decision/signalLayerBuilders.ts", import.meta.url),
    "utf8"
  );
  assert(
    signalLayerBuilders.includes('危机先验（参考）') &&
      signalLayerBuilders.includes('参考输入') &&
      signalLayerBuilders.includes('动作信号（辅助）') &&
      signalLayerBuilders.includes('辅助信号'),
    "decision explanation chain should demote formal probabilities and actionability to reference/auxiliary roles"
  );

  const actionPlanBuilders = await readFile(
    new URL("../apps/web/src/views/decision/buildersCore.ts", import.meta.url),
    "utf8"
  );
  assert(
    actionPlanBuilders.includes("系统预算参考") &&
      actionPlanBuilders.includes("当前先按预算参考解读") &&
      actionPlanBuilders.includes('value: actionValuesAreAuxiliary'),
    "action plan metrics should demote transitional budgets and actionability to reference signals"
  );

  const actionBoundaries = await readFile(
    new URL("../apps/web/src/views/decision/actionBoundaries.ts", import.meta.url),
    "utf8"
  );
  assert(
    actionBoundaries.includes("系统预算参考") &&
      actionBoundaries.includes("不应被直接当成精确执行指令"),
    "action boundary copy should warn that budget ranges are reference guidance under MVP mode"
  );

  const actionPlanPanel = await readFile(
    new URL("../apps/web/src/views/decision/panels.tsx", import.meta.url),
    "utf8"
  );
  assert(
    actionPlanPanel.includes("当前预算参考") &&
      actionPlanPanel.includes("风险资产上限（参考）") &&
      actionPlanPanel.includes("系统预算参考"),
    "action plan panel should visibly mark current budgets and bars as reference-only"
  );
  assert(
    actionPlanPanel.includes("相似度（0-100）") &&
      actionPlanPanel.includes("相似度是 0-100 的结构参照分，不是危机发生概率") &&
      actionPlanPanel.includes("不代表当前还剩多少天"),
    "historical analog panel should label similarity and lead time as reference scores, not probabilities or countdowns"
  );

  const decisionBacktestBuilders = await readFile(
    new URL("../apps/web/src/views/decision/buildersBacktests.ts", import.meta.url),
    "utf8"
  );
  assert(
    decisionBacktestBuilders.includes("动作信号精度（历史）") &&
      decisionBacktestBuilders.includes("不是当前正式概率的实时准确率") &&
      decisionBacktestBuilders.includes("历史回放评估点/区间数量"),
    "rolling audit metrics should identify historical replay precision/counts instead of implying current probability accuracy"
  );

  const backtestsViewModel = await readFile(
    new URL("../apps/web/src/views/backtests/useBacktestsViewModel.ts", import.meta.url),
    "utf8"
  );
  assert(
    backtestsViewModel.includes("动作命中（场景回测）") &&
      backtestsViewModel.includes("动作信号点（历史）") &&
      backtestsViewModel.includes("纯误报区间（历史）") &&
      backtestsViewModel.includes("不是今天新增事件数，也不是当前正式概率准确率"),
    "backtests headline metrics should label scenario hit rates and historical replay counts before showing bare numbers"
  );

  const indicatorsViewModel = await readFile(
    new URL("../apps/web/src/views/indicators/useIndicatorsViewModel.ts", import.meta.url),
    "utf8"
  );
  const indicatorsView = await readFile(
    new URL("../apps/web/src/views/indicators/IndicatorsView.tsx", import.meta.url),
    "utf8"
  );
  const indicatorsContent = await readFile(
    new URL("../apps/web/src/views/indicators/content.ts", import.meta.url),
    "utf8"
  );
  const viewRegistry = await readFile(new URL("../apps/web/src/viewRegistry.tsx", import.meta.url), "utf8");
  assert(
    indicatorsViewModel.includes("评分输入") &&
      indicatorsViewModel.includes("不是 ${humanizeNarrativeCopy(risk.indicator.display_name)} 当前水平") &&
      indicatorsViewModel.includes("当前水平请看左侧最近读数") &&
      indicatorsViewModel.includes("最近读数 ${formatValue") &&
      indicatorsViewModel.includes("指标级 ${qualityDetailLabel") &&
      indicatorsViewModel.includes("单项观测质量分") &&
      indicatorsViewModel.includes("不是整体结论可信度") &&
      indicatorsViewModel.includes("nearTermFrequencyRank") &&
      indicatorsViewModel.includes("isNearTermMonitorCandidate") &&
      indicatorsViewModel.includes("focusTrackingScope") &&
      indicatorsViewModel.includes('"背景跟踪"') &&
      indicatorsView.includes("近端最需盯的指标") &&
      indicatorsContent.includes("评分输入不是当前水平") &&
      indicatorsContent.includes("当前水平请看最近读数") &&
      indicatorsContent.includes("日频/周频指标") &&
      indicatorsContent.includes("月频、季频、年频高分项更适合按结构背景解释") &&
      indicatorsContent.includes("指标级质量") &&
      indicatorsContent.includes("不等同于整体结论可信度") &&
      viewRegistry.includes("指标级质量"),
    "indicators view should separate derived score inputs and indicator-level quality from latest observed levels and conclusion confidence"
  );

  const eventsViewModel = await readFile(
    new URL("../apps/web/src/views/events/useEventsViewModel.ts", import.meta.url),
    "utf8"
  );
  assert(
    eventsViewModel.includes("0-100 分事件确认输入") &&
      eventsViewModel.includes("不是危机概率") &&
      eventsViewModel.includes("不是当前新增事件数") &&
      eventsViewModel.includes("不等同于指标触发数量"),
    "events view should explain confirmation score and event counts before showing bare numbers"
  );

  const decisionApp = await readFile(new URL("../apps/web/src/App.tsx", import.meta.url), "utf8");
  assert(
    decisionApp.includes("mvpProbabilityInputIsAuditOnly"),
    "top-level loading and meta UI should use the shared MVP audit-only predicate"
  );
  assert(
    decisionApp.includes('drivers: ["assessment", "indicators", "overview", "posture"]'),
    "drivers view should require indicators data before rendering near-term driver lists"
  );
  assert(
    decisionApp.includes("当前处于参考态") &&
      decisionApp.includes("当前正式概率只作为参考输入，动作信号和预算边界也只作为辅助参考"),
    "top-level app shell should show a global reference-only banner across views"
  );
  assert(
    decisionApp.includes("productionSourceIssueLabels") &&
      decisionApp.includes("生产源健康降级") &&
      decisionApp.includes("partial_failure"),
    "top-level app shell should surface production source health downgrade counts"
  );

  const numberAudit = await readFile(
    new URL("../apps/web/src/views/decision/numberAudit.ts", import.meta.url),
    "utf8"
  );
  assert(
    numberAudit.includes("mvpProbabilityInputIsAuditOnly"),
    "number audit checklist should mark formal probabilities with the shared audit-only predicate"
  );
  assert(
    numberAudit.includes("来源：") &&
      numberAudit.includes("单位：") &&
      numberAudit.includes("日期：") &&
      numberAudit.includes("状态："),
    "number audit checklist should show source, unit, date, and status for key homepage numbers"
  );
  assert(
    numberAudit.includes("规则层风险分数") &&
      numberAudit.includes("API scores / scoring engine") &&
      numberAudit.includes("0-100 分") &&
      numberAudit.includes("不是危机发生概率"),
    "number audit checklist should explain the rule-layer risk scores used by the MVP conclusion"
  );
  assert(
    numberAudit.includes("事件确认") &&
      numberAudit.includes("eventSignalListLabel") &&
      numberAudit.includes("eventSignalAuditItems") &&
      numberAudit.includes("recent_events") &&
      numberAudit.includes("levelLabel") &&
      numberAudit.includes("API event_assessment / event rules") &&
      numberAudit.includes("不是自动动作指令") &&
    numberAudit.includes("日元套息放大器") &&
    numberAudit.includes("API jpy_carry / key indicators") &&
    numberAudit.includes("不是在预测日本危机") &&
      numberAudit.includes("20d 日收益波动") &&
      numberAudit.includes("关键指标覆盖") &&
      numberAudit.includes("不等同于全部免费源都健康"),
    "number audit checklist should explain event confirmation and JPY carry scores as non-probability MVP inputs"
  );
  assert(
    numberAudit.includes("结论可信度") &&
      numberAudit.includes("frontend decisionReliability / API data_trust + method + event_assessment") &&
      numberAudit.includes("可靠性，不是概率") &&
      numberAudit.includes("动作升级证据") &&
      numberAudit.includes("API action_evidence / scoring engine") &&
      numberAudit.includes("actionEvidenceHint"),
    "number audit checklist should explain reliability and action evidence as non-probability support scores"
  );

  const dataSourceReliability = await readFile(
    new URL("../apps/web/src/views/decision/dataSourceReliability.ts", import.meta.url),
    "utf8"
  );
  assert(
    dataSourceReliability.includes("替代路径"),
    "free data reliability section should explain fallback/source alternatives"
  );

  const sourcesViewModel = await readFile(
    new URL("../apps/web/src/views/sources/useSourcesViewModel.ts", import.meta.url),
    "utf8"
  );
  assert(
    sourcesViewModel.includes("sourceHealthMessage") &&
      sourcesViewModel.includes("sourceHealthWarning") &&
      sourcesViewModel.includes("源状态") &&
      sourcesViewModel.includes("humanizeNarrativeCopy") &&
      sourcesViewModel.includes("source.health.message") &&
      sourcesViewModel.includes("warnings: [...assessment.data_trust.warnings, ...sourceWarnings]") &&
      sourcesViewModel.includes("partial_failure") &&
      sourcesViewModel.includes("关键覆盖等级") &&
      sourcesViewModel.includes("源健康降级") &&
      sourcesViewModel.includes("不等同于全部源健康") &&
      sourcesViewModel.includes("源健康分") &&
      sourcesViewModel.includes("extractWatermarkPeriod") &&
      sourcesViewModel.includes("最新观测") &&
      sourcesViewModel.includes("观测滞后") &&
      sourcesViewModel.includes("抓取水位") &&
      sourcesViewModel.includes("最近成功刷新") &&
      sourcesViewModel.includes("未进入正式刷新监控") &&
      sourcesViewModel.includes("抓取/源状态分，不是当前结论可信度") &&
      sourcesViewModel.includes("sourceUsageRecommendation") &&
      sourcesViewModel.includes("可参与当前评估") &&
      sourcesViewModel.includes("先不要依赖") &&
      sourcesViewModel.includes("降级使用") &&
      sourcesViewModel.includes("慢变量背景") &&
      sourcesViewModel.includes("仅作辅助背景") &&
      sourcesViewModel.includes("不能单独触发动作升级"),
    "sources view should split observation dates, refresh watermarks, and user-facing source usage recommendations"
  );

  const sourcesView = await readFile(
    new URL("../apps/web/src/views/sources/SourcesView.tsx", import.meta.url),
    "utf8"
  );
  const sourcesContent = await readFile(
    new URL("../apps/web/src/views/sources/content.ts", import.meta.url),
    "utf8"
  );
  assert(
    sourcesView.includes("数据覆盖与源健康摘要") &&
      sourcesView.includes("源健康分") &&
      sourcesView.includes("使用建议") &&
      sourcesContent.includes("源健康分只说明抓取/源状态，不等同于当前结论可信度") &&
      sourcesContent.includes("最新观测、观测滞后、抓取水位和最近成功刷新是不同口径") &&
      sourcesContent.includes("使用建议说明该源能否参与当前判断"),
    "sources page should distinguish source health scores and source usage advice from overall conclusion confidence"
  );

  const driversViewModel = await readFile(
    new URL("../apps/web/src/views/drivers/useDriversViewModel.ts", import.meta.url),
    "utf8"
  );
  assert(
    driversViewModel.includes("当前主结论先按 MVP 规则层解释；正式概率和 posture 只作背景参考。") &&
      driversViewModel.includes("mvpRiskStateDetail") &&
      driversViewModel.includes("buildNearTermRiskDrivers") &&
      driversViewModel.includes("nearTermTimedDrivers.slice(0, 5)") &&
      driversViewModel.includes("nearTermTimedDrivers.filter"),
    "drivers view should align its summary conclusion with the MVP reference-only state"
  );

  const backtestsView = await readFile(
    new URL("../apps/web/src/views/backtests/BacktestsView.tsx", import.meta.url),
    "utf8"
  );
  assert(
    backtestsView.includes("当前运行历史轨迹（参考）") &&
      backtestsView.includes("只保留给模型复核和历史对照使用"),
    "backtests view should demote current formal history trajectory to reference-only when MVP is in reference mode"
  );

  const methodViewModel = await readFile(
    new URL("../apps/web/src/views/method/useMethodViewModel.ts", import.meta.url),
    "utf8"
  );
  const methodView = await readFile(
    new URL("../apps/web/src/views/method/MethodView.tsx", import.meta.url),
    "utf8"
  );
  const methodContent = await readFile(
    new URL("../apps/web/src/views/method/content.ts", import.meta.url),
    "utf8"
  );
  assert(
    methodViewModel.includes("只说明服务和 bundle 可加载，不代表正式概率已恢复为当前主结论") &&
      methodViewModel.includes("不是数据新鲜度或结论可信度") &&
      methodViewModel.includes("不是训练样本数量，也不是 Go/No-Go 通过次数") &&
      methodViewModel.includes("这是历史证据层，不代表当前正式概率可作主结论") &&
      methodView.includes("怎么看这些百分比") &&
      methodContent.includes("不是当前 5d / 20d / 60d 概率，也不是风险还差多少"),
    "method page should separate service status, PIT evidence counts, and threshold percentages from current conclusion confidence"
  );

  const auditViewModel = await readFile(
    new URL("../apps/web/src/views/audit/useAuditViewModel.ts", import.meta.url),
    "utf8"
  );
  const auditContent = await readFile(
    new URL("../apps/web/src/views/audit/content.ts", import.meta.url),
    "utf8"
  );
  const auditRuntimeContribution = await readFile(
    new URL("../apps/web/src/views/audit/runtimeContributionAuditSection.tsx", import.meta.url),
    "utf8"
  );
  const auditLeadtime = await readFile(
    new URL("../apps/web/src/views/audit/leadtimeAuditSection.tsx", import.meta.url),
    "utf8"
  );
  const auditFundingStress = await readFile(
    new URL("../apps/web/src/views/audit/fundingStressAuditSection.tsx", import.meta.url),
    "utf8"
  );
  const auditPrewarningGap = await readFile(
    new URL("../apps/web/src/views/audit/prewarningGapAuditSection.tsx", import.meta.url),
    "utf8"
  );
  const auditCooldown = await readFile(
    new URL("../apps/web/src/views/audit/cooldownAuditSection.tsx", import.meta.url),
    "utf8"
  );
  const auditDatasetSummary = await readFile(
    new URL("../apps/web/src/views/audit/datasetSummarySection.ts", import.meta.url),
    "utf8"
  );
  assert(
    auditViewModel.includes("不代表当前概率已可作为主结论") &&
      auditViewModel.includes("当前正式概率仍是参考输入") &&
      auditViewModel.includes("不代表可上线版本数") &&
      auditViewModel.includes("这是运行快照覆盖，不是模型训练样本数") &&
      auditViewModel.includes("历史证据层覆盖，不代表当前正式概率已经恢复主结论") &&
      auditContent.includes("登记数量不等于可上线数量") &&
      auditContent.includes("这些都是审计口径，不是自动上线放行结论") &&
      auditContent.includes("离线评估指标，不是当前页面主结论"),
    "audit page should label registry/replay/snapshot/release-review numbers as audit evidence, not deployment or current-conclusion proof"
  );
  assert(
    auditRuntimeContribution.includes("入线占比（审计）") &&
      auditRuntimeContribution.includes("不是训练样本数") &&
      auditContent.includes("入线占比只能当模型审计证据") &&
      !auditRuntimeContribution.includes("触线完成度") &&
      !auditContent.includes("触线完成度"),
    "runtime contribution audit should label touchline ratios as audit entry-line evidence, not risk distance"
  );
  assert(
    auditLeadtime.includes("动作精度（历史）") &&
      auditLeadtime.includes("不是当前正式概率准确率") &&
      auditLeadtime.includes("历史 runtime / strict 点") &&
      !auditLeadtime.includes("Action precision"),
    "lead-time audit should mark precision/hit/lead-time metrics as offline historical evidence"
  );
  assert(
    auditFundingStress.includes("20d 历史峰值 / 入线") &&
      auditFundingStress.includes("不是当前 20d 风险距离") &&
      auditFundingStress.includes("Dataset 证据（历史样本）"),
    "funding stress audit should separate historical floor gaps from current risk distance"
  );
  assert(
    auditPrewarningGap.includes("候选 20d 命中（历史）") &&
      auditPrewarningGap.includes("不是当前正式概率命中") &&
      auditPrewarningGap.includes("近线点（历史）"),
    "pre-warning gap audit should mark hit counts and near-threshold counts as historical replay evidence"
  );
  assert(
    auditCooldown.includes("审计结论（离线）") &&
      auditCooldown.includes("不是今天新增误报") &&
      auditCooldown.includes("误报数量（历史）"),
    "cooldown audit should mark no-go and false-positive episode counts as offline release evidence"
  );
  assert(
    auditDatasetSummary.includes("总样本行（历史）") &&
      auditDatasetSummary.includes("不是当前线上模型训练样本承诺") &&
      auditDatasetSummary.includes("目录覆盖"),
    "dataset summary audit should mark row counts and coverage as historical dataset evidence"
  );

  const justfile = await readFile(new URL("../justfile", import.meta.url), "utf8");
  assert(
    /refresh-latest:\s+cargo run -p fc-worker -- refresh latest-free[^\n]*--mvp-key-only/.test(justfile),
    "default refresh-latest should use --mvp-key-only so daily free-data refresh reaches BOJ/Treasury/SEC"
  );
  assert(
    /refresh-latest-full:\s+cargo run -p fc-worker -- refresh latest-free(?![^\n]*--mvp-key-only)[^\n]*--include-gdelt/.test(
      justfile
    ),
    "refresh-latest-full should remain the broader full refresh path rather than the MVP key-only path"
  );
}

async function validateRenderedUiIfAvailable(
  formattedProbabilitySummary,
  runtimeNumberAuditPhrases,
  renderedForbiddenPhrases = [],
  sources = []
) {
  if (process.env.FC_SKIP_RENDERED_UI === "1") {
    return "skipped(disabled)";
  }

  const browserPath = await findBrowserExecutable();
  if (!browserPath) {
    return "skipped(no_browser)";
  }

  const dom = await dumpRenderedDom(browserPath);
  const degradedProductionSources = (sources ?? []).filter(
    (source) =>
      source?.production_allowed === true &&
      ["delayed", "partial_failure", "failed"].includes(source?.health?.status)
  );
  const requiredPhrases = [
    "观察为主（概率参考）",
    "当前数字说明",
    "规则层风险分数",
    "API scores / scoring engine",
    "0-100 分",
    "不是危机发生概率",
    "事件确认",
    "API event_assessment / event rules",
    "不是自动动作指令",
    "日元套息放大器",
    "API jpy_carry / key indicators",
    "不是在预测日本危机",
    "20d 日收益波动",
    "关键指标覆盖",
    "今日与本周变化",
    "变化口径",
    "当前主要解释",
    "不是逐指标因果归因",
    "评估口径日期",
    "浏览器本地时间",
    "frontend decisionReliability / API data_trust + method + event_assessment",
    "可靠性，不是概率",
    "API action_evidence / scoring engine",
    "不是模型结论置信概率",
    ...runtimeNumberAuditPhrases,
    "关键数据覆盖 A",
    "当前正式概率只作为参考输入",
    "正式小概率直接当成低风险证明",
    "未进入动作窗口",
    "不参与 MVP 主结论",
    ...formattedProbabilitySummary.split(" / ")
  ];
  for (const phrase of requiredPhrases) {
    assert(dom.includes(phrase), `rendered decision dashboard is missing: ${phrase}`);
  }
  if (degradedProductionSources.length > 0) {
    assert(
      dom.includes(`生产源健康降级 ${degradedProductionSources.length}`),
      `rendered decision dashboard should show ${degradedProductionSources.length} degraded production source(s)`
    );
    for (const source of degradedProductionSources) {
      assert(
        dom.includes(source.display_name),
        `rendered decision dashboard should name degraded production source: ${source.display_name}`
      );
    }
  } else {
    assert(
      !dom.includes("生产源健康降级"),
      "rendered decision dashboard still shows stale production source downgrade copy"
    );
  }

  for (const phrase of [
    "审计读数",
    "待审计",
    "机械完成度",
    "触线仍需",
    "还差多少倍",
    "数据可信度 A",
    ...renderedForbiddenPhrases
  ]) {
    assert(!dom.includes(phrase), `rendered decision dashboard still contains stale/misleading copy: ${phrase}`);
  }

  const viewContracts = [
    {
      id: "decision",
      title: "美国金融危机风险决策面板",
      required: ["当前数字说明", "为什么是现在", "离风险还有多远"],
      allowAuditWord: false
    },
    {
      id: "drivers",
      title: "风险驱动拆解",
      required: ["近端风险驱动", "结构背景驱动", "当前结论"],
      allowAuditWord: false
    },
    {
      id: "events",
      title: "事件层确认",
      required: ["事件摘要", "确认分数", "最近事件"],
      allowAuditWord: false
    },
    {
      id: "backtests",
      title: "历史回测与误报边界",
      required: ["回测摘要", "滚动历史复核", "当前运行历史轨迹（参考）"],
      allowAuditWord: false
    },
    {
      id: "audit",
      title: "线上版本与研究核对",
      required: ["当前线上版本", "审计摘要", "最近一次 Release Review"],
      allowAuditWord: true
    },
    {
      id: "indicators",
      title: "指标细项总览",
      required: ["当前指标摘要", "近端最需盯的指标", "指标细项"],
      allowAuditWord: false
    },
    {
      id: "sources",
      title: "数据可信度与免费源状态",
      required: ["数据覆盖与源健康摘要", "使用建议", "源状态"],
      allowAuditWord: false
    },
    {
      id: "method",
      title: "方法说明与版本边界",
      required: ["当前方法摘要", "当前运行阈值", "当前结论的限制"],
      allowAuditWord: false
    }
  ];
  const commonBadRenderedTokens = ["审计读数", "待审计", "概率待审计", "NaN", "undefined", "null", "Infinity"];
  for (const contract of viewContracts) {
    const viewDom = contract.id === "decision" ? dom : await dumpRenderedDom(browserPath, renderedUrlForView(contract.id));
    assert(
      viewDom.includes(contract.title),
      `rendered ${contract.id} view is missing title: ${contract.title}`
    );
    assert(
      !viewDom.includes("正在加载视图"),
      `rendered ${contract.id} view still shows the loading placeholder`
    );
    for (const phrase of contract.required) {
      assert(viewDom.includes(phrase), `rendered ${contract.id} view is missing: ${phrase}`);
    }
    for (const token of commonBadRenderedTokens) {
      assert(!viewDom.includes(token), `rendered ${contract.id} view contains invalid token: ${token}`);
    }
    if (!contract.allowAuditWord) {
      assert(!viewDom.includes("审计"), `rendered ${contract.id} view should not expose audit wording`);
    }
  }

  return "checked";
}

const snapshot = await fetchJson("/api/assessment/current");
const sources = await fetchJson("/api/sources");
validateRuntimeMode(snapshot);
validateKeyIndicators(snapshot);
validateSourceCatalogRuntime(sources);
validateMvpAuditState(snapshot);
const formattedProbabilitySummary = validateProbabilityDisplayContract(snapshot);
const runtimeNumberAuditPhrases = [
  validateRuleLayerRiskScoreContract(snapshot),
  ...validateEventAndCarryNumberAuditContract(snapshot),
  ...validateReliabilityNumberAuditContract(snapshot),
  ...validateTopRiskDriverCopyContract(snapshot),
  ...validateHistoricalAnalogDisplayContract(snapshot)
];
const renderedForbiddenPhrases =
  eventSignalListLabel(snapshot.event_assessment?.state) === "已确认信号"
    ? [
        ...probabilityReferenceOnlyComparisonForbiddenPhrases(snapshot),
        ...referenceOnlyRiskDistanceForbiddenPhrases(snapshot)
      ]
    : [
        "已确认信号",
        ...probabilityReferenceOnlyComparisonForbiddenPhrases(snapshot),
        ...referenceOnlyRiskDistanceForbiddenPhrases(snapshot)
      ];
validatePositionGuidance(snapshot);
await validateUserFacingUiCopy();
const renderedUiStatus = await validateRenderedUiIfAvailable(
  formattedProbabilitySummary,
  runtimeNumberAuditPhrases,
  renderedForbiddenPhrases,
  sources
);

if (failures.length > 0) {
  console.error("MVP runtime regression check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

const usdJpy = keyIndicatorById(snapshot, "us_external_usdjpy_level");
const anomalyState = hasUsdJpyTailSuppressorAnomaly(snapshot) ? "reference_only expected" : "usable";
console.log(
  [
    "MVP runtime regression check passed.",
    `data_mode=${snapshot.runtime?.data_mode}`,
    `as_of=${snapshot.as_of_date}`,
    `USDJPY=${usdJpy?.latest_value} @ ${usdJpy?.latest_as_of_date}`,
    `probabilities=${formattedProbabilitySummary}`,
    `rendered_ui=${renderedUiStatus}`,
    `mvp=${snapshot.mvp_risk_state?.label} (${snapshot.mvp_risk_state?.probability_input_status})`,
    `anomaly=${anomalyState}`
  ].join(" | ")
);

function probabilityReferenceOnlyComparisonForbiddenPhrases(snapshot) {
  if (snapshot.mvp_risk_state?.probability_input_status !== "reference_only") {
    return [];
  }

  const probabilities = snapshot.probabilities ?? {};
  const p5d = numberOrNull(probabilities.p_5d);
  const p20d = numberOrNull(probabilities.p_20d);
  const p60d = numberOrNull(probabilities.p_60d);
  if (p5d === null || p20d === null || p60d === null) {
    return [];
  }

  const p5dLabel = formatProbabilityPercentExact(p5d);
  const p20dLabel = formatProbabilityPercentExact(p20d);
  const p60dLabel = formatProbabilityPercentExact(p60d);
  return [
    `20d 当前是 ${p20dLabel}`,
    `20d 只有 ${p20dLabel}`,
    `20日窗口 ${p20dLabel} 明显低于 5日 ${p5dLabel}`,
    `${p20dLabel}，明显低于 5d 是 ${p5dLabel}`,
    `${p20dLabel} 明显低于 5日 ${p5dLabel}`,
    `${p20dLabel} 明显低于 5日和 60日 ${p60dLabel}`
  ];
}

function referenceOnlyRiskDistanceForbiddenPhrases(snapshot) {
  if (snapshot.mvp_risk_state?.probability_input_status !== "reference_only") {
    return [];
  }

  return ["离风险多远\n常态"];
}
