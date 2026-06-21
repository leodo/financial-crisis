#!/usr/bin/env bash
# Export release-review-derived formal candidate audit evidence from production.
set -euo pipefail

ROOT="${FC_DEPLOY_ROOT:-/opt/financial-crisis}"
CURRENT_DIR="${FC_CURRENT_DIR:-$ROOT/current}"
BASELINE_RELEASE_ID=""
CANDIDATE_RELEASE_ID=""
HISTORY_MODE="${FC_RELEASE_REVIEW_HISTORY_MODE:-strict_rebuild}"
REPORT_PATH=""

usage() {
  cat <<'EOF'
Usage: formal-candidate-audit-pack.sh --baseline-release-id ID --candidate-release-id ID [options]

Generates lead-time and cooldown / false-positive audit JSON artifacts from an
existing release-review JSON artifact. It does not train, activate, reload, or
switch releases.

Options:
  --baseline-release-id ID    Baseline release id
  --candidate-release-id ID   Candidate release id
  --history-mode MODE         Release-review history mode, default strict_rebuild
  --report-path PATH          Explicit release-review JSON path

Environment:
  FC_DEPLOY_ROOT              Deployment root, default /opt/financial-crisis
  FC_CURRENT_DIR              Current release dir, default $FC_DEPLOY_ROOT/current
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --baseline-release-id)
      BASELINE_RELEASE_ID="${2:-}"
      shift 2
      ;;
    --candidate-release-id)
      CANDIDATE_RELEASE_ID="${2:-}"
      shift 2
      ;;
    --history-mode)
      HISTORY_MODE="${2:-}"
      shift 2
      ;;
    --report-path)
      REPORT_PATH="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "FORMAL_CANDIDATE_AUDIT_FAIL: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

[[ -n "$BASELINE_RELEASE_ID" ]] || { echo "FORMAL_CANDIDATE_AUDIT_FAIL: --baseline-release-id is required" >&2; exit 2; }
[[ -n "$CANDIDATE_RELEASE_ID" ]] || { echo "FORMAL_CANDIDATE_AUDIT_FAIL: --candidate-release-id is required" >&2; exit 2; }
[[ -d "$ROOT" ]] || { echo "FORMAL_CANDIDATE_AUDIT_FAIL: deployment root missing: $ROOT" >&2; exit 1; }
[[ -f "$CURRENT_DIR/scripts/formal-candidate-leadtime-audit.mjs" ]] || { echo "FORMAL_CANDIDATE_AUDIT_FAIL: script missing: $CURRENT_DIR/scripts/formal-candidate-leadtime-audit.mjs" >&2; exit 1; }
[[ -f "$CURRENT_DIR/scripts/formal-candidate-cooldown-audit.mjs" ]] || { echo "FORMAL_CANDIDATE_AUDIT_FAIL: script missing: $CURRENT_DIR/scripts/formal-candidate-cooldown-audit.mjs" >&2; exit 1; }

cd "$ROOT"

COMMON_ARGS=(
  --baseline-release-id "$BASELINE_RELEASE_ID"
  --candidate-release-id "$CANDIDATE_RELEASE_ID"
  --history-mode "$HISTORY_MODE"
)
if [[ -n "$REPORT_PATH" ]]; then
  COMMON_ARGS+=(--report-path "$REPORT_PATH")
fi

node "$CURRENT_DIR/scripts/formal-candidate-leadtime-audit.mjs" "${COMMON_ARGS[@]}"
node "$CURRENT_DIR/scripts/formal-candidate-cooldown-audit.mjs" "${COMMON_ARGS[@]}"
