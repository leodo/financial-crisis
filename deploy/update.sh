#!/usr/bin/env bash
# Financial Crisis 部署更新脚本
# 用法: sudo ./update.sh [--keep 3]
# 选项:
#   --keep N   保留最近 N 个版本 (默认 3)
set -euo pipefail

KEEP=3
while [[ $# -gt 0 ]]; do
  case "$1" in
    --keep) KEEP="$2"; shift 2 ;;
    *) echo "未知参数: $1"; exit 1 ;;
  esac
done

ROOT="/opt/financial-crisis"
RELEASES_DIR="$ROOT/releases"
LOGS_DIR="$ROOT/logs"
CURRENT_LINK="$ROOT/current"
mkdir -p "$RELEASES_DIR" "$LOGS_DIR" "$ROOT/data" "$ROOT/deploy"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
if [ -f "$SCRIPT_DIR/../Cargo.toml" ]; then
  REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
elif [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
  REPO_DIR="$SCRIPT_DIR"
else
  echo "错误: 无法定位仓库根目录，请从项目 checkout 中运行 update.sh"
  exit 1
fi
TIMESTAMP=$(date -u +%Y%m%d_%H%M%S)
RELEASE_DIR="$RELEASES_DIR/v$TIMESTAMP"

# 日志函数
log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" | tee -a "$LOGS_DIR/update.log"; }

repair_runtime_permissions() {
  mkdir -p "$ROOT/data" "$ROOT/logs"
  if id fc-service >/dev/null 2>&1; then
    chown -R fc-service:fc-service "$ROOT/data" "$ROOT/logs"
    chmod 750 "$ROOT/data" "$ROOT/logs"
    find "$ROOT/data" -maxdepth 1 -name 'fc-local.sqlite*' -exec chmod 660 {} + 2>/dev/null || true
  else
    log "警告: fc-service 用户不存在，跳过运行时权限修复"
  fi
}

sync_deploy_files() {
  mkdir -p "$ROOT/deploy"
  cp "$REPO_DIR/deploy/fc-api.service" "$ROOT/deploy/"
  cp "$REPO_DIR/deploy/fc-refresh.service" "$ROOT/deploy/"
  cp "$REPO_DIR/deploy/fc-refresh.timer" "$ROOT/deploy/"
  cp "$REPO_DIR/deploy/operational-check.sh" "$ROOT/deploy/"
  cp "$REPO_DIR/deploy/smoke-check.sh" "$ROOT/deploy/"
  chmod +x "$ROOT/deploy/operational-check.sh" "$ROOT/deploy/smoke-check.sh"

  if [ "$(id -u)" -eq 0 ] && command -v systemctl >/dev/null 2>&1; then
    ln -sf "$ROOT/deploy/fc-api.service" /etc/systemd/system/fc-api.service
    ln -sf "$ROOT/deploy/fc-refresh.service" /etc/systemd/system/fc-refresh.service
    ln -sf "$ROOT/deploy/fc-refresh.timer" /etc/systemd/system/fc-refresh.timer
    systemctl daemon-reload
    if systemctl is-enabled --quiet fc-refresh.timer 2>/dev/null; then
      systemctl enable --now fc-refresh.timer >/dev/null 2>&1 || true
    fi
  else
    log "警告: 不是 root 或 systemctl 不可用，跳过 systemd unit 同步"
  fi
}

log "=== 开始更新: v$TIMESTAMP ==="
log "脚本目录: $SCRIPT_DIR"
log "仓库目录: $REPO_DIR"
repair_runtime_permissions

# 1) 拉取最新代码
log "[1/8] 拉取最新代码..."
cd "$REPO_DIR"
git fetch origin main
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ]; then
  log "警告: 当前不在 main 分支 ($CURRENT_BRANCH)，切换到 main"
  git checkout main
fi

git merge origin/main --ff-only
COMMIT_HASH=$(git rev-parse --short HEAD)
COMMIT_MSG=$(git log -1 --pretty=%s)
log "当前 commit: $COMMIT_HASH - $COMMIT_MSG"

# 检查是否有未提交修改
if ! git diff --quiet; then
  log "错误: 工作区有未提交的修改，请先提交或 stash"
  exit 1
fi

# 2) 编译 Rust 后端
log "[2/8] 编译 Rust 后端 (release)..."
cargo build --release --workspace 2>&1 | tee -a "$LOGS_DIR/update.log"
log "编译完成"

# 3) 构建前端
log "[3/8] 构建前端..."
cd "$REPO_DIR/apps/web"
npm ci 2>&1 | tee -a "$LOGS_DIR/update.log"
npm run build 2>&1 | tee -a "$LOGS_DIR/update.log"
log "前端构建完成"
cd "$REPO_DIR"

# 4) 创建版本目录
log "[4/8] 创建 release 目录: $RELEASE_DIR"
mkdir -p "$RELEASE_DIR"/{bin,web,config,scripts}
mkdir -p "$ROOT/deploy"

# 复制二进制
cp target/release/fc-api "$RELEASE_DIR/bin/"
cp target/release/fc-worker "$RELEASE_DIR/bin/"
strip "$RELEASE_DIR/bin/fc-api" "$RELEASE_DIR/bin/fc-worker"

# 复制前端构建产物
cp -r apps/web/dist "$RELEASE_DIR/web/dist"

# 复制运行时配置
cp -r config/ "$RELEASE_DIR/config/"
cp -r scripts/*.ps1 "$RELEASE_DIR/scripts/" 2>/dev/null || true
cp -r scripts/*.mjs "$RELEASE_DIR/scripts/" 2>/dev/null || true
cp justfile "$RELEASE_DIR/" 2>/dev/null || true
sync_deploy_files
repair_runtime_permissions

# 保留 commit 信息
echo "$COMMIT_HASH" > "$RELEASE_DIR/COMMIT"
echo "$COMMIT_MSG" > "$RELEASE_DIR/COMMIT_MSG"
echo "$TIMESTAMP" > "$RELEASE_DIR/RELEASE_DATE"

# 5) 创建/更新 current 软链接
log "[5/8] 更新 current 符号链接..."
ln -sfn "$RELEASE_DIR" "$CURRENT_LINK.tmp"
mv -Tf "$CURRENT_LINK.tmp" "$CURRENT_LINK"
log "current -> $RELEASE_DIR"

# 6) 重启服务
log "[6/8] 重启 API 服务..."
API_SERVICE_RESTARTED=0
if systemctl is-active --quiet fc-api 2>/dev/null; then
  systemctl restart fc-api
  log "fc-api 已重启"
  API_SERVICE_RESTARTED=1
else
  log "警告: fc-api service 未运行，跳过重启与本阶段部署验收；首次部署会在 enable 服务后验收"
fi

# 7) 部署验收
log "[7/8] 运行部署验收..."
if [ "$API_SERVICE_RESTARTED" = "1" ]; then
  "$ROOT/deploy/operational-check.sh" --mode deploy 2>&1 | tee -a "$LOGS_DIR/update.log"
  "$ROOT/deploy/smoke-check.sh" --expected-commit "$COMMIT_HASH" 2>&1 | tee -a "$LOGS_DIR/update.log"
  log "部署验收完成"
else
  log "部署验收已跳过"
fi

# 8) 清理旧版本
log "[8/8] 清理旧版本 (保留最近 $KEEP 个)..."
cd "$RELEASES_DIR"
ls -d v* 2>/dev/null | sort -r | tail -n +$((KEEP + 1)) | while read -r OLD; do
  log "  删除旧版本: $OLD"
  rm -rf "$RELEASES_DIR/$OLD"
done

log "=== 更新完成: v$TIMESTAMP ==="
log "当前二进制: $CURRENT_LINK/bin/fc-api ($(du -h "$CURRENT_LINK/bin/fc-api" | cut -f1))"
log "当前前端: $CURRENT_LINK/web/dist ($(du -sh "$CURRENT_LINK/web/dist" | cut -f1))"
log ""
log "快速验证:"
log "  systemctl status fc-api              # 查看 API 服务状态"
log "  journalctl -u fc-api -n 30 --no-pager # 最近 30 行日志"
log "  curl http://127.0.0.1:18080/api/assessment/current | head -c 200  # API 响应"
