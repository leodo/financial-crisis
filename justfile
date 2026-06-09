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

# 检查生成工件是否误入版本化目录。
# 已暂存的版本化 artifact 会让命令失败，除非先补充证据说明并设置 ALLOW_TRACKED_ARTIFACTS=1。
artifact-status:
    ./scripts/artifact-status.ps1

# 检查当前仓库最大的源码热点文件，并在你直接改到这些大文件时阻止继续提交。
# 如果这次修改本身就是拆分热点文件，可临时设置 `ALLOW_HOTSPOT_TOUCH=1` 后重跑，并在提交说明里写清原因。
hotspot-status:
    ./scripts/hotspot-status.ps1

# 从正在运行的本地 API 导出一份滚动审计报告，默认写到 reports/rolling-audit。
# 适合每次 refresh/backfill 后留存一份当前评估快照，便于复盘模型是否在“高压但未危机”的阶段过度频繁动作。
audit-report:
    cargo run -p fc-worker -- audit export-current

# 查看本地 SQLite 中已经登记的 model release 列表。
# 适合检查当前有哪些候选版、激活版和历史版。
release-list:
    cargo run -p fc-worker -- research release list

# 用“当前 active release”对比一个 candidate release，自动切换 API、以 strict_rebuild 方式重放历史、导出 review 报告，再恢复原 active。
# 默认导出到忽略目录 artifacts/research/release-review，避免实验副产物长期污染 Git 工作区。
# 用法：`just release-review us_formal_pit_20260531T160129`
release-review candidate_release_id:
    cargo run -p fc-worker -- research release review --candidate-release-id {{candidate_release_id}}

# 快速评审入口：不强制 strict_rebuild，只用 API 默认历史构建策略，适合 strict review 太慢时先判断候选方向。
# 这不是正式 Go/No-Go 证据，正式放行仍应使用 `just release-review`。
release-review-fast candidate_release_id:
    cargo run -p fc-worker -- research release review --candidate-release-id {{candidate_release_id}} --history-mode default --history-limit 5000

# 和 `release-review` 相同，但显式导出到版本化目录 reports/release-review。
# 只在需要把这次 review 作为仓库证据长期保留时使用。
release-review-tracked candidate_release_id:
    cargo run -p fc-worker -- research release review --candidate-release-id {{candidate_release_id}} --output-dir reports/release-review

# 用当前仓库自带的 heuristic bootstrap manifest 初始化一份 release，并激活到本地 API。
# 这不会把系统伪装成正式概率模型；它只是把 method metadata 从硬编码迁到 release registry。
release-bootstrap:
    cargo run -p fc-worker -- research release publish --manifest config/model-releases/us-heuristic-bootstrap.json --activate --reload-api

# 查看已经落库的 prediction snapshot 历史。
# 适合确认 SQLite 中是否已经生成了 release-backed 的评估轨迹。
snapshot-list:
    cargo run -p fc-worker -- research snapshot list --market-scope financial_system

# 导出 prediction snapshot 原始审计表，可选 --format csv。
# 默认输出 JSON 到终端，也可以追加 `--output-path reports/snapshots.json`。
snapshot-export:
    cargo run -p fc-worker -- research snapshot export --market-scope financial_system

# 导出用于 formal 概率研究流水线的特征+标签数据集。
# 适合先检查样本质量，再决定是否训练新的正式概率 release。
snapshot-dataset:
    cargo run -p fc-worker -- research snapshot dataset --market-scope financial_system --output-path reports/pipeline-dataset.json

# 从原始观测值生成 point-in-time feature snapshot，并写入 SQLite。
# 这是正式模型主线的第一步，不再依赖 prediction snapshot 反推特征。
feature-build:
    cargo run -p fc-worker -- research feature build --market-scope financial_system

# 基于 raw observations -> feature snapshots -> labels 构建 formal_v1 主数据集。
# 这一步会把 dataset manifest 和逐日样本一起写入 SQLite。
formal-dataset-build:
    cargo run -p fc-worker -- research dataset build-main --market-scope financial_system

# 构建 1990+ 的 protected stress / extension 数据集。
# 适合把 1990-1993、1994、2000-2001、2011 这些“高压但不等同主危机正例”的场景单独拉出来做 summary、审计和扩展训练。
formal-dataset-build-ext-stress:
    cargo run -p fc-worker -- research dataset build-main --market-scope financial_system --dataset-id formal_v1_ext_stress_1990_daily --label-version formal_label_v1_ext_stress

# 构建 1987 / 1998 急性冲击扩展数据集。
# 这套数据集允许使用更宽松的 proxy gate，不要求现代 VIX 完整覆盖，主要服务短窗研究与历史类比。
formal-dataset-build-ext-acute:
    cargo run -p fc-worker -- research dataset build-main --market-scope financial_system --dataset-id formal_v1_ext_acute_pre1990 --label-version formal_label_v1_ext_acute

# 按年度切块重建 long-history feature snapshots，再汇总生成一版 formal dataset。
# 适合第一次迁移到新的 PIT 口径，或者中途中断后继续续跑。
# 由于 feature build 现在会复用同版 snapshots，这个命令可重复执行，不会每次从头重算。
formal-rebuild-history:
    ./scripts/formal-rebuild-history.ps1

# 一键补齐 formal 主模型需要的免费历史数据，并重建一版长历史训练数据集。
# 当前会回填 FRED 主序列、DFF 作为联邦基金利率历史补丁、以及 BOJ money-market。
# 10Y-2Y 已通过 FRED 的 T10Y2Y 覆盖，因此这里不额外依赖 Treasury XML 长跑任务。
formal-history-backfill:
    cargo run -p fc-worker -- backfill fred --start 1990-01-01 --end 2026-05-31
    cargo run -p fc-worker -- backfill fred --start 1990-01-01 --end 2026-05-31 --indicator us_liquidity_effr --external-code DFF
    cargo run -p fc-worker -- backfill boj --dataset money-market --start 1990-01-01 --end 2026-05-31
    cargo run -p fc-worker -- research dataset build-main --market-scope financial_system --from 1990-01-01 --to 2026-05-31

# 查看已经写入 SQLite 的 formal dataset manifest。
formal-dataset-list:
    cargo run -p fc-worker -- research dataset list-main --market-scope financial_system

# 一键导出当前 main / ext_stress / ext_acute 三套 formal dataset summary。
# 它会自动挑选每个 dataset_id 当前最新的一版 key，并把 JSON/Markdown 写到 ignored 的 artifacts/research/dataset-summary-check。
formal-dataset-summary-pack:
    ./scripts/formal-dataset-summary-pack.ps1

# 导出某个危机场景的 formal dataset 样本切片，便于逐日看 split / label / features。
# 用法：`just formal-dataset-slice us_regional_banks_2023 2022-12-01 2023-03-15`
formal-dataset-slice scenario_id from_date to_date:
    cargo run -p fc-worker -- research dataset slice-main --market-scope financial_system --scenario-id {{scenario_id}} --from {{from_date}} --to {{to_date}}

# 用某个 release 的 bundle 直接离线打分 formal dataset 场景切片，导出 raw/calibrated/final probability 与 overlay contribution。
# 适合做“为什么这个候选版在某次危机上提前量丢了”的根因排查，避免等待 API strict_rebuild。
# 用法：`just formal-probability-slice us_formal_family_hybrid_20260603T144814 us_regional_banks_2023 2022-12-01 2023-03-15`
formal-probability-slice release_id scenario_id from_date to_date:
    cargo run -p fc-worker -- research release formal-probability-slice --market-scope financial_system --release-id {{release_id}} --scenario-id {{scenario_id}} --from {{from_date}} --to {{to_date}}

# 离线对比 baseline 与 candidate 在同一 formal dataset 场景窗口里的逐日概率、阈值命中与 base feature contribution 差异。
# 适合先锁定“哪几天掉得最厉害、哪些特征最该改”，再决定重训方向。
# 用法：`just formal-probability-compare us_formal_interaction_tail_extmix10_20260602T061401 us_formal_family_hybrid_20260603T144814 us_regional_banks_2023 2022-12-01 2023-03-15`
formal-probability-compare baseline_release_id candidate_release_id scenario_id from_date to_date:
    cargo run -p fc-worker -- research release formal-probability-compare --market-scope financial_system --baseline-release-id {{baseline_release_id}} --candidate-release-id {{candidate_release_id}} --scenario-id {{scenario_id}} --from {{from_date}} --to {{to_date}}

# 用固定三段窗口快速审计某个 family-hybrid 候选：
# 1) `regional_banks` 正窗口是否保住 20d 连续性；
# 2) `2023-02` 常态误报是否收敛；
# 3) `2023-07` 常态误报是否收敛。
# 适合在决定是否值得跑 `release-review-fast` 之前先做离线筛选。
# 用法：`just formal-candidate-window-audit us_formal_family_hybrid_20260604T034053 us_formal_family_hybrid_20260604T064930`
formal-candidate-window-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-window-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 对比两个候选在指定 horizon 的阈值、regime 概率分布和关键特征权重差异。
# 适合在 window audit 之后快速判断“到底是 threshold 变了，还是 curve / USDJPY / family context 权重变了”。
# 用法：`just formal-candidate-feature-audit us_formal_family_hybrid_20260604T034053 us_formal_family_hybrid_20260604T064930`
formal-candidate-feature-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-feature-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 进一步对齐 `curve / bond-spread / USDJPY / jpy carry / 20d threshold` 语义审计。
# 会明确输出哪些约束已经在训练层落实，哪些还只是文档约束，以及最小代码入口在哪。
# 用法：`just formal-candidate-semantics-audit us_formal_family_hybrid_20260604T034053 us_formal_family_hybrid_20260604T064930`
formal-candidate-semantics-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-semantics-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 审计 JPY carry proxy 在 1987 / 1990 / 2024 高 FX 窗口里是否主要来自 protected/pre-warning 压力，而不是普通汇率尖峰。
# 这个命令不重训模型，只从 SQLite formal dataset 行按正式 resolver 公式重算 `family_proxy__jpy_carry` 并导出 JSON。
formal-candidate-jpy-carry-audit:
    ./scripts/formal-candidate-jpy-carry-audit.ps1

# 对比 baseline / candidate 的 strict release-review 工件，专门审计 timely warning / actionable lead time。
# 会把 60d runtime separation、L2 但无 L3 的历史样本、Focus Scenarios 的 runtime block mix、
# Historical Audit workstreams/actions 统一摊开，直接回答“为什么看见了风险却没形成可执行提前量”。
# 用法：`just formal-candidate-leadtime-audit us_formal_family_hybrid_20260604T081030 us_formal_family_hybrid_20260605T202246`
formal-candidate-leadtime-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-leadtime-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 对比指定 history mode 的 release-review 工件；当 API 最新审计是 default 口径时，用这个命令生成可被页面匹配的 lead-time audit。
# 用法：`just formal-candidate-leadtime-audit-mode us_formal_family_hybrid_20260606T112926 us_formal_family_hybrid_20260609T162641 default`
formal-candidate-leadtime-audit-mode baseline_release_id candidate_release_id history_mode:
    ./scripts/formal-candidate-leadtime-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}} -HistoryMode {{history_mode}}

# 用固定美国历史场景包一口气审计 baseline / candidate：
# 直接把 1987、1990s、2000、2008、2011、2020、2022、2023 的 compare、coverage 和 release-review blocker
# 收到同一份 JSON 里，优先回答“免费数据能不能覆盖、该用哪个 dataset、主要卡在 gate 还是 continuity”。
# 用法：`just formal-candidate-scenario-pack-audit us_formal_family_hybrid_20260606T112926 us_formal_family_hybrid_20260608T173701`
formal-candidate-scenario-pack-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-scenario-pack-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 固定审计 `1987 / 1998 / 2000-2001 / 2011` 的 prewarning_signal_gap：
# 自动跑每个场景的 formal-probability-compare 与 formal dataset slice，合并输出样本覆盖、标签、
# 动作 episode、阈值命中、近阈值行数和下一步诊断，直接回答“为什么没有形成提前预警”。
# 用法：`just formal-candidate-prewarning-gap-audit us_formal_family_hybrid_20260606T112926 us_formal_family_hybrid_20260608T173701`
formal-candidate-prewarning-gap-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-prewarning-gap-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 固定审计 `2011 美欧融资压力 / no_runtime_floor_signal`：
# 在 prewarning-gap 总览之后继续下钻 funding stress 的 split、标签、20d/60d floor 距离、
# mixed-systemic family context 与关键 feature separation，判断应先修训练拓扑/特征还是阈值。
# 用法：`just formal-candidate-funding-stress-audit us_formal_family_hybrid_20260606T112926 us_formal_family_hybrid_20260608T173701`
formal-candidate-funding-stress-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-funding-stress-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 对 `prewarning_signal_gap / weak_signal_continuity` 这类 residual workstream 直接拉 formal dataset slice，
# 汇总样本覆盖、split、标签、episode 和 feature 覆盖，避免只知道“哪条线有问题”却不知道“数据证据长什么样”。
# 用法：`just formal-candidate-workstream-audit us_formal_family_hybrid_20260605T202246 us_formal_family_hybrid_20260606T112926`
formal-candidate-workstream-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-workstream-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 固定审计 `2022 联储加息与久期冲击`：
# 把 formal dataset slice 与 baseline/candidate 概率 compare 拼起来，直接看 primary / late_validation /
# prepare / hedge 各自的 hit rate、最长连续命中、阈值距离和特征分离。
# 用法：`just formal-candidate-rate-shock-audit us_formal_family_hybrid_20260606T112926 us_formal_family_hybrid_20260608T173701`
formal-candidate-rate-shock-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-rate-shock-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 固定审计候选版的 cooldown bleed 与 false-positive 回归：
# 读取或生成 default release review，导出结构化 JSON，直接列出 precision、最长误报、runtime floor、
# 20d/60d cooldown regime、候选新增/拉长误报 episode 与 no-go 原因。
# 用法：`just formal-candidate-cooldown-audit us_formal_family_hybrid_20260606T112926 us_formal_family_hybrid_20260608T191024`
formal-candidate-cooldown-audit baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-cooldown-audit.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 标准候选筛选入口：先跑三段窗口 compare，再读取或生成 default release review，
# 把 20d cooldown bleed、动作精度、最长误报区间和 runtime floor hit 纳入 no-go 判断；
# 然后跑 20d 特征/阈值审计、语义审计，最后补一轮美国历史场景包审计。
# 适合 family-hybrid 主线的新候选第一轮筛查；只有这一步结论足够好，才继续跑 `release-review-fast`。
# 用法：`just formal-candidate-screen us_formal_family_hybrid_20260604T034053 us_formal_family_hybrid_20260604T064930`
formal-candidate-screen baseline_release_id candidate_release_id:
    ./scripts/formal-candidate-screen.ps1 -BaselineReleaseId {{baseline_release_id}} -CandidateReleaseId {{candidate_release_id}}

# 默认基于 SQLite 中最新的 persisted formal dataset 训练正式 bundle，并写出 bundle / manifest / evaluation 三份文件。
# 默认输出到忽略目录 artifacts/research/model-*/generated；如需回退旧的 prediction snapshot 过渡链路，可手动追加 `--dataset-source snapshot`。
formal-train:
    cargo run -p fc-worker -- research pipeline train-probability --market-scope financial_system

# 只加载 formal train/calibration/evaluation 数据并打印 split / topology repair 计数，不训练、不写 bundle。
# 适合先验证样本是否真的进入训练拓扑。
formal-train-dry-run:
    cargo run -p fc-worker -- research pipeline train-probability --market-scope financial_system --dry-run

# 和 `formal-train` 相同，但显式把 bundle / manifest 输出到版本化 generated 目录。
# 只在需要把该候选版作为仓库长期证据保留时使用。
formal-train-tracked:
    cargo run -p fc-worker -- research pipeline train-probability --market-scope financial_system --output-dir config/model-bundles/generated --manifest-dir config/model-releases/generated

# 使用新的 `interaction_tail_v1` 非线性交互/尾部特征形态训练候选 bundle。
# 适合在 formal main + 扩展数据集组合上验证“表达力增强后，runtime 提前量是否恢复”。
formal-train-interaction-tail:
    cargo run -p fc-worker -- research pipeline train-probability --market-scope financial_system --model-shape interaction_tail_v1

# 训练 `interaction_tail_v1`，并把产物显式输出到版本化 generated 目录。
formal-train-interaction-tail-tracked:
    cargo run -p fc-worker -- research pipeline train-probability --market-scope financial_system --model-shape interaction_tail_v1 --output-dir config/model-bundles/generated --manifest-dir config/model-releases/generated

# 自动解析最新的 formal main + ext_stress + ext_acute dataset key，
# 训练 `family_conditional_v1` overlay 候选，避免每次手工拼接长参数。
formal-train-family-overlay:
    ./scripts/formal-train-family-overlay.ps1

# 自动解析最新 main + ext_stress + ext_acute key，只做训练拓扑 dry-run 审计。
# 用来确认 2011 funding-stress 等 protected extension row 是否已进入 train_topology_repair。
formal-train-family-overlay-dry-run:
    ./scripts/formal-train-family-overlay.ps1 -DryRun

# 和 `formal-train-family-overlay` 相同，但把 bundle / manifest 显式写到版本化 generated 目录。
formal-train-family-overlay-tracked:
    ./scripts/formal-train-family-overlay.ps1 -Tracked

# 使用和 family overlay 相同的数据集拼装入口，但把 `60d` 基座退回 `interaction_tail_v1`，
# 只在 `5d/20d` 与 overlay 侧保留 family conditional 形态。
formal-train-family-hybrid:
    ./scripts/formal-train-family-overlay.ps1 -ModelShape family_hybrid_v1

# 使用 family-hybrid 模型形态做训练拓扑 dry-run，不训练、不写 bundle。
formal-train-family-hybrid-dry-run:
    ./scripts/formal-train-family-overlay.ps1 -DryRun -ModelShape family_hybrid_v1

# 和 `formal-train-family-hybrid` 相同，但把 bundle / manifest 显式写到版本化 generated 目录。
formal-train-family-hybrid-tracked:
    ./scripts/formal-train-family-overlay.ps1 -Tracked -ModelShape family_hybrid_v1

# 一键训练、发布并激活 formal bundle，然后触发 API reload。
# 默认走最新 formal dataset；只有显式传 `--dataset-source snapshot` 时才回退旧链路。
# 这会把线上 probability_mode 从 heuristic 切到 formal_bundle_v1（若 bundle 可正常加载）。
formal-bootstrap:
    cargo run -p fc-worker -- research pipeline bootstrap-formal-release --market-scope financial_system

# 训练、发布并激活 `interaction_tail_v1` 候选 release。
# 适合在本地 API 上直接切换到下一代模型做 strict rebuild review 或面板联调。
formal-bootstrap-interaction-tail:
    cargo run -p fc-worker -- research pipeline bootstrap-formal-release --market-scope financial_system --model-shape interaction_tail_v1

# 停止 `just dev` 启动的 API 和 Web 服务。
stop:
    ./scripts/dev-stop.ps1

# 格式化全部 Rust 代码。
fmt:
    cargo fmt --all

# 检查 Rust 格式是否已满足仓库约束，不改文件。
fmt-check:
    cargo fmt --all -- --check

# 运行 Rust workspace 的全部测试。
test:
    cargo test --workspace

# 运行 Rust clippy，按 warning 即失败处理。
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# 本地质量门禁：检查版本化工件目录是否混入未审计副产物。
verify-artifacts:
    ./scripts/artifact-status.ps1

# 本地治理门禁：如果直接改到了当前仓库前几位的大源码文件，要求先拆模块或显式说明原因。
verify-hotspots:
    ./scripts/hotspot-status.ps1

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
# 默认跳过 World Bank 年频慢变量，保证日常刷新能在几分钟内完成。
# 这是日常维护本地评估库的首选入口；需要慢变量时再单独运行 `just backfill-world-bank`。
refresh-latest:
    cargo run -p fc-worker -- refresh latest-free --skip-world-bank
    ./scripts/dev-status.ps1

# 在日常刷新基础上追加 GDELT prototype 事件源。
# 同样默认跳过 World Bank，避免把“看当下”的刷新命令拖成慢任务。
refresh-latest-full:
    cargo run -p fc-worker -- refresh latest-free --skip-world-bank --include-gdelt
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

# 常用检查：格式检查、Rust 测试、前端构建。
check-all: fmt-check test web-build

# 完整本地门禁：版本化工件审计、Rust 格式检查、测试、clippy、前端构建。
verify: verify-artifacts verify-hotspots fmt-check test lint web-build
