#!/usr/bin/env bash
# Non-destructive backup/restore drill using a temporary SQLite database.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DRILL_ROOT="${FC_SQLITE_DRILL_ROOT:-}"
KEEP_ROOT=0

usage() {
  cat <<'EOF'
Usage: sqlite-drill.sh [--keep-root]

Creates a temporary SQLite database, runs sqlite-backup.sh against it, mutates the DB,
restores from the backup with sqlite-restore.sh in --skip-service mode, and verifies
the original row is recovered. This never touches the production database.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --keep-root)
      KEEP_ROOT=1
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
  echo "SQLITE_DRILL_FAIL: $*" >&2
  exit 1
}

command -v sqlite3 >/dev/null 2>&1 || fail "sqlite3 is required"

if [[ -z "$DRILL_ROOT" ]]; then
  DRILL_ROOT="$(mktemp -d)"
fi
mkdir -p "$DRILL_ROOT/data" "$DRILL_ROOT/backups/sqlite"

cleanup() {
  if [[ "$KEEP_ROOT" != "1" ]]; then
    rm -rf "$DRILL_ROOT"
  fi
}
trap cleanup EXIT

db_path="$DRILL_ROOT/data/fc-local.sqlite"
sqlite3 "$db_path" <<'SQL'
CREATE TABLE drill_state(id INTEGER PRIMARY KEY, label TEXT NOT NULL);
INSERT INTO drill_state(id, label) VALUES (1, 'before-restore');
SQL

backup_path="$("$SCRIPT_DIR/sqlite-backup.sh" --db "$db_path" --backup-dir "$DRILL_ROOT/backups/sqlite" --keep 2)"
[[ -f "$backup_path" ]] || fail "backup was not created"

sqlite3 "$db_path" "UPDATE drill_state SET label = 'after-mutation' WHERE id = 1;"
"$SCRIPT_DIR/sqlite-restore.sh" --backup "$backup_path" --db "$db_path" --yes --skip-service --skip-smoke >/dev/null

label="$(sqlite3 "$db_path" "SELECT label FROM drill_state WHERE id = 1;")"
[[ "$label" == "before-restore" ]] || fail "restore drill expected before-restore, got $label"

echo "SQLite backup/restore drill passed: $DRILL_ROOT"
