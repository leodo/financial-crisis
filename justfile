set shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

# 默认入口：列出所有可用命令和说明。
default:
    @just --list

# 查看命令列表。第一次不知道怎么启动时，先运行 `just` 或 `just help`。
help:
    @just --list

# 一键启动本地开发环境：后台启动 Rust API 和 React Web 面板。
dev:
    ./scripts/dev-start.ps1

# 查看一键启动的服务状态、PID 和访问地址。
status:
    ./scripts/dev-status.ps1

# 停止 `just dev` 启动的 API 和 Web 服务。
stop:
    ./scripts/dev-stop.ps1

# 格式化全部 Rust 代码。
fmt:
    cargo fmt --all

# 运行 Rust workspace 的全部测试。
test:
    cargo test --workspace

# 运行 Rust clippy，按 warning 即失败处理。
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# 只启动后端 API，默认地址 http://127.0.0.1:18080。
api:
    cargo run -p fc-api

# 只运行 worker 的一次 demo 抓取流程。
worker:
    cargo run -p fc-worker

# 初始化本地 SQLite 数据库，默认路径 data/fc-local.sqlite。
db-init:
    cargo run -p fc-worker -- db init

# 写入本地 SQLite 所需的 FRED、Treasury 元数据和首批指标映射。
db-seed:
    cargo run -p fc-worker -- db seed

# 无需 API key，使用 FRED 图表 CSV 回填历史数据到本地 SQLite。
backfill-fred:
    cargo run -p fc-worker -- backfill fred

# 无需 API key，使用 FRED 图表 CSV 回填指定日期范围的历史数据。
backfill-fred-range start end:
    cargo run -p fc-worker -- backfill fred --start {{start}} --end {{end}}

# 可选增强：使用官方 FRED API 回填历史数据，需要先设置 FRED_API_KEY。
backfill-fred-api:
    cargo run -p fc-worker -- backfill fred --api

# 可选增强：使用官方 FRED API 回填指定日期范围，需要先设置 FRED_API_KEY。
backfill-fred-api-range start end:
    cargo run -p fc-worker -- backfill fred --api --start {{start}} --end {{end}}

# 无需 API key，使用美国财政部官方收益率曲线作为利率数据兜底源。
backfill-treasury-yield:
    cargo run -p fc-worker -- backfill treasury-yield

# 无需 API key，回填指定日期范围的美国财政部收益率曲线。
backfill-treasury-yield-range start end:
    cargo run -p fc-worker -- backfill treasury-yield --start {{start}} --end {{end}}

# 安装前端依赖。首次运行前端前需要执行一次。
web-install:
    cd apps/web; npm install

# 只启动前端 Vite 开发服务器，默认地址 http://127.0.0.1:5173。
web-dev:
    cd apps/web; npm run dev

# 构建前端生产包。
web-build:
    cd apps/web; npm run build

# 常用检查：格式化、Rust 测试、前端构建。
check-all: fmt test web-build

# 完整检查：格式化、Rust 测试、clippy、前端构建。
verify: fmt test lint web-build
