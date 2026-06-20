#!/usr/bin/env bash
# Shared post-deploy / post-refresh checks for the production runtime.
set -euo pipefail

ROOT="${FC_DEPLOY_ROOT:-/opt/financial-crisis}"
CURRENT_DIR="${FC_CURRENT_DIR:-$ROOT/current}"
LOGS_DIR="${FC_LOGS_DIR:-$ROOT/logs}"
API_BASE_URL="${FC_API_BASE_URL:-http://127.0.0.1:18080}"
MODE="deploy"
RUN_DEPLOY_CHECK=1
RUN_DAILY_HEALTH=0
RUN_RISK_THRESHOLD=0
SKIP_WEB="auto"
CHECK_EXIT_CODE=0
REPORT_PATHS=()

usage() {
  cat <<'EOF'
Usage: operational-check.sh [--mode deploy|bootstrap|rollback|refresh] [--skip-web] [--with-daily-health] [--with-risk-threshold]

Runs the Node-based deployment check and/or daily health report against the current release.
By default, deploy/bootstrap/rollback run deploy-check. Refresh runs deploy-check plus daily health.
If FC_WEB_BASE_URL is not set, deploy-check skips the web root because production web is usually served by nginx.
When FC_ALERT_* webhook variables are configured, failed checks send an operational alert.
Refresh mode also writes a business risk-threshold report and sends reminder-only alerts when configured.
EOF
}

wait_for_api() {
  local attempts="${FC_OPERATIONAL_CHECK_WAIT_ATTEMPTS:-30}"
  local delay="${FC_OPERATIONAL_CHECK_WAIT_SECS:-2}"
  local i

  for ((i = 1; i <= attempts; i++)); do
    if FC_API_BASE_URL="$API_BASE_URL" node -e 'fetch(`${process.env.FC_API_BASE_URL}/health`).then((r) => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1));' >/dev/null 2>&1; then
      return 0
    fi
    sleep "$delay"
  done

  echo "API did not become healthy after $attempts attempts: $API_BASE_URL/health" >&2
  return 1
}

send_operational_alert() {
  local status="$1"
  local message="$2"

  if [[ "${FC_ALERT_ON_SUCCESS:-0}" != "1" && "$status" == "ok" ]]; then
    return 0
  fi

  if [[ ! -f scripts/operational-alert.mjs ]]; then
    echo "Skipping operational alert: scripts/operational-alert.mjs is missing" >&2
    return 0
  fi

  local alert_args=(scripts/operational-alert.mjs --mode "$MODE" --status "$status" --message "$message")
  local report_path
  for report_path in "${REPORT_PATHS[@]}"; do
    if [[ -n "$report_path" && -f "$report_path" ]]; then
      alert_args+=(--report "$report_path")
    fi
  done

  node "${alert_args[@]}" || true
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    --skip-web)
      SKIP_WEB="yes"
      shift
      ;;
    --with-daily-health)
      RUN_DAILY_HEALTH=1
      shift
      ;;
    --with-risk-threshold)
      RUN_RISK_THRESHOLD=1
      shift
      ;;
    --daily-health-only)
      RUN_DEPLOY_CHECK=0
      RUN_DAILY_HEALTH=1
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

case "$MODE" in
  deploy|bootstrap|rollback)
    ;;
  refresh)
    RUN_DEPLOY_CHECK="${FC_RUN_DEPLOY_CHECK_AFTER_REFRESH:-1}"
    RUN_DAILY_HEALTH=1
    RUN_RISK_THRESHOLD="${FC_RUN_RISK_THRESHOLD_AFTER_REFRESH:-1}"
    SKIP_WEB="yes"
    ;;
  *)
    echo "Unknown mode: $MODE" >&2
    usage >&2
    exit 2
    ;;
esac

mkdir -p "$LOGS_DIR"

if [[ ! -d "$CURRENT_DIR" ]]; then
  echo "Current release directory is missing: $CURRENT_DIR" >&2
  exit 1
fi

if ! command -v node >/dev/null 2>&1; then
  echo "node is required for operational checks" >&2
  exit 1
fi

cd "$CURRENT_DIR"

timestamp="$(date -u +%Y%m%d-%H%M%S)"
deploy_report="$LOGS_DIR/deploy-check-${MODE}-${timestamp}.md"
health_report="$LOGS_DIR/daily-health-${MODE}-${timestamp}.md"
risk_threshold_report="$LOGS_DIR/risk-threshold-${MODE}-${timestamp}.md"

if ! wait_for_api; then
  readiness_report="$LOGS_DIR/operational-check-${MODE}-${timestamp}.md"
  {
    echo "# Operational Check"
    echo ""
    echo "- Status: ATTENTION"
    echo "- Mode: $MODE"
    echo "- API: $API_BASE_URL"
    echo "- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo ""
    echo "## Failure"
    echo ""
    echo "- API did not become healthy before the check timeout."
  } > "$readiness_report"
  REPORT_PATHS+=("$readiness_report")
  send_operational_alert "attention" "API readiness check failed before deploy/daily health reports could run."
  exit 1
fi

deploy_args=(scripts/deploy-check.mjs --fail-on-issues --output "$deploy_report")
if [[ "$SKIP_WEB" == "yes" || ( "$SKIP_WEB" == "auto" && -z "${FC_WEB_BASE_URL:-}" ) ]]; then
  deploy_args+=(--skip-web)
fi

if [[ "$RUN_DEPLOY_CHECK" == "1" ]]; then
  if [[ ! -f scripts/deploy-check.mjs ]]; then
    echo "Missing scripts/deploy-check.mjs in $CURRENT_DIR" >&2
    exit 1
  fi
  echo "Running deployment check -> $deploy_report"
  REPORT_PATHS+=("$deploy_report")
  set +e
  FC_API_BASE_URL="$API_BASE_URL" node "${deploy_args[@]}"
  deploy_exit_code=$?
  set -e
  if [[ "$deploy_exit_code" -ne 0 ]]; then
    CHECK_EXIT_CODE="$deploy_exit_code"
  fi
fi

if [[ "$RUN_DAILY_HEALTH" == "1" ]]; then
  if [[ ! -f scripts/daily-health-report.mjs ]]; then
    echo "Missing scripts/daily-health-report.mjs in $CURRENT_DIR" >&2
    exit 1
  fi
  echo "Running daily health report -> $health_report"
  REPORT_PATHS+=("$health_report")
  set +e
  FC_API_BASE_URL="$API_BASE_URL" node scripts/daily-health-report.mjs \
    --fail-on-issues \
    --output "$health_report"
  health_exit_code=$?
  set -e
  if [[ "$health_exit_code" -ne 0 && "$CHECK_EXIT_CODE" -eq 0 ]]; then
    CHECK_EXIT_CODE="$health_exit_code"
  fi
fi

if [[ "$RUN_RISK_THRESHOLD" == "1" ]]; then
  if [[ ! -f scripts/risk-threshold-alert.mjs ]]; then
    echo "Missing scripts/risk-threshold-alert.mjs in $CURRENT_DIR" >&2
    exit 1
  fi
  echo "Running risk threshold check -> $risk_threshold_report"
  REPORT_PATHS+=("$risk_threshold_report")
  set +e
  FC_API_BASE_URL="$API_BASE_URL" node scripts/risk-threshold-alert.mjs \
    --output "$risk_threshold_report"
  risk_threshold_exit_code=$?
  set -e
  if [[ "$risk_threshold_exit_code" -ne 0 && "$CHECK_EXIT_CODE" -eq 0 ]]; then
    CHECK_EXIT_CODE="$risk_threshold_exit_code"
  fi
fi

if [[ "$CHECK_EXIT_CODE" -ne 0 ]]; then
  send_operational_alert "attention" "Operational check failed with exit code $CHECK_EXIT_CODE."
  exit "$CHECK_EXIT_CODE"
fi

send_operational_alert "ok" "Operational check passed."
