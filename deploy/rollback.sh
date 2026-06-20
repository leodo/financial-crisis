#!/usr/bin/env bash
# Financial Crisis 回滚脚本
# 用法: sudo ./rollback.sh [版本名]
#   不指定版本名时回滚到上一个版本
set -euo pipefail

ROOT="/opt/financial-crisis"
RELEASES_DIR="$ROOT/releases"
CURRENT_LINK="$ROOT/current"
LOGS_DIR="$ROOT/logs"

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOGS_DIR/update.log"; }

# 确定当前和上一个版本
CURRENT_TARGET=$(readlink "$CURRENT_LINK")
CURRENT_NAME=$(basename "$CURRENT_TARGET")

if [ $# -ge 1 ]; then
  TARGET_NAME="$1"
  TARGET_DIR="$RELEASES_DIR/$TARGET_NAME"
else
  # 自动找上一个版本（按名字排序的倒数第二个）
  TARGET_NAME=$(ls -d "$RELEASES_DIR"/v* 2>/dev/null | sort -r | sed -n '2p' | xargs basename)
  TARGET_DIR="$RELEASES_DIR/$TARGET_NAME"
fi

if [ -z "$TARGET_NAME" ] || [ ! -d "$TARGET_DIR" ]; then
  log "错误: 找不到目标版本 '$TARGET_NAME'"
  echo "可用版本:"
  ls "$RELEASES_DIR" | sort -r
  exit 1
fi

if [ "$TARGET_DIR" = "$CURRENT_TARGET" ]; then
  log "当前已是 $TARGET_NAME，无需回滚"
  exit 0
fi

log "=== 回滚: $CURRENT_NAME -> $TARGET_NAME ==="

# 切换 symlink
ln -sfn "$TARGET_DIR" "$CURRENT_LINK.tmp"
mv -Tf "$CURRENT_LINK.tmp" "$CURRENT_LINK"
log "current -> $TARGET_DIR"

# 重启服务
systemctl restart fc-api 2>/dev/null || true
log "fc-api 已重启"

"$ROOT/deploy/operational-check.sh" --mode rollback 2>&1 | tee -a "$LOGS_DIR/update.log"
log "回滚验收完成"

log "=== 回滚完成: $TARGET_NAME ==="
