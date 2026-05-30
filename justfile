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
