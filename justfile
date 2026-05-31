set shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-ExecutionPolicy", "Bypass", "-Command"]

# 默认入口：列出所有可用命令和说明。
default:
    @just --list

# 查看命令列表。第一次不知道怎么启动时，先运行 `just` 或 `just help`。
help:
    @just --list

# 一键启动本地开发环境：后台启动 Rust API 和 React Web 面板。
# 如果 data/fc-local.sqlite 已存在，API 默认读取本地 SQLite；否则退回 demo 数据。
dev:
    ./scripts/dev-start.ps1

# 一键启动并强制 API 使用本地 SQLite 数据。
# 适合已经执行过 `just db-init` / `just db-seed` / backfill 的场景。
dev-sqlite:
    $env:FC_DATA_MODE='sqlite'
    $env:FC_SQLITE_PATH='data/fc-local.sqlite'
    ./scripts/dev-start.ps1

# 查看一键启动的服务状态、PID、访问地址，以及当前 data mode / 最新观测 / USDJPY。
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

# 检查本地 SQLite 关键指标是否足够新鲜，避免把旧数据误当成当前市场值。
db-check:
    cargo run -p fc-worker -- db check

# 一键刷新最近一段免费高频数据，并在 API 运行时自动触发 /api/system/reload。
# 当前会串行刷新 FRED / Treasury / BOJ / SEC EDGAR，World Bank 可按需关闭。
# 这是日常维护本地评估库的首选入口。
refresh-latest:
    cargo run -p fc-worker -- refresh latest-free
    ./scripts/dev-status.ps1

# 在日常刷新基础上追加 GDELT prototype 事件源。
# 适合想把新闻压力序列也一并拉进当前面板的场景。
refresh-latest-full:
    cargo run -p fc-worker -- refresh latest-free --include-gdelt
    ./scripts/dev-status.ps1

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

# 无需 API key，回填 World Bank 年频慢变量。
backfill-world-bank:
    cargo run -p fc-worker -- backfill world-bank

# 无需 API key，回填指定日期范围的 World Bank 年频慢变量。
backfill-world-bank-range start end:
    cargo run -p fc-worker -- backfill world-bank --start {{start}} --end {{end}}

# 无需 API key，回填 SEC EDGAR 银行/金融机构公告事件，并写入本地告警。
backfill-sec-edgar:
    cargo run -p fc-worker -- backfill sec-edgar

# 无需 API key，按日期范围回填 SEC EDGAR 公告事件。
backfill-sec-edgar-range start end:
    cargo run -p fc-worker -- backfill sec-edgar --start {{start}} --end {{end}}

# 无需 API key，回填 GDELT 金融压力新闻聚合序列。
# 当前属于 prototype 信号，建议按需手动回填，不默认放进 refresh-latest。
backfill-gdelt:
    cargo run -p fc-worker -- backfill gdelt

# 无需 API key，按日期范围回填 GDELT 新闻聚合序列。
backfill-gdelt-range start end:
    cargo run -p fc-worker -- backfill gdelt --start {{start}} --end {{end}}

# 无需 API key，回填日元套息监控所需的 USDJPY 历史数据。
# 当前优先使用 BOJ 官方 USDJPY；失败时自动回退到 FRED 免费序列。
backfill-jpy-carry:
    cargo run -p fc-worker -- backfill jpy-carry

# 无需 API key，回填指定日期范围的 USDJPY 历史数据。
backfill-jpy-carry-range start end:
    cargo run -p fc-worker -- backfill jpy-carry --start {{start}} --end {{end}}

# 无需 API key，直接回填 BOJ 官方 USDJPY 历史数据。
backfill-boj-fx:
    cargo run -p fc-worker -- backfill boj --dataset fx-daily

# 无需 API key，按日期范围回填 BOJ 官方 USDJPY 历史数据。
backfill-boj-fx-range start end:
    cargo run -p fc-worker -- backfill boj --dataset fx-daily --start {{start}} --end {{end}}

# 无需 API key，回填 BOJ 官方无担保隔夜拆借利率。
backfill-boj-money-market:
    cargo run -p fc-worker -- backfill boj --dataset money-market

# 无需 API key，按日期范围回填 BOJ 官方无担保隔夜拆借利率。
backfill-boj-money-market-range start end:
    cargo run -p fc-worker -- backfill boj --dataset money-market --start {{start}} --end {{end}}

# 初始化本地 SQLite，并写入元数据。
bootstrap-sqlite:
    cargo run -p fc-worker -- db init
    cargo run -p fc-worker -- db seed

# 为“真实历史回测”准备长区间免费历史库。
# 默认会从 2006 年开始回填 FRED / Treasury / BOJ / World Bank，并补近年的 SEC EDGAR。
backfill-backtest-history:
    ./scripts/backfill-backtest-history.ps1

# 指定日期范围构建长区间历史库，便于跑真实历史回测。
backfill-backtest-history-range start end:
    ./scripts/backfill-backtest-history.ps1 -CoreStart {{start}} -End {{end}}

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
