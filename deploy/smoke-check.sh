#!/usr/bin/env bash
# Fast production smoke check for the active release and live API.
set -euo pipefail

ROOT="${FC_DEPLOY_ROOT:-/opt/financial-crisis}"
CURRENT_DIR="${FC_CURRENT_DIR:-$ROOT/current}"
API_BASE_URL="${FC_API_BASE_URL:-http://127.0.0.1:18080}"
PUBLIC_URL="${FC_PUBLIC_URL:-}"
EXPECTED_COMMIT=""
ALLOW_STALE=0
MAX_SOURCE_ISSUES="${FC_SMOKE_MAX_SOURCE_ISSUES:-0}"
CHECK_TIMER=1
SKIP_SYSTEMD=0

usage() {
  cat <<'EOF'
Usage: smoke-check.sh [--expected-commit HASH] [--public-url URL] [--allow-stale] [--max-source-issues N] [--skip-timer] [--skip-systemd]

Checks the current release symlink, fc-api, fc-refresh.timer, /health, /api/assessment/current,
/api/sources, key-indicator freshness, and optionally a public web URL.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --expected-commit)
      EXPECTED_COMMIT="${2:-}"
      shift 2
      ;;
    --public-url)
      PUBLIC_URL="${2:-}"
      shift 2
      ;;
    --allow-stale)
      ALLOW_STALE=1
      shift
      ;;
    --max-source-issues)
      MAX_SOURCE_ISSUES="${2:-}"
      shift 2
      ;;
    --skip-timer)
      CHECK_TIMER=0
      shift
      ;;
    --skip-systemd)
      SKIP_SYSTEMD=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

fail() {
  echo "SMOKE_CHECK_FAIL: $*" >&2
  exit 1
}

note() {
  echo "SMOKE_CHECK: $*"
}

[[ -d "$CURRENT_DIR" ]] || fail "current release is missing: $CURRENT_DIR"
[[ -f "$CURRENT_DIR/COMMIT" ]] || fail "current release COMMIT file is missing"
EXPECTED_COMMIT="${EXPECTED_COMMIT//$'\r'/}"
EXPECTED_COMMIT="${EXPECTED_COMMIT//$'\n'/}"
ACTUAL_COMMIT="$(tr -d '\r\n' < "$CURRENT_DIR/COMMIT" 2>/dev/null || true)"
if [[ -n "$EXPECTED_COMMIT" && "${ACTUAL_COMMIT:0:${#EXPECTED_COMMIT}}" != "$EXPECTED_COMMIT" ]]; then
  fail "current commit mismatch: expected $EXPECTED_COMMIT, got $ACTUAL_COMMIT"
fi
note "current release commit: $ACTUAL_COMMIT"

if [[ "$SKIP_SYSTEMD" == "1" ]]; then
  note "skipped systemd service/timer checks"
elif command -v systemctl >/dev/null 2>&1; then
  systemctl is-active --quiet fc-api || fail "fc-api is not active"
  if [[ "$CHECK_TIMER" == "1" ]]; then
    systemctl is-active --quiet fc-refresh.timer || fail "fc-refresh.timer is not active"
    systemctl is-enabled --quiet fc-refresh.timer || fail "fc-refresh.timer is not enabled"
  fi
else
  note "systemctl unavailable, skipped service/timer checks"
fi

command -v node >/dev/null 2>&1 || fail "node is required"

FC_API_BASE_URL="$API_BASE_URL" \
FC_PUBLIC_URL="$PUBLIC_URL" \
FC_SMOKE_ALLOW_STALE="$ALLOW_STALE" \
FC_SMOKE_MAX_SOURCE_ISSUES="$MAX_SOURCE_ISSUES" \
node <<'NODE'
const apiBaseUrl = process.env.FC_API_BASE_URL;
const publicUrl = process.env.FC_PUBLIC_URL;
const allowStale = process.env.FC_SMOKE_ALLOW_STALE === "1";
const maxSourceIssues = Number.parseInt(process.env.FC_SMOKE_MAX_SOURCE_ISSUES ?? "0", 10);

const fail = (message) => {
  console.error(`SMOKE_CHECK_FAIL: ${message}`);
  process.exit(1);
};

const getJson = async (path) => {
  const response = await fetch(`${apiBaseUrl}${path}`);
  if (!response.ok) {
    fail(`${path} returned HTTP ${response.status}`);
  }
  return response.json();
};

const health = await fetch(`${apiBaseUrl}/health`);
if (!health.ok) {
  fail(`/health returned HTTP ${health.status}`);
}

const assessmentResponse = await getJson("/api/assessment/current");
const assessment = assessmentResponse?.assessment ?? assessmentResponse?.data ?? assessmentResponse;
const runtime = assessment?.runtime ?? assessmentResponse?.runtime ?? {};
const dataMode = assessment?.data_mode ?? runtime?.data_mode;
const latestKeyIndicatorAt = assessment?.latest_key_indicator_at ?? runtime?.latest_key_indicator_at;
const staleWarning = assessment?.stale_warning ?? runtime?.stale_warning;

if (dataMode !== "sqlite") {
  fail(`assessment data_mode should be sqlite, got ${dataMode ?? "missing"}`);
}
if (!latestKeyIndicatorAt) {
  fail("assessment latest_key_indicator_at is missing");
}
if (!allowStale && staleWarning) {
  fail(`assessment has stale_warning: ${staleWarning}`);
}

const keyIndicators =
  assessment?.data_freshness?.key_indicators ??
  assessment?.key_indicators ??
  assessmentResponse?.data_freshness?.key_indicators ??
  assessmentResponse?.key_indicators ??
  [];
const staleIndicators = keyIndicators.filter((indicator) => indicator?.status && indicator.status !== "fresh");
if (!allowStale && staleIndicators.length > 0) {
  fail(
    `key indicators are not fresh: ${staleIndicators
      .map((indicator) => `${indicator.indicator_id ?? indicator.label}:${indicator.status}`)
      .join(", ")}`
  );
}

const sources = await getJson("/api/sources");
const sourceItems = Array.isArray(sources) ? sources : Array.isArray(sources?.sources) ? sources.sources : [];
const sourceIssues = sourceItems.filter(
  (source) =>
    source?.production_allowed === true &&
    ["delayed", "partial_failure", "failed"].includes(source?.health?.status)
);
if (sourceIssues.length > maxSourceIssues) {
  fail(
    `production source issues ${sourceIssues.length} exceed ${maxSourceIssues}: ${sourceIssues
      .map((source) => `${source.source_id ?? source.name}:${source.health?.status}`)
      .join(", ")}`
  );
}

if (publicUrl) {
  const response = await fetch(publicUrl);
  if (!response.ok) {
    fail(`public URL returned HTTP ${response.status}: ${publicUrl}`);
  }
  const body = await response.text();
  if (!body.includes("金融危机预警系统")) {
    fail(`public URL did not serve the expected frontend shell: ${publicUrl}`);
  }
}

console.log(
  JSON.stringify(
    {
      status: "ok",
      data_mode: dataMode,
      latest_key_indicator_at: latestKeyIndicatorAt,
      source_issues: sourceIssues.length,
      key_indicators: keyIndicators.length,
      public_url_checked: Boolean(publicUrl),
    },
    null,
    2
  )
);
NODE

note "passed"
