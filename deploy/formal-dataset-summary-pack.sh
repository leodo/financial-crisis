#!/usr/bin/env bash
# Export formal dataset summary evidence from the production deployment root.
set -euo pipefail

ROOT="${FC_DEPLOY_ROOT:-/opt/financial-crisis}"
CURRENT_DIR="${FC_CURRENT_DIR:-$ROOT/current}"

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  cat <<EOF
Usage: formal-dataset-summary-pack.sh [extra node script args...]

Runs current/scripts/formal-dataset-summary-pack.mjs from the deployment root so
the API can read generated evidence from artifacts/research/dataset-summary-check.

Environment:
  FC_DEPLOY_ROOT                         Deployment root, default /opt/financial-crisis
  FC_CURRENT_DIR                         Current release dir, default \$FC_DEPLOY_ROOT/current
  FC_FORMAL_DATASET_SUMMARY_OUTPUT_DIR   Output dir, default \$FC_DEPLOY_ROOT/artifacts/research/dataset-summary-check
  FC_MARKET_SCOPE                        Market scope, default financial_system
EOF
  exit 0
fi

[[ -d "$ROOT" ]] || { echo "FORMAL_DATASET_SUMMARY_FAIL: deployment root missing: $ROOT" >&2; exit 1; }
[[ -x "$CURRENT_DIR/bin/fc-worker" ]] || { echo "FORMAL_DATASET_SUMMARY_FAIL: fc-worker missing: $CURRENT_DIR/bin/fc-worker" >&2; exit 1; }
[[ -f "$CURRENT_DIR/scripts/formal-dataset-summary-pack.mjs" ]] || { echo "FORMAL_DATASET_SUMMARY_FAIL: script missing: $CURRENT_DIR/scripts/formal-dataset-summary-pack.mjs" >&2; exit 1; }

if [[ -f "$ROOT/deploy/fc-api.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "$ROOT/deploy/fc-api.env"
  set +a
fi

OUTPUT_DIR="${FC_FORMAL_DATASET_SUMMARY_OUTPUT_DIR:-$ROOT/artifacts/research/dataset-summary-check}"
MARKET_SCOPE="${FC_MARKET_SCOPE:-financial_system}"

cd "$ROOT"
mkdir -p "$OUTPUT_DIR"
exec node "$CURRENT_DIR/scripts/formal-dataset-summary-pack.mjs" \
  --market-scope "$MARKET_SCOPE" \
  --output-dir "$OUTPUT_DIR" \
  --worker-bin "$CURRENT_DIR/bin/fc-worker" \
  "$@"
