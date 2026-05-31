import ReactECharts from "./charts";
import {
  Activity,
  BadgeInfo,
  Database,
  History,
  Layers3,
  Radar,
  ShieldCheck,
  Siren,
  Table2
} from "lucide-react";
import type {
  AlertEvent,
  AssessmentMethodResponse,
  AssessmentSnapshot,
  BacktestScenarioSummary,
  BacktestWindowPoint,
  DataSource,
  IndicatorRisk,
  PostureGuidance,
  ResearchAuditResponse,
  RiskSnapshot
} from "./types";
import {
  auditEpisodeClass,
  auditEpisodeLabel,
  backtestSignalSourceLabel,
  eventStateLabel,
  formatDate,
  formatDateTime,
  formatNumber,
  formatPercent,
  postureLabel,
  qualityLabel
} from "./format";

export function DriversView({
  assessment,
  overview,
  posture
}: {
  assessment: AssessmentSnapshot;
  overview: RiskSnapshot;
  posture: PostureGuidance;
}) {
  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>上行风险驱动</h2>
            <Siren size={18} />
          </div>
          <DriverList rows={assessment.top_risk_drivers} />
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>缓冲因素</h2>
            <ShieldCheck size={18} />
          </div>
          <DriverList rows={assessment.top_relief_drivers} reverse />
        </section>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>维度解释</h2>
          <Layers3 size={18} />
        </div>
        <div className="dimension-detail-grid">
          {overview.dimensions.map((dimension) => (
            <div className="dimension-detail" key={dimension.dimension}>
              <div className="dimension-row-head">
                <strong>{dimension.label}</strong>
                <b>{formatNumber(dimension.score)}</b>
              </div>
              <span className="dimension-caption">
                这个维度当前最敏感的是 {dimension.top_contributors[0]?.display_name ?? "—"}。
              </span>
              <div className="list-stack compact">
                {dimension.top_contributors.map((item) => (
                  <div className="list-row compact" key={`${dimension.dimension}-${item.indicator_id}`}>
                    <div>
                      <strong>{item.display_name}</strong>
                      <span>{item.explanation}</span>
                    </div>
                    <b>{formatNumber(item.score)}</b>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>当前结论</h2>
          <BadgeInfo size={18} />
        </div>
        <div className="rule-box">
          <strong>系统摘要</strong>
          <span>{assessment.summary}</span>
        </div>
        <div className="rule-box">
          <strong>旧风险引擎解释</strong>
          <span>{overview.level_reason}</span>
        </div>
        <div className="rule-box">
          <strong>Posture 结论</strong>
          <span>{posture.summary}</span>
        </div>
      </section>
    </section>
  );
}

export function IndicatorsView({ indicators }: { indicators: IndicatorRisk[] }) {
  const rows = [...indicators].sort((left, right) => right.score - left.score);

  return (
    <section className="workspace">
      <section className="surface">
        <div className="surface-head">
          <h2>指标细项</h2>
          <Table2 size={18} />
        </div>
        <div className="table-note">
          指标风险分来自各指标自己的评分口径和历史分位。部分指标使用同比、变化幅度或振幅，不再直接拿原始水平值硬算高风险。
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>指标</th>
                <th>维度</th>
                <th>最新值</th>
                <th>评分口径</th>
                <th>风险分</th>
                <th>历史分位</th>
                <th>30d 变化</th>
                <th>质量</th>
                <th>来源</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((risk) => (
                <tr key={risk.indicator.indicator_id}>
                  <td>
                    <strong>{risk.indicator.display_name}</strong>
                    <span>{risk.indicator.indicator_id}</span>
                  </td>
                  <td>{risk.indicator.dimension}</td>
                  <td>
                    {formatNumber(risk.latest_observation?.value)} {risk.indicator.unit}
                  </td>
                  <td>
                    <strong>{risk.score_basis}</strong>
                    <span>
                      {formatNumber(risk.score_input_value)} {risk.score_input_unit ?? ""}
                    </span>
                  </td>
                  <td>{formatNumber(risk.score)}</td>
                  <td>{formatNumber(risk.percentile, "%")}</td>
                  <td>{formatNumber(risk.change_30d)}</td>
                  <td>{qualityLabel(risk.quality_grade)}</td>
                  <td>{risk.indicator.default_source_id}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}

export function SourcesView({
  assessment,
  sources
}: {
  assessment: AssessmentSnapshot;
  sources: DataSource[];
}) {
  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>数据可信度</h2>
            <Database size={18} />
          </div>
          <div className="mini-metrics">
            <Metric label="总覆盖" value={formatPercent(assessment.data_trust.coverage_score)} />
            <Metric
              label="核心特征"
              value={formatPercent(assessment.data_trust.core_feature_coverage)}
            />
            <Metric
              label="触发特征"
              value={formatPercent(assessment.data_trust.trigger_feature_coverage)}
            />
            <Metric
              label="外部特征"
              value={formatPercent(assessment.data_trust.external_feature_coverage)}
            />
          </div>
          <div className="list-stack">
            {assessment.data_trust.warnings.map((warning, index) => (
              <div className="bullet-row" key={`${warning}-${index}`}>
                <span className="bullet-dot" />
                <span>{warning}</span>
              </div>
            ))}
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>免费数据源策略</h2>
            <ShieldCheck size={18} />
          </div>
          <GuideList
            rows={[
              ["FRED Graph CSV", "默认无 key 路径，适合宏观和部分市场序列。"],
              ["U.S. Treasury", "官方收益率曲线兜底，不依赖第三方包装。"],
              ["World Bank", "年频慢变量补充结构脆弱性。"],
              ["BOJ + JPY carry", "BOJ 官方 USDJPY 和日本隔夜拆借利率已接入，用于免费跟踪套息融资环境。"],
              ["SEC EDGAR", "已接入官方 submissions JSON，并聚合为银行公告事件特征与告警。"],
              ["GDELT", "已支持可选回填和运行时展示，但默认仍按 prototype 辅助信号处理。"]
            ]}
          />
        </section>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>源状态</h2>
          <Database size={18} />
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>源</th>
                <th>类型</th>
                <th>状态</th>
                <th>质量</th>
                <th>是否可用于生产</th>
                <th>说明</th>
              </tr>
            </thead>
            <tbody>
              {sources.map((source) => (
                <tr key={source.source_id}>
                  <td>
                    <strong>{source.display_name}</strong>
                    <span>{source.source_id}</span>
                  </td>
                  <td>{source.source_type}</td>
                  <td>{source.health.status}</td>
                  <td>{formatNumber(source.health.quality_score)}</td>
                  <td>{source.production_allowed ? "是" : "否"}</td>
                  <td>
                    <strong>{source.health.message}</strong>
                    <span>{source.license_note}</span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}

export function MethodView({
  assessment,
  posture,
  method
}: {
  assessment: AssessmentSnapshot;
  posture: PostureGuidance;
  method: AssessmentMethodResponse;
}) {
  const heuristicMode = assessment.method.probability_mode === "heuristic_mvp";
  const degradedRelease = assessment.method.release_status === "degraded";

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>方法分层</h2>
            <BadgeInfo size={18} />
          </div>
          <GuideList
            rows={[
              ["风险强度", "0-100 分只表示指标组合位于历史压力区的什么位置，不等于危机发生概率。"],
              ["危机概率", "告诉你未来 5d / 20d / 60d 进入风险窗口的可能性。"],
              ["Time bucket", "告诉你更像是数月、数周还是当下风险。"],
              ["Posture", "把概率和可信度转换成可执行的风险处理节奏。"]
            ]}
          />
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>当前版本</h2>
            <History size={18} />
          </div>
          <div className="version-list">
            <VersionRow label="score" value={assessment.method.score_method_version} />
            <VersionRow label="prob" value={assessment.method.prob_model_version} />
            <VersionRow label="calibration" value={assessment.method.calibration_version} />
            <VersionRow label="feature" value={assessment.method.feature_set_version} />
            <VersionRow label="label" value={assessment.method.label_version} />
            <VersionRow label="posture" value={assessment.method.posture_policy_version} />
            <VersionRow label="playbook" value={assessment.method.action_playbook_version} />
            <VersionRow label="prob mode" value={assessment.method.probability_mode} />
            <VersionRow label="release" value={assessment.method.release_status} />
            <VersionRow label="release id" value={assessment.method.release_id ?? "none"} />
            <VersionRow label="pit mode" value={assessment.method.point_in_time_mode} />
          </div>
        </section>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>当前结论的限制</h2>
          <BadgeInfo size={18} />
        </div>
        <div className="list-stack">
          <div className="bullet-row">
            <span className="bullet-dot" />
            <span>{method.note}</span>
          </div>
          <div className="bullet-row">
            <span className="bullet-dot" />
            <span>
              {heuristicMode
                ? `当前 probability mode 为 ${assessment.method.probability_mode}，说明这仍是启发式过渡层，不能当成校准后的正式危机概率。`
                : `当前 probability mode 为 ${assessment.method.probability_mode}，已经切到 release-backed 概率层，但仍要结合数据新鲜度、回测审计和事件确认一起解释。`}
            </span>
          </div>
          <div className="bullet-row">
            <span className="bullet-dot" />
            <span>
              {degradedRelease
                ? `当前 release status 为 ${assessment.method.release_status}，因此页面上的仓位预算更适合当作执行节奏和保护框架，而不是自动交易指令。`
                : `当前 release status 为 ${assessment.method.release_status}，表示当前线上版本处于正式服务状态，但仓位建议仍应配合你的账户约束和流动性条件执行。`}
            </span>
          </div>
          <div className="bullet-row">
            <span className="bullet-dot" />
            <span>{posture.summary}</span>
          </div>
        </div>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>受保护压力窗口目录</h2>
          <ShieldCheck size={18} />
        </div>
        <div className="rule-box">
          <strong>目录说明</strong>
          <span>{method.protected_stress_window_catalog.note}</span>
        </div>
        <div className="mini-metrics">
          <Metric label="目录版本" value={method.protected_stress_window_catalog.catalog_id} />
          <Metric label="市场范围" value={method.protected_stress_window_catalog.market_scope.toUpperCase()} />
          <Metric label="窗口数量" value={`${method.protected_stress_window_catalog.windows.length}`} />
        </div>
        <div className="rule-box">
          <strong>配置来源</strong>
          <span>{method.protected_stress_window_catalog.source}</span>
        </div>
        {method.protected_stress_window_catalog.warning ? (
          <div className="rule-box">
            <strong>配置告警</strong>
            <span>{method.protected_stress_window_catalog.warning}</span>
          </div>
        ) : null}
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>窗口</th>
                <th>开始</th>
                <th>结束</th>
                <th>说明</th>
              </tr>
            </thead>
            <tbody>
              {method.protected_stress_window_catalog.windows.map((window) => (
                <tr key={window.window_id}>
                  <td>
                    <strong>{window.label}</strong>
                    <span>{window.window_id}</span>
                  </td>
                  <td>{formatDate(window.start_date)}</td>
                  <td>{formatDate(window.end_date)}</td>
                  <td>{window.note}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}

export function EventsView({
  assessment,
  events
}: {
  assessment: AssessmentSnapshot;
  events: AlertEvent[];
}) {
  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>事件层结论</h2>
            <Radar size={18} />
          </div>
          <div className="jpy-state">
            <span className="state-pill">{eventStateLabel(assessment.event_assessment.state)}</span>
            <b>{formatNumber(assessment.event_assessment.confirmation_score)}</b>
          </div>
          <p className="body-copy">{assessment.event_assessment.summary}</p>
          <div className="list-stack compact">
            {assessment.event_assessment.confirmed_signals.map((signal, index) => (
              <div className="bullet-row" key={`${signal}-${index}`}>
                <span className="bullet-dot" />
                <span>{signal}</span>
              </div>
            ))}
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>待补确认</h2>
            <Activity size={18} />
          </div>
          <div className="list-stack compact">
            {assessment.event_assessment.pending_gaps.length === 0 ? (
              <div className="bullet-row">
                <span className="bullet-dot" />
                <span>当前没有明显待补确认项。</span>
              </div>
            ) : (
              assessment.event_assessment.pending_gaps.map((gap, index) => (
                <div className="bullet-row" key={`${gap}-${index}`}>
                  <span className="bullet-dot" />
                  <span>{gap}</span>
                </div>
              ))
            )}
          </div>
        </section>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>最近事件</h2>
          <Radar size={18} />
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>日期</th>
                <th>类型</th>
                <th>级别</th>
                <th>说明</th>
                <th>相关指标</th>
              </tr>
            </thead>
            <tbody>
              {events.map((event) => (
                <tr key={event.alert_id}>
                  <td>{formatDate(event.triggered_as_of_date)}</td>
                  <td>{event.event_type}</td>
                  <td>{event.level}</td>
                  <td>{event.trigger_reason}</td>
                  <td>{event.related_indicators.join(", ")}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}

export function BacktestsView({
  assessment,
  backtests,
  timeline
}: {
  assessment: AssessmentSnapshot;
  backtests: BacktestScenarioSummary[];
  timeline: BacktestWindowPoint[];
}) {
  const option = {
    animation: false,
    grid: { left: 42, right: 18, top: 20, bottom: 32 },
    legend: { bottom: 0 },
    xAxis: {
      type: "category",
      data: timeline.map((point) => formatDate(point.as_of_date))
    },
    yAxis: {
      type: "value",
      max: 1
    },
    series: [
      {
        name: "5d",
        type: "line",
        smooth: true,
        symbol: "none",
        data: timeline.map((point) => point.p_5d)
      },
      {
        name: "20d",
        type: "line",
        smooth: true,
        symbol: "none",
        data: timeline.map((point) => point.p_20d)
      },
      {
        name: "60d",
        type: "line",
        smooth: true,
        symbol: "none",
        data: timeline.map((point) => point.p_60d)
      }
    ]
  };

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>回测摘要</h2>
            <History size={18} />
          </div>
          <p className="body-copy">{assessment.backtest_summary.summary}</p>
          <div className="mini-metrics">
            <Metric label="结构抬升率" value={formatPercent(assessment.backtest_summary.structural_warning_rate)} />
            <Metric label="可执行预警率" value={formatPercent(assessment.backtest_summary.timely_warning_rate)} />
            <Metric label="漏报率" value={formatPercent(assessment.backtest_summary.missed_rate)} />
            <Metric label="平均结构提前量" value={formatNumber(assessment.backtest_summary.avg_structural_lead_time_days, "d")} />
            <Metric label="平均动作提前量" value={formatNumber(assessment.backtest_summary.avg_lead_time_days, "d")} />
            <Metric label="预警折返" value={formatNumber(assessment.backtest_summary.total_false_positive_count)} />
            <Metric label="真实样本" value={formatNumber(assessment.backtest_summary.real_scenario_count)} />
            <Metric label="模板样本" value={formatNumber(assessment.backtest_summary.fallback_scenario_count)} />
          </div>
          <div className="rule-box">
            <strong>历史覆盖</strong>
            <span>
              {assessment.backtest_summary.history_start && assessment.backtest_summary.history_end
                ? `${formatDate(assessment.backtest_summary.history_start)} - ${formatDate(assessment.backtest_summary.history_end)}`
                : "当前没有可用历史区间。"}
            </span>
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>滚动审计</h2>
            <ShieldCheck size={18} />
          </div>
          <p className="body-copy">{assessment.backtest_summary.rolling_audit.summary}</p>
          <div className="mini-metrics">
            <Metric label="动作信号精度" value={formatPercent(assessment.backtest_summary.rolling_audit.actionable_precision)} />
            <Metric label="动作信号点" value={formatNumber(assessment.backtest_summary.rolling_audit.actionable_signal_count)} />
            <Metric label="危机前命中点" value={formatNumber(assessment.backtest_summary.rolling_audit.pre_crisis_signal_count)} />
            <Metric label="危机中信号点" value={formatNumber(assessment.backtest_summary.rolling_audit.in_crisis_signal_count)} />
            <Metric label="受保护压力点" value={formatNumber(assessment.backtest_summary.rolling_audit.stress_window_signal_count)} />
            <Metric label="纯误报点" value={formatNumber(assessment.backtest_summary.rolling_audit.false_positive_signal_count)} />
            <Metric label="误报区间" value={formatNumber(assessment.backtest_summary.rolling_audit.false_positive_episode_count)} />
            <Metric label="最长误报区间" value={formatNumber(assessment.backtest_summary.rolling_audit.longest_false_positive_episode_days, "d")} />
          </div>
          <div className="rule-box">
            <strong>审计口径</strong>
            <span>
              危机前命中看的是危机前 20 日内是否出现动作级预警；受保护压力用于承认 2009 余震、2015-2016 美元流动性压力、
              2022 加息冲击这类应允许防守的系统压力窗口；纯误报才代表模型应继续收紧的噪声。
            </span>
          </div>
          <div className="rule-box">
            <strong>区间展示规则</strong>
            <span>下表按持续时间排序，只展示最长的 12 段非危机动作区间，便于快速定位最需要复盘的阶段。</span>
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>Posture 解释</h2>
            <ShieldCheck size={18} />
          </div>
          <div className="rule-box">
            <strong>当前 posture</strong>
            <span>{postureLabel(assessment.posture)}</span>
          </div>
          <div className="rule-box">
            <strong>回测用途</strong>
            <span>看的是这套评估体系在历史压力阶段能否提前给出足够强的准备或防守信号。</span>
          </div>
        </section>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>历史轨迹</h2>
          <History size={18} />
        </div>
        <ReactECharts option={option} style={{ height: 280 }} notMerge />
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>场景样本</h2>
          <History size={18} />
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>场景</th>
                <th>样本来源</th>
                <th>危机区间</th>
                <th>结构抬升</th>
                <th>动作预警</th>
                <th>峰值</th>
                <th>折返</th>
                <th>说明</th>
              </tr>
            </thead>
            <tbody>
              {backtests.map((scenario) => (
                <tr key={scenario.scenario_id}>
                  <td>{scenario.name}</td>
                  <td>{backtestSignalSourceLabel(scenario.signal_source)}</td>
                  <td>
                    {formatDate(scenario.crisis_start)} - {formatDate(scenario.crisis_end)}
                  </td>
                  <td>{scenario.lead_time_days ?? "—"}d</td>
                  <td>{scenario.actionable_lead_time_days ?? "—"}d</td>
                  <td>{formatNumber(scenario.max_score)}</td>
                  <td>{formatNumber(scenario.false_positive_count)}</td>
                  <td>{scenario.note}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>

      <section className="surface">
        <div className="surface-head">
          <h2>非危机动作区间</h2>
          <ShieldCheck size={18} />
        </div>
        {assessment.backtest_summary.rolling_audit.classified_episodes.length > 0 ? (
          <div className="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>类型</th>
                  <th>开始</th>
                  <th>结束</th>
                  <th>持续</th>
                  <th>信号点</th>
                  <th>说明</th>
                </tr>
              </thead>
              <tbody>
                {assessment.backtest_summary.rolling_audit.classified_episodes.map((episode) => (
                  <tr key={`${episode.classification}-${episode.start_date}-${episode.end_date}`}>
                    <td>
                      <span className={`state-pill ${auditEpisodeClass(episode.classification)}`}>
                        {auditEpisodeLabel(episode.classification)}
                      </span>
                    </td>
                    <td>{formatDate(episode.start_date)}</td>
                    <td>{formatDate(episode.end_date)}</td>
                    <td>{formatNumber(episode.duration_days, "d")}</td>
                    <td>{formatNumber(episode.signal_count)}</td>
                    <td>{episode.note}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        ) : (
          <p className="body-copy">当前没有可展示的非危机动作区间。</p>
        )}
      </section>
    </section>
  );
}

export function AuditView({
  assessment,
  audit
}: {
  assessment: AssessmentSnapshot;
  audit: ResearchAuditResponse;
}) {
  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>当前线上版本</h2>
            <History size={18} />
          </div>
          <div className="mini-metrics">
            <Metric label="运行模式" value={audit.runtime_probability_mode} />
            <Metric label="服务状态" value={audit.runtime_release_status} />
            <Metric label="active release" value={audit.active_release_id ?? "none"} />
            <Metric label="最新快照" value={formatDate(audit.latest_snapshot_date)} />
          </div>
          <div className="rule-box">
            <strong>审计说明</strong>
            <span>{audit.note}</span>
          </div>
          <div className="rule-box">
            <strong>当前 assessment method</strong>
            <span>
              {assessment.method.probability_mode} / {assessment.method.release_status} /{" "}
              {assessment.method.release_id ?? "none"}
            </span>
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>如何看这页</h2>
            <ShieldCheck size={18} />
          </div>
          <GuideList
            rows={[
              ["release registry", "看线上当前登记了哪些候选版、正式版和历史版。"],
              ["runtime mode", "看 API 现在真正正在用的是 heuristic 还是 formal bundle。"],
              ["snapshot history", "看每天落库的概率快照是否和当前 active release 对得上。"],
              ["降级识别", "如果 release manifest 是正式版，但 runtime mode 退回 heuristic，说明 bundle 加载或服务检查失败。"]]}
          />
        </section>
      </section>

      {!audit.supported ? (
        <section className="surface">
          <div className="surface-head">
            <h2>当前环境</h2>
            <Database size={18} />
          </div>
          <p className="body-copy">
            当前 data mode 为 {audit.storage_mode}，暂时没有可展示的本地 release / snapshot 审计数据。
          </p>
        </section>
      ) : (
        <>
          <section className="surface">
            <div className="surface-head">
              <h2>Release Registry</h2>
              <History size={18} />
            </div>
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>release</th>
                    <th>状态</th>
                    <th>prob mode</th>
                    <th>服务</th>
                    <th>训练区间</th>
                    <th>评估</th>
                    <th>创建时间</th>
                  </tr>
                </thead>
                <tbody>
                  {audit.releases.map((release) => (
                    <tr key={release.release_id}>
                      <td>
                        <strong>{release.release_id}</strong>
                        <span>{release.bundle_uri}</span>
                      </td>
                      <td>
                        <strong>{release.status}</strong>
                        <span>{release.point_in_time_mode}</span>
                      </td>
                      <td>{release.probability_mode}</td>
                      <td>{release.serving_status}</td>
                      <td>
                        {formatDate(release.training_range_start)} -{" "}
                        {formatDate(release.training_range_end)}
                      </td>
                      <td>
                        <strong>
                          Brier {release.brier_score !== null ? release.brier_score.toFixed(4) : "—"}
                        </strong>
                        <span>
                          LogLoss {release.log_loss !== null ? release.log_loss.toFixed(4) : "—"} / ECE{" "}
                          {release.ece !== null ? release.ece.toFixed(4) : "—"}
                        </span>
                      </td>
                      <td>{formatDateTime(release.created_at)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>

          <section className="surface">
            <div className="surface-head">
              <h2>Prediction Snapshots</h2>
              <Database size={18} />
            </div>
            <div className="table-wrap">
              <table>
                <thead>
                  <tr>
                    <th>日期</th>
                    <th>release</th>
                    <th>mode</th>
                    <th>5d / 20d / 60d</th>
                    <th>posture</th>
                    <th>freshness</th>
                    <th>覆盖</th>
                    <th>记录时间</th>
                  </tr>
                </thead>
                <tbody>
                  {audit.snapshots.map((snapshot) => (
                    <tr key={`${snapshot.as_of_date}-${snapshot.release_id ?? "inline"}-${snapshot.recorded_at}`}>
                      <td>{formatDate(snapshot.as_of_date)}</td>
                      <td>
                        <strong>{snapshot.release_id ?? "inline"}</strong>
                        <span>{snapshot.point_in_time_mode}</span>
                      </td>
                      <td>
                        <strong>{snapshot.probability_mode}</strong>
                        <span>{snapshot.release_status}</span>
                      </td>
                      <td>
                        <strong>
                          {formatPercent(snapshot.calibrated_p_5d)} / {formatPercent(snapshot.calibrated_p_20d)} /{" "}
                          {formatPercent(snapshot.calibrated_p_60d)}
                        </strong>
                        <span>
                          raw {formatPercent(snapshot.raw_p_5d)} / {formatPercent(snapshot.raw_p_20d)} /{" "}
                          {formatPercent(snapshot.raw_p_60d)}
                        </span>
                      </td>
                      <td>
                        <strong>{snapshot.posture}</strong>
                        <span>{snapshot.time_to_risk_bucket}</span>
                      </td>
                      <td>{snapshot.freshness_status}</td>
                      <td>{formatPercent(snapshot.coverage_score)}</td>
                      <td>{formatDateTime(snapshot.recorded_at)}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </section>
        </>
      )}
    </section>
  );
}


function GuideList({ rows }: { rows: Array<[string, string]> }) {
  return (
    <div className="list-stack">
      {rows.map(([title, text]) => (
        <div className="list-row" key={title}>
          <div>
            <strong>{title}</strong>
            <span>{text}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

function DriverList({
  rows,
  reverse = false
}: {
  rows: AssessmentSnapshot["top_risk_drivers"];
  reverse?: boolean;
}) {
  return (
    <div className="list-stack">
      {rows.map((row) => (
        <div className="list-row" key={row.indicator_id}>
          <div>
            <strong>{row.display_name}</strong>
            <span>{row.explanation}</span>
          </div>
          <b>{formatNumber(reverse ? 100 - row.score : row.score)}</b>
        </div>
      ))}
    </div>
  );
}

function VersionRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="version-row">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}
