#!/usr/bin/env bash
# Restore the production MVP SQLite database from a verified backup.
set -euo pipefail

ROOT="${FC_DEPLOY_ROOT:-/opt/financial-crisis}"
DB_PATH="${FC_SQLITE_PATH:-$ROOT/data/fc-local.sqlite}"
BACKUP_PATH=""

usage() {
  cat <<'EOF'
Usage: sqlite-restore.sh --backup PATH [--db PATH] [--yes] [--skip-service] [--skip-smoke]

Verifies the backup with PRAGMA integrity_check, stops fc-api when systemd is available,
copies the current database aside, restores the backup atomically, repairs runtime
permissions, restarts fc-api, and runs smoke-check if available.
EOF
}

YES=0
SKIP_SERVICE=0
SKIP_SMOKE=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --backup)
      BACKUP_PATH="${2:-}"
      shift 2
      ;;
    --db)
      DB_PATH="${2:-}"
      shift 2
      ;;
    --yes)
      YES=1
      shift
      ;;
    --skip-service)
      SKIP_SERVICE=1
      shift
      ;;
    --skip-smoke)
      SKIP_SMOKE=1
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
  echo "SQLITE_RESTORE_FAIL: $*" >&2
  exit 1
}

[[ -n "$BACKUP_PATH" ]] || fail "--backup is required"
[[ -f "$BACKUP_PATH" ]] || fail "backup not found: $BACKUP_PATH"
command -v sqlite3 >/dev/null 2>&1 || fail "sqlite3 is required"

integrity="$(sqlite3 "$BACKUP_PATH" 'PRAGMA integrity_check;')"
[[ "$integrity" == "ok" ]] || fail "backup integrity check failed: $integrity"

if [[ "$YES" != "1" ]]; then
  fail "refusing to restore without --yes"
fi

mkdir -p "$(dirname "$DB_PATH")"
timestamp="$(date -u +%Y%m%d-%H%M%S)"
pre_restore="$DB_PATH.pre-restore-$timestamp"
tmp_restore="$DB_PATH.restore-tmp"

if [[ "$SKIP_SERVICE" != "1" ]] && command -v systemctl >/dev/null 2>&1; then
  systemctl stop fc-api 2>/dev/null || true
fi

if [[ -f "$DB_PATH" ]]; then
  cp -p "$DB_PATH" "$pre_restore"
fi

rm -f "$tmp_restore"
cp "$BACKUP_PATH" "$tmp_restore"
sqlite3 "$tmp_restore" 'PRAGMA integrity_check;' | grep -qx ok || fail "restored copy integrity check failed"
rm -f "$DB_PATH-wal" "$DB_PATH-shm"
mv "$tmp_restore" "$DB_PATH"

if id fc-service >/dev/null 2>&1; then
  chown fc-service:fc-service "$DB_PATH" "$DB_PATH"* 2>/dev/null || true
  chmod 660 "$DB_PATH" "$DB_PATH"* 2>/dev/null || true
fi

if [[ "$SKIP_SERVICE" != "1" ]] && command -v systemctl >/dev/null 2>&1; then
  systemctl start fc-api
fi

if [[ "$SKIP_SMOKE" != "1" && -x "$ROOT/deploy/smoke-check.sh" ]]; then
  "$ROOT/deploy/smoke-check.sh" --expected-commit "$(cat "$ROOT/current/COMMIT" 2>/dev/null || true)"
fi

echo "Restored $DB_PATH from $BACKUP_PATH"
if [[ -f "$pre_restore" ]]; then
  echo "Previous database saved at $pre_restore"
fi
