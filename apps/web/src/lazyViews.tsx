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
  RiskSnapshot
} from "./types";
import {
  eventStateLabel,
  formatDate,
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
          指标风险分来自各指标自己的历史分位和方向规则，不同指标的原始值不能直接互相比较。
        </div>
        <div className="table-wrap">
          <table>
            <thead>
              <tr>
                <th>指标</th>
                <th>维度</th>
                <th>最新值</th>
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
              ["GDELT", "仍处于 prototype，新闻事件链路和本地事件存储尚未接入当前运行链路。"]
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
            <span>现在的概率还是启发式 MVP，不能当成校准后的正式违约概率或危机发生率。</span>
          </div>
          <div className="bullet-row">
            <span className="bullet-dot" />
            <span>真正要做到“提前一周离场”，还需要事件层、回测和校准链路继续加强。</span>
          </div>
          <div className="bullet-row">
            <span className="bullet-dot" />
            <span>{posture.summary}</span>
          </div>
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
            <Metric label="有效预警率" value={formatPercent(assessment.backtest_summary.timely_warning_rate)} />
            <Metric label="漏报率" value={formatPercent(assessment.backtest_summary.missed_rate)} />
            <Metric label="平均提前量" value={formatNumber(assessment.backtest_summary.avg_lead_time_days, "d")} />
            <Metric label="误报次数" value={formatNumber(assessment.backtest_summary.total_false_positive_count)} />
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
                <th>危机区间</th>
                <th>提前量</th>
                <th>峰值</th>
                <th>误报</th>
              </tr>
            </thead>
            <tbody>
              {backtests.map((scenario) => (
                <tr key={scenario.scenario_id}>
                  <td>{scenario.name}</td>
                  <td>
                    {formatDate(scenario.crisis_start)} - {formatDate(scenario.crisis_end)}
                  </td>
                  <td>{scenario.lead_time_days ?? "—"}d</td>
                  <td>{formatNumber(scenario.max_score)}</td>
                  <td>{formatNumber(scenario.false_positive_count)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </section>
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
