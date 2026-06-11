export const auditContent = {
  guideRows: [
    ["版本登记册", "看线上当前登记了哪些候选版、正式版和历史版；登记数量不等于可上线数量。"],
    ["运行模式", "看 API 当前实际加载的是启发式过渡层还是正式概率包；它不是当前主结论可信度。"],
    ["Release Review", "看最近一次 baseline vs candidate 评审是否通过 guard，以及问题到底属于候选退化还是主线共性短板。"],
    ["历史场景包", "看固定美国历史场景里，哪些已经稳定通过，哪些仍是主线共享漏报或执行连续性问题。"],
    ["Dataset Evidence", "看 main / ext_stress / ext_acute 三套 formal dataset 到底有没有真实样本、哪些场景能训练、哪些只是类比。"],
    ["Residual Workstream", "看当前 residual workstream 到底有没有训练样本、落在哪些 dataset、标签和 regime 长什么样。"],
    ["提前预警缺口", "看 1987 / 1998 / 2000 / 2011 这些历史样本到底是真缺运行信号，还是已有信号但候选边际变弱。"],
    ["2011 Funding Stress", "单独看 2011 美欧融资压力为什么有数据、有 protected rows，却没有形成可执行 runtime floor。"],
    ["Lead-Time 转化", "看 runtime 已有信号时，为什么仍没有转成更早的 L3 可执行预警。"],
    ["Runtime Contribution", "看当前面板概率和入线占比为什么偏冷；这是模型诊断，不是当前风险距离。"],
    ["Overlay 审计", "看当前 active release 是只有 family 审计元数据，还是已经有 overlay head 真正参与 runtime。"],
    ["运行快照 / 旧桥接视图", "看每天落库的概率截面是否和当前生效版本对得上，并核对旧 snapshot bridge 是否还有残留。"],
    ["降级识别", "如果版本登记已经是正式版，但运行态退回启发式层，通常说明 bundle 加载或服务检查失败。"]
  ] as Array<[string, string]>,
  noteSummary:
    "这页主要用来核对当前线上版本、release review、历史场景包审计、版本登记历史，以及运行快照 / 旧桥接视图是否一致；如果运行中的概率层和登记状态对不上，通常表示系统已经自动降级。",
  summaryNote:
    "先看版本总量、当前/已批准、回放批次、快照覆盖和当前默认历史的证据层，再进入下面的 release review 与版本明细；这些都是审计口径，不是自动上线放行结论。",
  provenanceNote:
    "这块回答的是当前线上默认历史轨迹到底是 PIT feature-backed 正式证据，还是 raw observation 过渡口径，或仍残留旧 snapshot bridge。",
  releaseReviewSummary:
    "这里展示最近一次已落库的 release review。先看 guard 是否通过，再看 historical audit action 判断问题属于候选退化、主线共性短板，还是只需继续人工复核。",
  releaseReviewCoverageSummary:
    "这块回答的是：本次 review 涉及的历史场景到底能不能作为正式主训练、扩展训练、protected stress 或历史类比。不要把所有危机都当成同一类正例。",
  scenarioPackSummary:
    "这块回答的是：固定美国历史场景包里，哪些场景已经稳定覆盖，哪些是主线共享漏报，哪些主要卡在执行连续性。它是 release review 之外的横向历史复盘面板。",
  datasetSummary:
    "这块回答的是：main / ext_stress / ext_acute 三套 formal dataset 现在到底装了哪些历史样本。这里的行数、覆盖场景和可训练标签都是 dataset evidence，不是当前模型样本量承诺，也不是上线批准。",
  workstreamSummary:
    "这块回答的是：remaining residual workstream 到底有没有真实训练样本与 feature 证据。它不判断候选是否该上线，而是避免团队继续把问题归因停留在“可能没数据”。",
  rateShockSummary:
    "这块把 2022 联储加息与久期冲击单独拉出来，直接看 primary / late validation 与 prepare / hedge 这两组连续性证据。重点不是看候选是否更高，而是看真正该提前升温的窗口有没有形成连续命中。",
  prewarningGapSummary:
    "这块把 1987 / 1998 / 2000-2001 / 2011 的提前预警缺口拆开看。重点是区分“真的没有运行阈值信号”、“已有 protected context 但尚未进入主线”，以及“候选相对基线边际变弱”。",
  fundingStressSummary:
    "这块专门回答 2011 美欧融资压力为什么没有形成 runtime floor。重点看它是不是仍被 evaluation-only 挡在训练外、mixed-systemic proxy 是否真正参与 runtime、哪些 base 特征在压低 20d/60d，以及与离线运行线的差距；这里不是当前风险距离。",
  leadtimeSummary:
    "这块回答的是：候选在历史回放中已经有 runtime floor hit 或 60d 提前分离时，为什么仍没有变成更早的 L3 可执行预警。这里的 precision、hit 和 lead time 都是离线历史口径，不是当前概率准确率。",
  cooldownSummary:
    "这块回答的是：候选版有没有因为 20d cooldown bleed 或历史纯误报变长而不适合继续晋升。它把 release review 里的动作精度、最长误报、runtime floor 和误报 episode 变化收口到一处；这些都是离线晋升阻断证据。",
  runtimeContributionSummary:
    "这块回答的是：当前 runtime 概率和“入线占比”为什么这么冷。它直接比较 baseline / candidate 在最近运行窗口里的 base contribution、语义异常、阈值和 runtime group；入线占比只能当模型审计证据，不能解释成风险很远或新的 Go/No-Go 放行结论。",
  releaseReviewEmpty:
    "还没有找到可用的 release review 落库结果。通常说明这条链路尚未执行，或者当前 market scope 还没有写入 artifacts/research/release-review。",
  scenarioPackEmpty:
    "还没有找到与最近一次 release review 对应的 scenario-pack audit 工件。通常说明这条离线审计还没跑，或者当前 review 对应的历史场景包结果尚未落库。",
  datasetSummaryEmpty:
    "还没有找到 formal dataset summary 工件。通常说明 main / ext_stress / ext_acute 的 summary 还没导出，或者当前 SQLite 里还没有可用的 dataset evidence JSON。",
  workstreamEmpty:
    "还没有找到与最近一次 release review 对应的 residual workstream 审计工件。通常说明这条 dataset evidence 审计还没跑，或者 baseline / candidate 对应的 JSON 结果尚未落库。",
  rateShockEmpty:
    "还没有找到与最近一次 release review 对应的 2022 rate-shock 专项审计工件。通常说明这条离线 continuity 审计还没跑，或者 baseline / candidate 对应的 JSON 结果尚未落库。",
  prewarningGapEmpty:
    "还没有找到与最近一次 release review 对应的 pre-warning gap 审计工件。通常说明还没运行 `just formal-candidate-prewarning-gap-audit <baseline> <candidate>`，或者 JSON 与当前 review 的 baseline / candidate 不一致。",
  fundingStressEmpty:
    "还没有找到与最近一次 release review 对应的 2011 funding-stress 专项审计工件。通常说明还没运行 `just formal-candidate-funding-stress-audit <baseline> <candidate>`，或者 JSON 与当前 review 的 baseline / candidate 不一致。",
  leadtimeEmpty:
    "还没有找到与最近一次 release review 对应的 lead-time 转化链审计工件。通常说明还没运行 `just formal-candidate-leadtime-audit <baseline> <candidate>`，或者 JSON 与当前 review 的 baseline / candidate / history mode 不一致。",
  cooldownEmpty:
    "还没有找到与最近一次 release review 对应的 cooldown / false-positive 审计工件。通常说明还没运行 `just formal-candidate-cooldown-audit <baseline> <candidate>`，或者 JSON 与当前 review 的 baseline / candidate / history mode 不一致。",
  runtimeContributionEmpty:
    "还没有找到与最近一次 release review 对应的 runtime contribution 审计工件。通常说明还没运行 `just formal-candidate-runtime-contribution-audit <baseline> <candidate>`，或者 JSON 与当前 review 的 baseline / candidate / history mode 不一致。",
  releaseReviewCoverageTableNote:
    "先看目录结论和可用范围，再看 grade / PIT / 免费主源，最后看主要缺口。重点复核场景和 protected window 会直接影响后续训练与 posture 规则约束。",
  scenarioPackTableNote:
    "先看当前判读，再看真实结果与 lead time，最后结合覆盖目录、dataset evidence 和 takeaway 判断这是候选退化、主线共享短板，还是稳定覆盖。",
  datasetSummaryTableNote:
    "先看三套 dataset 的真实时间范围、split 行数和正标签，再看 catalog intent 与 recommendation。这里回答的是“这套历史数据能拿来干什么”，不是候选版是否已经通过 release review，也不是当前模型训练样本数承诺。",
  datasetScenarioTableNote:
    "逐场景看 main / ext_stress / ext_acute 里到底有没有样本、用什么角色、主要缺口是什么。若这里已经有稳定样本行，后续问题通常不再是“没数据”，而是训练目标、特征或 gate 设计本身。",
  workstreamTableNote:
    "先看它落到哪个 workstream，再看 dataset / split / regime / 标签分布。若这里已经有足够样本与正标签，问题通常不再是“没数据”，而是训练拓扑、feature separation 或 gate 设计本身。",
  rateShockPhaseTableNote:
    "按阶段看 20d / 60d 均值、命中数和最长连续段。primary 依然过冷，通常说明问题还在 trainability 或标签连续性，而不是单纯阈值。",
  rateShockActionTableNote:
    "按动作层看 prepare / hedge / defend 的连续性。先看 20d 段数和最长段，再看离阈值 5pp 内的行数，判断它是不是已经接近可执行窗口。",
  prewarningGapTableNote:
    "先看诊断分类，再看历史 dataset 行数、标签和候选 20d/60d 回放命中。命中数只说明历史回放有没有越过离线线，不代表当前正式概率可用；若 2011 这类完全没有 runtime floor，才优先查 feature separation / family context。",
  fundingStressFeatureTableNote:
    "先看 positive window 相对 normal 的 standardized gap，再看它是否正好落在 funding、credit、curve、VIX 或 USDJPY 这些可解释维度。若 proxy 已存在但行仍是 evaluation split，下一步通常是训练拓扑和 family/context 迁移，而不是直接降阈值。",
  fundingStressContributionTableNote:
    "这是候选模型在 2011 窗口里的绝对 base contribution，不是候选相对基线的差分。先看哪些特征在压低 20d，再决定是修 mixed-systemic proxy、训练拓扑，还是候选权重。",
  fundingStressOverlayTableNote:
    "Overlay 行只说明 family gate 是否实际参与了候选打分。若 gate value 或 blend 很低，说明 family context 还没有真正进入 runtime 信号。",
  leadtimeRuntimeTableNote:
    "先看候选是否已经有 early-warning separation，再看它离 runtime floor 和 strict review gate 还有多远。若 separation 已有但 timely warning 不升，问题通常在 posture continuity、p20d gate 或 sustained-hit 转化。",
  cooldownRuntimeTableNote:
    "先看候选诊断是否出现 cooldown bleed，再看 cooldown - positive 与 cooldown - normal。这里比较的是历史窗口均值差，不是当前概率差；若 cooldown 不低于 positive window，说明模型把危机后余震和前瞻窗口混在一起。",
  cooldownEpisodeTableNote:
    "候选新增或拉长的纯误报 episode 是离线 release 晋升的核心阻断项。优先检查持续天数和是否能被 protected stress window 解释；它不是今天新增误报数。",
  cooldownScenarioTableNote:
    "按历史场景看纯误报数量变化。这里不是看命中率，而是看候选有没有为了局部连续性牺牲误报治理。",
  runtimeContributionHorizonTableNote:
    "先看三期限的候选均值是否接近运行线，再看 semantic anomalies。若 USDJPY 高位 tail 仍是负贡献，入线占比只能当模型审计证据，不能解释成风险很远。",
  runtimeContributionGroupTableNote:
    "Runtime group 把候选输出按时距、posture 和阈值状态分组。若 building / cold 之间差异很大，说明候选已经能区分部分状态，但阈值或特征方向仍可能有问题。",
  runtimeContributionLatestDateTableNote:
    "这张表只看审计窗口最后一天，方便对照当前面板读数；它不是实时风险距离，正式决策仍以当前 active release、MVP 风险状态和人工复核为准。",
  runtimeContributionAnomalyTableNote:
    "语义异常说明特征方向与金融解释冲突，例如 USDJPY 高位或上行反而压低危机概率。出现这类行时，应修训练约束和 release review，而不是在运行时硬抬概率。",
  releaseReviewActionTableNote:
    "先看 action type，再看它落在哪个 workstream，最后结合 recommendation 判断是该判退候选、先修 blocker，还是继续补主线研究。",
  releaseReviewAttributionTableNote:
    "归因表回答的是问题到底属于谁。baseline / candidate 计数越不对称，越可能是候选版新增退化；两边都命中则更像主线共性短板。",
  overlaySummary:
    "这里只看当前 active release 的 runtime overlay 证据，不替代离线 release review。先看当前有没有真正挂载 overlay，再看每个 horizon 的 final 概率是否被改写。",
  overlayEmpty:
    "当前 active release 没有输出 family overlay 诊断；这通常说明还在旧 bundle，或者该版本没有挂载 overlay head。",
  overlayTableNote:
    "先看场景数和三段 split 行数，再看 gate-active 行数，最后结合 note 判断这个 family 是真正可训，还是只有候选审计。",
  unsupportedPrefix: "当前数据存储模式为",
  unsupportedSuffix: "，暂时没有可展示的本地版本 / 快照审计数据。",
  releaseTableNote: "小屏幕下这张表支持横向滚动，优先看版本、登记状态和训练区间；概率误差/损失/校准误差是离线评估指标，不是当前页面主结论。",
  snapshotTableNote:
    "这张表是运行快照审计，不是当前实时决策结论；当前实时结论请以“决策面板”的“离风险还有多远”为准。先看它是当前线上快照还是历史/候选快照，再看 5d/20d/60d 的百分比、接口小数和 bp。"
} as const;
