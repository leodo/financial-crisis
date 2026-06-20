#!/usr/bin/env bash
# Financial Crisis 首次部署引导脚本
# 在全新服务器上一次性执行
set -euo pipefail

ROOT="/opt/financial-crisis"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "============================================"
echo " Financial Crisis 首次部署引导"
echo "============================================"
echo "目标目录: $ROOT"
echo "仓库目录: $REPO_DIR"
echo ""

# 检查 root
if [ "$(id -u)" -ne 0 ]; then
  echo "错误: 请用 root 执行 (sudo ./deploy/bootstrap.sh)"
  exit 1
fi

# 1) 系统依赖
echo "[1/8] 检查系统依赖..."
for cmd in cargo node npm systemctl; do
  if ! which "$cmd" >/dev/null 2>&1; then
    echo "  - $cmd: 未安装"
    if [ "$cmd" = "cargo" ]; then
      echo "    -> 安装 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    fi
    if [ "$cmd" = "node" ] || [ "$cmd" = "npm" ]; then
      echo "    -> 安装 Node.js: apt install -y nodejs npm"
    fi
    if [ "$cmd" = "systemctl" ]; then
      echo "    -> 此系统没有 systemd，请改用 cron 模式"
    fi
  else
    echo "  - $cmd: $($cmd --version 2>/dev/null | head -1)"
  fi
done

# 2) 创建目录结构
echo "[2/8] 创建目录结构..."
mkdir -p "$ROOT"/{releases,data,logs,deploy}

# 3) 创建运行用户
echo "[3/8] 创建运行时用户 fc-service..."
if id fc-service >/dev/null 2>&1; then
  echo "  - fc-service 已存在"
else
  useradd --system --no-create-home --shell /usr/sbin/nologin fc-service
  echo "  - fc-service 已创建"
fi

# 4) 初始化数据库
echo "[4/8] 初始化 SQLite 数据库..."
if [ -f "$ROOT/data/fc-local.sqlite" ]; then
  echo "  - 数据库已存在，跳过初始化"
else
  cargo run -p fc-worker -- db init 2>&1 | tail -3
  cargo run -p fc-worker -- db seed 2>&1 | tail -3
  echo "  - 数据库已初始化"
fi
cp "$ROOT/data/fc-local.sqlite" "$ROOT/data/fc-local.sqlite.init"  # 备份初始状态

# 5) 复制环境变量
echo "[5/8] 部署环境变量配置..."
cp "$REPO_DIR/deploy/fc-api.env" "$ROOT/deploy/fc-api.env"
echo "  - 请检查 $ROOT/deploy/fc-api.env 并按需要修改"

# 6) 回填历史数据
echo "[6/8] 回填免费历史数据 (FRED + BOJ + Treasury)..."
echo "    这可能需要几分钟，具体取决于网络..."
cargo run -p fc-worker -- backfill fred --start 2020-01-01 --end 2026-06-30 2>&1 | tail -5
cargo run -p fc-worker -- backfill boj --dataset fx-daily --start 2020-01-01 --end 2026-06-30 2>&1 | tail -5
cargo run -p fc-worker -- backfill treasury-yield --start 2020-01-01 --end 2026-06-30 2>&1 | tail -5
echo "  - 历史数据回填完成"

# 7) 执行首次更新 (编译 + 部署)
echo "[7/8] 执行首次更新 (编译 + 部署)..."
cd "$REPO_DIR"
bash "$SCRIPT_DIR/update.sh"
echo "  - 首次更新完成"

# 8) 安装 systemd 服务
echo "[8/8] 安装 systemd 服务..."
cp "$REPO_DIR/deploy/fc-api.service" "$ROOT/deploy/"
cp "$REPO_DIR/deploy/fc-refresh.service" "$ROOT/deploy/"
cp "$REPO_DIR/deploy/fc-refresh.timer" "$ROOT/deploy/"
cp "$REPO_DIR/deploy/operational-check.sh" "$ROOT/deploy/"
chmod +x "$ROOT/deploy/operational-check.sh"

# 软链接到 systemd 目录
ln -sf "$ROOT/deploy/fc-api.service" /etc/systemd/system/fc-api.service
ln -sf "$ROOT/deploy/fc-refresh.service" /etc/systemd/system/fc-refresh.service
ln -sf "$ROOT/deploy/fc-refresh.timer" /etc/systemd/system/fc-refresh.timer

systemctl daemon-reload
systemctl enable --now fc-api
systemctl enable --now fc-refresh.timer

echo ""
echo "部署验收:"
"$ROOT/deploy/operational-check.sh" --mode bootstrap

echo ""
echo "============================================"
echo " 部署完成！"
echo "============================================"
echo ""
echo "服务状态:"
systemctl status fc-api --no-pager 2>&1 | head -10
echo ""
echo "日频刷新定时器:"
systemctl status fc-refresh.timer --no-pager 2>&1 | head -5
echo ""
echo "验证 API:"
echo "  sudo $ROOT/deploy/operational-check.sh --mode deploy"
echo ""
echo "日常操作:"
echo "  sudo systemctl status fc-api              # 查看 API 状态"
echo "  journalctl -u fc-api -n 50                # 查看最近日志"
echo "  sudo systemctl start fc-refresh.service   # 手动触发数据刷新"
echo "  cd $REPO_DIR && git pull && sudo ./deploy/update.sh  # 更新到最新版本"
echo ""
