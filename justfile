set shell := ["powershell.exe", "-NoLogo", "-Command"]

fmt:
    cargo fmt --all

test:
    cargo test --workspace

lint:
    cargo clippy --workspace --all-targets -- -D warnings

api:
    cargo run -p fc-api

worker:
    cargo run -p fc-worker

web-install:
    cd apps/web; npm install

web-dev:
    cd apps/web; npm run dev

web-build:
    cd apps/web; npm run build

check-all: fmt test web-build

