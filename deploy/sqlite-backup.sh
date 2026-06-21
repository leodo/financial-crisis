#!/usr/bin/env bash
# Create a consistent SQLite backup for the production MVP database.
set -euo pipefail

ROOT="${FC_DEPLOY_ROOT:-/opt/financial-crisis}"
DB_PATH="${FC_SQLITE_PATH:-$ROOT/data/fc-local.sqlite}"
BACKUP_DIR="${FC_BACKUP_DIR:-$ROOT/backups/sqlite}"
KEEP="${FC_BACKUP_KEEP:-14}"

usage() {
  cat <<'EOF'
Usage: sqlite-backup.sh [--db PATH] [--backup-dir DIR] [--keep N]

Creates a SQLite-safe backup using VACUUM INTO when sqlite3 is available, then verifies
the backup with PRAGMA integrity_check. Older backups are pruned by filename order.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --db)
      DB_PATH="${2:-}"
      shift 2
      ;;
    --backup-dir)
      BACKUP_DIR="${2:-}"
      shift 2
      ;;
    --keep)
      KEEP="${2:-}"
      shift 2
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
  echo "SQLITE_BACKUP_FAIL: $*" >&2
  exit 1
}

[[ -f "$DB_PATH" ]] || fail "database not found: $DB_PATH"
command -v sqlite3 >/dev/null 2>&1 || fail "sqlite3 is required"

mkdir -p "$BACKUP_DIR"
timestamp="$(date -u +%Y%m%d-%H%M%S)"
backup_path="$BACKUP_DIR/fc-local-$timestamp.sqlite"
tmp_path="$backup_path.tmp"

rm -f "$tmp_path"
sqlite3 "$DB_PATH" "VACUUM INTO '$tmp_path';"
integrity="$(sqlite3 "$tmp_path" 'PRAGMA integrity_check;')"
if [[ "$integrity" != "ok" ]]; then
  rm -f "$tmp_path"
  fail "backup integrity check failed: $integrity"
fi

mv "$tmp_path" "$backup_path"
chmod 640 "$backup_path" 2>/dev/null || true

if [[ "$KEEP" =~ ^[0-9]+$ && "$KEEP" -gt 0 ]]; then
  find "$BACKUP_DIR" -maxdepth 1 -type f -name 'fc-local-*.sqlite' | sort -r | tail -n +$((KEEP + 1)) | while read -r old_backup; do
    rm -f "$old_backup"
  done
fi

echo "$backup_path"
