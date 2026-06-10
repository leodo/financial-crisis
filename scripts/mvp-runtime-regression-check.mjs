const apiBaseUrl = process.env.FC_API_BASE_URL ?? "http://127.0.0.1:18080";
const allowDemoMode = process.env.FC_ALLOW_DEMO === "1";
const tailSuppressorFeature = "tail_pos__us_usdjpy_level__145";

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
    ["us_external_usdjpy_level", "USDJPY"],
    ["us_market_vix_close", "VIX"],
    ["us_liquidity_effr", "EFFR"],
    ["jp_rates_call_rate", "JPY call rate"]
  ];

  for (const [indicatorId, label] of requiredIndicators) {
    const indicator = keyIndicatorById(snapshot, indicatorId);
    assert(indicator, `${label} key indicator is missing`);
    if (!indicator) {
      continue;
    }
    assert(numberOrNull(indicator.latest_value) !== null, `${label} latest_value is not numeric`);
    assert(stringValue(indicator.latest_as_of_date), `${label} latest_as_of_date is missing`);
    assert(stringValue(indicator.source_id), `${label} source_id is missing`);
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
    mvp.probability_input_status === "usable" || mvp.probability_input_status === "audit_only",
    "mvp_risk_state.probability_input_status is not usable/audit_only"
  );

  if (anomalyActive) {
    assert(
      mvp.probability_input_status === "audit_only",
      "USDJPY tail suppressor anomaly is active, but MVP probability input is not audit_only"
    );
    assert(mvp.label?.includes("待审计"), "audit_only MVP label should tell the user it is under audit");
    assert(
      summary.includes("MVP 风险状态"),
      "audit_only API summary should lead with MVP risk state"
    );
    assert(
      summary.includes("不参与主结论"),
      "audit_only API summary should state formal probabilities do not drive the main conclusion"
    );
    assert(
      summary.includes("审计读数"),
      "audit_only API summary should explicitly call formal probabilities audit readings"
    );
    assert(
      summary.includes("不能解释成风险已经远离"),
      "audit_only API summary should prevent low formal probabilities from being read as risk is far away"
    );
    assert(
      !summary.includes("当前仍偏常态区间"),
      "audit_only API summary must not say the current state is simply normal"
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
      mvp.probability_input_status === "audit_only",
      "20d is materially colder than 5d/60d, but MVP did not downgrade formal probabilities to audit_only"
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

const snapshot = await fetchJson("/api/assessment/current");
validateRuntimeMode(snapshot);
validateKeyIndicators(snapshot);
validateMvpAuditState(snapshot);
validatePositionGuidance(snapshot);

if (failures.length > 0) {
  console.error("MVP runtime regression check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

const usdJpy = keyIndicatorById(snapshot, "us_external_usdjpy_level");
const anomalyState = hasUsdJpyTailSuppressorAnomaly(snapshot) ? "audit_only expected" : "usable";
console.log(
  [
    "MVP runtime regression check passed.",
    `data_mode=${snapshot.runtime?.data_mode}`,
    `as_of=${snapshot.as_of_date}`,
    `USDJPY=${usdJpy?.latest_value} @ ${usdJpy?.latest_as_of_date}`,
    `mvp=${snapshot.mvp_risk_state?.label} (${snapshot.mvp_risk_state?.probability_input_status})`,
    `anomaly=${anomalyState}`
  ].join(" | ")
);
