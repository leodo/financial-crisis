export const auditContent = {
  guideRows: [
    ["版本登记册", "看线上当前登记了哪些候选版、正式版和历史版。"],
    ["运行模式", "看 API 当前实际正在使用启发式过渡层，还是正式概率包。"],
    ["Release Review", "看最近一次 baseline vs candidate 评审是否通过 guard，以及问题到底属于候选退化还是主线共性短板。"],
    ["历史场景包", "看固定美国历史场景里，哪些已经稳定通过，哪些仍是主线共享漏报或执行连续性问题。"],
    ["Dataset Evidence", "看 main / ext_stress / ext_acute 三套 formal dataset 到底有没有真实样本、哪些场景能训练、哪些只是类比。"],
    ["Residual Workstream", "看当前 residual workstream 到底有没有训练样本、落在哪些 dataset、标签和 regime 长什么样。"],
    ["Overlay 审计", "看当前 active release 是只有 family 审计元数据，还是已经有 overlay head 真正参与 runtime。"],
    ["运行快照 / 旧桥接视图", "看每天落库的概率截面是否和当前生效版本对得上，并核对旧 snapshot bridge 是否还有残留。"],
    ["降级识别", "如果版本登记已经是正式版，但运行态退回启发式层，通常说明 bundle 加载或服务检查失败。"]
  ] as Array<[string, string]>,
  noteSummary:
    "这页主要用来核对当前线上版本、release review、历史场景包审计、版本登记历史，以及运行快照 / 旧桥接视图是否一致；如果运行中的概率层和登记状态对不上，通常表示系统已经自动降级。",
  summaryNote:
    "先看版本总量、当前/已批准、回放批次、快照覆盖和当前默认历史的证据层，再进入下面的 release review 与版本明细。",
  provenanceNote:
    "这块回答的是当前线上默认历史轨迹到底是 PIT feature-backed 正式证据，还是 raw observation 过渡口径，或仍残留旧 snapshot bridge。",
  releaseReviewSummary:
    "这里展示最近一次已落库的 release review。先看 guard 是否通过，再看 historical audit action 判断问题属于候选退化、主线共性短板，还是只需继续人工复核。",
  releaseReviewCoverageSummary:
    "这块回答的是：本次 review 涉及的历史场景到底能不能作为正式主训练、扩展训练、protected stress 或历史类比。不要把所有危机都当成同一类正例。",
  scenarioPackSummary:
    "这块回答的是：固定美国历史场景包里，哪些场景已经稳定覆盖，哪些是主线共享漏报，哪些主要卡在执行连续性。它是 release review 之外的横向历史复盘面板。",
  datasetSummary:
    "这块回答的是：main / ext_stress / ext_acute 三套 formal dataset 现在到底装了哪些历史样本。它把“目录说可覆盖”和“SQLite 里真的有多少行样本”这两件事合在一起看，避免继续靠口头判断数据是否已经够训练。",
  workstreamSummary:
    "这块回答的是：remaining residual workstream 到底有没有真实训练样本与 feature 证据。它不判断候选是否该上线，而是避免团队继续把问题归因停留在“可能没数据”。",
  rateShockSummary:
    "这块把 2022 联储加息与久期冲击单独拉出来，直接看 primary / late validation 与 prepare / hedge 这两组连续性证据。重点不是看候选是否更高，而是看真正该提前升温的窗口有没有形成连续命中。",
  cooldownSummary:
    "这块回答的是：候选版有没有因为 20d cooldown bleed 或纯误报变长而不适合继续晋升。它把 release review 里的动作精度、最长误报、runtime floor 和误报 episode 变化收口到一处。",
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
  cooldownEmpty:
    "还没有找到与最近一次 release review 对应的 cooldown / false-positive 审计工件。通常说明还没运行 `just formal-candidate-cooldown-audit <baseline> <candidate>`，或者 JSON 与当前 review 的 baseline / candidate / history mode 不一致。",
  releaseReviewCoverageTableNote:
    "先看目录结论和可用范围，再看 grade / PIT / 免费主源，最后看主要缺口。重点复核场景和 protected window 会直接影响后续训练与 posture 规则约束。",
  scenarioPackTableNote:
    "先看当前判读，再看真实结果与 lead time，最后结合 coverage / dataset 和 takeaway 判断这是候选退化、主线共享短板，还是稳定覆盖。",
  datasetSummaryTableNote:
    "先看三套 dataset 的真实时间范围、split 行数和正标签，再看 catalog intent 与 recommendation。这里回答的是“这套数据能拿来干什么”，不是候选版是否已经通过 release review。",
  datasetScenarioTableNote:
    "逐场景看 main / ext_stress / ext_acute 里到底有没有样本、用什么角色、主要缺口是什么。若这里已经有稳定样本行，后续问题通常不再是“没数据”，而是训练目标、特征或 gate 设计本身。",
  workstreamTableNote:
    "先看它落到哪个 workstream，再看 dataset / split / regime / 标签分布。若这里已经有足够样本与正标签，问题通常不再是“没数据”，而是训练拓扑、feature separation 或 gate 设计本身。",
  rateShockPhaseTableNote:
    "按阶段看 20d / 60d 均值、命中数和最长连续段。primary 依然过冷，通常说明问题还在 trainability 或标签连续性，而不是单纯阈值。",
  rateShockActionTableNote:
    "按动作层看 prepare / hedge / defend 的连续性。先看 20d 段数和最长段，再看离阈值 5pp 内的行数，判断它是不是已经接近可执行窗口。",
  cooldownRuntimeTableNote:
    "先看候选诊断是否出现 cooldown bleed，再看 cooldown - positive 与 cooldown - normal。若 cooldown 不低于 positive window，说明模型把危机后余震和前瞻窗口混在一起。",
  cooldownEpisodeTableNote:
    "候选新增或拉长的纯误报 episode 是 release 晋升的核心阻断项。优先检查持续天数和是否能被 protected stress window 解释。",
  cooldownScenarioTableNote:
    "按历史场景看纯误报数量变化。这里不是看命中率，而是看候选有没有为了局部连续性牺牲误报治理。",
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
  releaseTableNote: "小屏幕下这张表支持横向滚动，优先看版本、登记状态和训练区间。",
  snapshotTableNote:
    "这张表不是 formal history 主证据链。先看日期、版本和 5d/20d/60d，再核对它和当前 active release 是否一致，以及旧 snapshot bridge 是否还有残留。"
} as const;
