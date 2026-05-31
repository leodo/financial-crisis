import { Suspense, lazy, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import ReactECharts from "./charts";
import {
  Activity,
  ArrowUpRight,
  BadgeInfo,
  ChartColumnIncreasing,
  Database,
  GitCompareArrows,
  History,
  Layers3,
  Radar,
  RefreshCw,
  ShieldCheck,
  Siren,
  Table2
} from "lucide-react";
import { api } from "./api";
import {
  formatDate,
  formatDateTime,
  dataModeLabel,
  eventStateLabel,
  formatNumber,
  formatSignedNumber,
  formatPercent,
  freshnessLabel,
  jpyStateLabel,
  postureClass,
  postureLabel,
  qualityLabel,
  timeBucketLabel,
  userProfileLabel
} from "./format";
import type {
  AssessmentHistoryPoint,
  AssessmentMethodResponse,
  AssessmentSnapshot,
  AlertEvent,
  BacktestScenarioSummary,
  BacktestWindowPoint,
  DataSource,
  IndicatorRisk,
  PostureGuidance,
  RiskSnapshot
} from "./types";

const DriversView = lazy(async () => {
  const module = await import("./lazyViews");
  return { default: module.DriversView };
});
const IndicatorsView = lazy(async () => {
  const module = await import("./lazyViews");
  return { default: module.IndicatorsView };
});
const SourcesView = lazy(async () => {
  const module = await import("./lazyViews");
  return { default: module.SourcesView };
});
const MethodView = lazy(async () => {
  const module = await import("./lazyViews");
  return { default: module.MethodView };
});
const EventsView = lazy(async () => {
  const module = await import("./lazyViews");
  return { default: module.EventsView };
});
const BacktestsView = lazy(async () => {
  const module = await import("./lazyViews");
  return { default: module.BacktestsView };
});

type View = "decision" | "drivers" | "events" | "backtests" | "indicators" | "sources" | "method";

const navItems: Array<{ id: View; label: string; icon: typeof Activity }> = [
  { id: "decision", label: "决策面板", icon: ShieldCheck },
  { id: "drivers", label: "风险驱动", icon: Layers3 },
  { id: "events", label: "事件确认", icon: Radar },
  { id: "backtests", label: "回测表现", icon: History },
  { id: "indicators", label: "指标细项", icon: Table2 },
  { id: "sources", label: "数据可信度", icon: Database },
  { id: "method", label: "方法说明", icon: BadgeInfo }
];

const liveQueryOptions = {
  refetchInterval: 60_000,
  refetchIntervalInBackground: true,
  refetchOnMount: "always" as const,
  refetchOnWindowFocus: true,
  staleTime: 10_000
};

export default function App() {
  const [view, setView] = useState<View>("decision");
  const queryClient = useQueryClient();

  const assessment = useQuery({
    queryKey: ["assessment-current"],
    queryFn: api.assessmentCurrent,
    ...liveQueryOptions
  });
  const assessmentHistory = useQuery({
    queryKey: ["assessment-history"],
    queryFn: api.assessmentHistory,
    ...liveQueryOptions
  });
  const posture = useQuery({
    queryKey: ["assessment-posture"],
    queryFn: api.assessmentPosture,
    ...liveQueryOptions
  });
  const method = useQuery({
    queryKey: ["assessment-method"],
    queryFn: api.assessmentMethod,
    ...liveQueryOptions
  });
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview, ...liveQueryOptions });
  const indicators = useQuery({
    queryKey: ["indicators"],
    queryFn: api.indicators,
    ...liveQueryOptions
  });
  const events = useQuery({
    queryKey: ["events-recent"],
    queryFn: api.eventsRecent,
    ...liveQueryOptions
  });
  const sources = useQuery({ queryKey: ["sources"], queryFn: api.sources, ...liveQueryOptions });
  const backtests = useQuery({
    queryKey: ["backtests"],
    queryFn: api.backtests,
    ...liveQueryOptions
  });
  const backtestTimeline = useQuery({
    queryKey: ["backtests-timeline"],
    queryFn: api.backtestTimeline,
    ...liveQueryOptions
  });
  const reload = useMutation({
    mutationFn: api.systemReload,
    onSuccess: async () => {
      await queryClient.invalidateQueries();
    }
  });

  const isLoading =
    assessment.isLoading ||
    assessmentHistory.isLoading ||
    posture.isLoading ||
    method.isLoading ||
    overview.isLoading ||
    indicators.isLoading ||
    events.isLoading ||
    sources.isLoading ||
    backtests.isLoading ||
    backtestTimeline.isLoading;
  const error =
    assessment.error ??
    assessmentHistory.error ??
    posture.error ??
    method.error ??
    overview.error ??
    indicators.error ??
    events.error ??
    sources.error ??
    backtests.error ??
    backtestTimeline.error;
  const errorText = error instanceof Error ? error.message : "未知错误";

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <ShieldCheck size={22} />
          <div>
            <strong>金融危机概率评估</strong>
            <span>US Crisis Decision Console</span>
          </div>
        </div>

        <nav className="nav">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                className={view === item.id ? "nav-item active" : "nav-item"}
                onClick={() => setView(item.id)}
                type="button"
                title={item.label}
              >
                <Icon size={18} />
                <span>{item.label}</span>
              </button>
            );
          })}
        </nav>

        <section className="sidebar-note">
          <strong>这不是自动交易指令</strong>
          <span>面板输出的是系统评估、时间窗口和 posture，不替代个性化仓位决策。</span>
        </section>
      </aside>

      <main className="main">
        <header className="topbar">
          <div>
            <h1>美国金融危机风险决策面板</h1>
            <p>把风险强度、危机概率、历史类比和数据可信度分层展示。</p>
          </div>
          <div className="topbar-side">
            <button
              className={reload.isPending ? "icon-button spinning" : "icon-button"}
              disabled={reload.isPending}
              onClick={() => {
                reload.mutate();
              }}
              title="重新加载本地库并刷新面板"
              type="button"
            >
              <RefreshCw size={16} />
            </button>
            {assessment.data && (
              <div className="meta-strip">
                <span>As of {assessment.data.as_of_date}</span>
                <span>Mode {dataModeLabel(assessment.data.runtime.data_mode)}</span>
                <span>Latest {assessment.data.runtime.latest_observation_at ?? "n/a"}</span>
                <span>Generated {formatDateTime(assessment.data.runtime.generated_at)}</span>
                <span>{timeBucketLabel(assessment.data.time_to_risk_bucket)}</span>
                <span>Data {qualityLabel(assessment.data.data_trust.quality_grade)}</span>
              </div>
            )}
          </div>
        </header>

        {isLoading && <div className="notice">正在加载评估数据…</div>}
        {error && <div className="notice error">API 请求失败：{errorText}</div>}
        {reload.isError && (
          <div className="notice error">重新加载本地库失败，请检查 API 日志或数据源状态。</div>
        )}

        {!isLoading &&
          !error &&
          assessment.data &&
          assessmentHistory.data &&
          posture.data &&
          method.data &&
          overview.data &&
          indicators.data &&
          events.data &&
          sources.data &&
          backtests.data &&
          backtestTimeline.data && (
            <>
              {view === "decision" && (
                <DecisionView
                  assessment={assessment.data}
                  history={assessmentHistory.data}
                  posture={posture.data}
                  overview={overview.data}
                  backtests={backtests.data}
                />
              )}
              <Suspense fallback={<div className="notice">正在加载视图…</div>}>
                {view === "drivers" && (
                  <DriversView
                    assessment={assessment.data}
                    overview={overview.data}
                    posture={posture.data}
                  />
                )}
                {view === "events" && (
                  <EventsView assessment={assessment.data} events={events.data} />
                )}
                {view === "backtests" && (
                  <BacktestsView
                    assessment={assessment.data}
                    backtests={backtests.data}
                    timeline={backtestTimeline.data}
                  />
                )}
                {view === "indicators" && <IndicatorsView indicators={indicators.data} />}
                {view === "sources" && (
                  <SourcesView assessment={assessment.data} sources={sources.data} />
                )}
                {view === "method" && (
                  <MethodView
                    assessment={assessment.data}
                    posture={posture.data}
                    method={method.data}
                  />
                )}
              </Suspense>
            </>
          )}
      </main>
    </div>
  );
}

function DecisionView({
  assessment,
  history,
  posture,
  overview,
  backtests
}: {
  assessment: AssessmentSnapshot;
  history: AssessmentHistoryPoint[];
  posture: PostureGuidance;
  overview: RiskSnapshot;
  backtests: BacktestScenarioSummary[];
}) {
  const probabilityTrendOption = useMemo(
    () => buildProbabilityTrendOption(history),
    [history]
  );
  const layerScoreOption = useMemo(() => buildLayerScoreOption(assessment), [assessment]);
  const analogOption = useMemo(
    () => buildAnalogOption(assessment, backtests),
    [assessment, backtests]
  );
  const nearestAnalog = assessment.historical_analogs[0];
  const currentRiskBand = describeRiskScoreBand(assessment.scores.overall_score);
  const usdJpyIndicator = assessment.key_indicators.find(
    (item) => item.indicator_id === "us_external_usdjpy_level"
  );

  return (
    <section className="workspace">
      <section className="callout">
        <BadgeInfo size={18} />
        <div>
          <strong>风险强度分不是危机概率。</strong>
          <span>
            `overall / structural / trigger / external` 反映的是压力位置；真正用于决策的是
            `5d / 20d / 60d` 概率、time bucket 和 posture。
          </span>
        </div>
      </section>

      {assessment.runtime.stale_warning && (
        <section className={assessment.runtime.demo_mode ? "notice error" : "notice"}>
          <strong>
            {assessment.runtime.demo_mode ? "当前是 Demo 数据" : "当前数据存在时效性提醒"}
          </strong>
          <div>{assessment.runtime.stale_warning}</div>
        </section>
      )}

      <section className="runtime-surface">
        <div className="runtime-header">
          <div>
            <strong>当前数据状态</strong>
            <span>
              这是基于免费日频/周频数据的危机预警面板，不是逐笔行情终端。先看日期和模式，再解读数值。
            </span>
          </div>
          <span className={assessment.runtime.demo_mode ? "runtime-chip runtime-chip-demo" : "runtime-chip"}>
            {assessment.runtime.demo_mode
              ? "Demo 样例"
              : `${dataModeLabel(assessment.runtime.data_mode)} 已加载`}
          </span>
        </div>

        <div className="runtime-card-grid">
          <div className="runtime-card">
            <span>最新关键观测</span>
            <strong>{formatDate(assessment.runtime.latest_observation_at)}</strong>
            <small>
              {assessment.runtime.latest_observation_lag_days === null
                ? "当前没有可用滞后信息。"
                : `距离请求日滞后 ${assessment.runtime.latest_observation_lag_days} 天。`}
            </small>
          </div>
          <div className="runtime-card">
            <span>本次评估生成</span>
            <strong>{formatDateTime(assessment.runtime.generated_at)}</strong>
            <small>点击右上角刷新按钮可以立即重新载入本地库。</small>
          </div>
          <div className="runtime-card">
            <span>当前 USDJPY</span>
            <strong>{formatNumber(usdJpyIndicator?.latest_value)}</strong>
            <small>
              {usdJpyIndicator?.latest_as_of_date
                ? `${formatDate(usdJpyIndicator.latest_as_of_date)} · ${usdJpyIndicator.source_id ?? "—"} · ${freshnessLabel(usdJpyIndicator.status)}`
                : "缺少 USDJPY 最新观测。"}
            </small>
          </div>
          <div className="runtime-card">
            <span>系统节奏</span>
            <strong>日频预警</strong>
            <small>更适合判断未来几天到数周的风险窗口，不适合替代盘中报价软件。</small>
          </div>
        </div>
      </section>

      <section className="decision-row">
        <section className={`hero-surface ${postureClass(assessment.posture)}`}>
          <span className="kicker">当前 posture</span>
          <div className="hero-value">{postureLabel(assessment.posture)}</div>
          <div className="hero-subtitle">
            风险窗口判断：{timeBucketLabel(assessment.time_to_risk_bucket)}
          </div>
          <p>{posture.summary}</p>
          <div className="hero-metrics">
            <Metric label="Conviction" value={formatPercent(assessment.conviction_score)} />
            <Metric label="数据覆盖" value={formatPercent(assessment.data_trust.coverage_score)} />
            <Metric label="风险强度" value={formatNumber(assessment.scores.overall_score)} />
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>离风险还有多远</h2>
            <Siren size={18} />
          </div>
          <div className="probability-grid">
            <ProbabilityTile
              label="5 个交易日"
              value={assessment.probabilities.p_5d}
              hint="用于判断是不是已经接近急性风险窗口。"
            />
            <ProbabilityTile
              label="20 个交易日"
              value={assessment.probabilities.p_20d}
              hint="用于判断未来几周是否应考虑保护性对冲。"
            />
            <ProbabilityTile
              label="60 个交易日"
              value={assessment.probabilities.p_60d}
              hint="用于判断中期脆弱性是否已经积累。"
            />
          </div>
          <div className="legend-note">
            `5d` 看急性冲击，`20d` 看未来几周是否需要离场和保护，`60d` 看数月级脆弱性。
          </div>
          <div className="rule-box">
            <strong>时距判断</strong>
            <span>{describeTimeBucket(assessment.time_to_risk_bucket)}</span>
          </div>
          <div className="rule-box">
            <strong>历史参照</strong>
            <span>{describeAnalogWindow(nearestAnalog, assessment.time_to_risk_bucket)}</span>
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>四档 posture 在做什么</h2>
            <ShieldCheck size={18} />
          </div>
          <p className="body-copy">
            posture 是系统建议的风险处理节奏，从观察到防守一共四档，当前高亮的是系统结论。
          </p>
          <PostureLadder current={assessment.posture} />
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>总风险强度怎么读</h2>
            <ChartColumnIncreasing size={18} />
          </div>
          <div className="score-summary">
            <div className="score-summary-head">
              <span className="kicker">当前总风险强度</span>
              <div className="score-value">{formatNumber(assessment.scores.overall_score)}</div>
              <p>{currentRiskBand.description}</p>
            </div>
            <div className="score-band-list">
              {RISK_SCORE_BANDS.map((band) => {
                const active = currentRiskBand.label === band.label;
                return (
                  <div className={active ? "score-band active" : "score-band"} key={band.label}>
                    <div>
                      <strong>{band.label}</strong>
                      <span>{band.rangeText}</span>
                    </div>
                    <span>{band.note}</span>
                  </div>
                );
              })}
            </div>
          </div>
          <div className="legend-note">
            `0-100` 强度分只是指标组合所处的历史压力位置。即使接近 `100`，也不等于危机已经发生，
            更不等于 `100%` 会发生。
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>组合动作建议</h2>
            <ChartColumnIncreasing size={18} />
          </div>
          <p className="body-copy">{assessment.position_guidance.action_summary}</p>
          <div className="budget-stack">
            <BudgetBar
              label="风险资产上限"
              value={assessment.position_guidance.target_equity_exposure_pct}
              note="风险窗口打开时，系统建议先压低总暴露。"
              tone="risk"
            />
            <BudgetBar
              label="现金目标"
              value={assessment.position_guidance.target_cash_pct}
              note="用于应对流动性冲击和执行保护动作。"
              tone="cash"
            />
            <BudgetBar
              label="对冲覆盖"
              value={assessment.position_guidance.hedge_ratio_pct}
              note="核心仓位应有多少保护覆盖。"
              tone="hedge"
            />
            <BudgetBar
              label="杠杆上限"
              value={assessment.position_guidance.leverage_cap_pct}
              note="风险窗口内不宜维持高杠杆。"
              tone="leverage"
            />
            <BudgetBar
              label="期权保护"
              value={assessment.position_guidance.option_overlay_pct}
              note="可用来做尾部保护，而不是替代全部风控。"
              tone="option"
            />
          </div>
          <div className="list-stack compact">
            {assessment.position_guidance.actions.map((action, index) => (
              <div className="bullet-row" key={`${action}-${index}`}>
                <span className="bullet-dot" />
                <span>{action}</span>
              </div>
            ))}
          </div>
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>关键指标是否最新</h2>
            <Database size={18} />
          </div>
          <div className="list-stack">
            {assessment.key_indicators.map((item) => (
              <div className="list-row" key={`${item.entity_id}-${item.indicator_id}`}>
                <div>
                  <strong>
                    {item.display_name} · {freshnessLabel(item.status)}
                  </strong>
                  <span>
                    {formatNumber(item.latest_value)} {item.unit} · 日期{" "}
                    {item.latest_as_of_date ? formatDate(item.latest_as_of_date) : "—"} · 来源{" "}
                    {item.source_id ?? "—"}
                    {item.lag_days !== null ? ` · 滞后 ${item.lag_days} 天` : ""}
                  </span>
                  <span>{item.note}</span>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>事件层确认</h2>
            <Activity size={18} />
          </div>
          <div className="jpy-state">
            <span className="state-pill">{eventStateLabel(assessment.event_assessment.state)}</span>
            <b>{formatNumber(assessment.event_assessment.confirmation_score)}</b>
          </div>
          <p className="body-copy">{assessment.event_assessment.summary}</p>
          <div className="list-stack compact">
            {assessment.event_assessment.confirmed_signals.map((item, index) => (
              <div className="bullet-row" key={`${item}-${index}`}>
                <span className="bullet-dot" />
                <span>{item}</span>
              </div>
            ))}
            {assessment.event_assessment.pending_gaps.map((item, index) => (
              <div className="bullet-row" key={`${item}-${index}`}>
                <span className="bullet-dot" />
                <span>{item}</span>
              </div>
            ))}
          </div>
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>概率轨迹</h2>
            <History size={18} />
          </div>
          <ReactECharts option={probabilityTrendOption} style={{ height: 300 }} notMerge />
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>历史类比</h2>
            <GitCompareArrows size={18} />
          </div>
          <ReactECharts option={analogOption} style={{ height: 260 }} notMerge />
          <div className="list-stack">
            {assessment.historical_analogs.map((analog) => (
              <div className="list-row" key={analog.scenario_id}>
                <div>
                  <strong>{analog.name}</strong>
                  <span>
                    相似度 {formatNumber(analog.similarity_score)} · 历史样本提前{" "}
                    {analog.lead_time_days ?? "—"} 天 · {analog.note}
                  </span>
                </div>
                <b>{formatNumber(analog.similarity_score)}</b>
              </div>
            ))}
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>日元套息放大器</h2>
            <ArrowUpRight size={18} />
          </div>
          <div className="jpy-state">
            <span className={`state-pill state-${assessment.jpy_carry.state}`}>
              {jpyStateLabel(assessment.jpy_carry.state)}
            </span>
            <b>{formatNumber(assessment.jpy_carry.score)}</b>
          </div>
          <p className="body-copy">{assessment.jpy_carry.reason}</p>
          <div className="mini-metrics">
            <Metric
              label="USDJPY"
              value={formatNumber(assessment.jpy_carry.usdjpy_level)}
              hint={
                usdJpyIndicator?.latest_as_of_date
                  ? `${formatDate(usdJpyIndicator.latest_as_of_date)} · ${usdJpyIndicator.source_id ?? "—"}`
                  : "无最新日期"
              }
            />
            <Metric label="5d 变化" value={formatNumber(assessment.jpy_carry.change_5d)} />
            <Metric label="日短端" value={formatNumber(assessment.jpy_carry.jp_call_rate, "%")} />
            <Metric label="美短端" value={formatNumber(assessment.jpy_carry.us_short_rate, "%")} />
            <Metric
              label="美日利差"
              value={formatSignedNumber(assessment.jpy_carry.us_jp_short_rate_diff, 2, "%")}
            />
            <Metric label="20d 波动" value={formatNumber(assessment.jpy_carry.realized_vol_20d)} />
            <Metric label="融资压力" value={formatNumber(assessment.jpy_carry.funding_pressure_score)} />
            <Metric label="VIX 联动" value={formatNumber(assessment.jpy_carry.vix_coupling_score)} />
          </div>
          <div className="legend-note">
            这张卡不是在预测日本危机，而是在看日元融资环境是否可能放大美国风险资产的同步回撤。
          </div>
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>为什么是现在</h2>
            <Activity size={18} />
          </div>
          <div className="list-stack">
            {posture.reasons.map((reason, index) => (
              <div className="bullet-row" key={`${reason}-${index}`}>
                <span className="bullet-dot" />
                <span>{reason}</span>
              </div>
            ))}
          </div>
          <div className="driver-preview">
            <strong>当前最强的上行驱动</strong>
            <DriverList rows={assessment.top_risk_drivers.slice(0, 3)} />
          </div>
          <div className="rule-box">
            <strong>升级条件</strong>
            <span>{posture.upgrade_condition}</span>
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>为什么还没更糟</h2>
            <ShieldCheck size={18} />
          </div>
          <p className="body-copy">
            这些缓冲因素解释了为什么系统虽然进入高压 posture，但还没有把所有场景都推到最坏假设。
          </p>
          <DriverList rows={assessment.top_relief_drivers.slice(0, 3)} reverse />
          <div className="rule-box">
            <strong>降级条件</strong>
            <span>{posture.downgrade_condition}</span>
          </div>
          <div className="rule-box">
            <strong>旧风险引擎解释</strong>
            <span>{overview.level_reason}</span>
          </div>
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>风险层拆解</h2>
            <Layers3 size={18} />
          </div>
          <ReactECharts option={layerScoreOption} style={{ height: 300 }} notMerge />
          <div className="legend-note">
            结构性风险决定脆弱性底色，触发性风险决定窗口压缩速度，外部冲击决定是否出现共振放大。
          </div>
        </section>

        <section className="surface">
          <div className="surface-head">
            <h2>回测与用户参数</h2>
            <Database size={18} />
          </div>
          <div className="rule-box">
            <strong>回测摘要</strong>
            <span>{assessment.backtest_summary.summary}</span>
          </div>
          <div className="mini-metrics">
            <Metric
              label="有效预警率"
              value={formatPercent(assessment.backtest_summary.timely_warning_rate)}
            />
            <Metric label="漏报率" value={formatPercent(assessment.backtest_summary.missed_rate)} />
            <Metric
              label="平均提前量"
              value={formatNumber(assessment.backtest_summary.avg_lead_time_days, "d")}
            />
            <Metric
              label="误报次数"
              value={formatNumber(assessment.backtest_summary.total_false_positive_count)}
            />
            <Metric
              label="真实样本"
              value={formatNumber(assessment.backtest_summary.real_scenario_count)}
            />
            <Metric
              label="模板样本"
              value={formatNumber(assessment.backtest_summary.fallback_scenario_count)}
            />
            <Metric
              label="用户风险档位"
              value={userProfileLabel(assessment.user_preferences.profile)}
            />
          </div>
          <div className="rule-box">
            <strong>历史覆盖</strong>
            <span>
              {assessment.backtest_summary.history_start && assessment.backtest_summary.history_end
                ? `${formatDate(assessment.backtest_summary.history_start)} - ${formatDate(assessment.backtest_summary.history_end)}`
                : "当前没有可用历史区间。"}
            </span>
          </div>
          <div className="rule-box">
            <strong>用户约束</strong>
            <span>{assessment.user_preferences.note}</span>
          </div>
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>可信度与数据缺口</h2>
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
          <div className="list-stack compact">
            {assessment.data_trust.warnings.map((warning, index) => (
              <div className="bullet-row" key={`${warning}-${index}`}>
                <span className="bullet-dot" />
                <span>{warning}</span>
              </div>
            ))}
          </div>
        </section>
      </section>

      <section className="band-grid">
        <section className="surface">
          <div className="surface-head">
            <h2>执行护栏</h2>
            <ShieldCheck size={18} />
          </div>
          <div className="list-stack compact">
            {assessment.position_guidance.guardrails.map((guardrail, index) => (
              <div className="bullet-row" key={`${guardrail}-${index}`}>
                <span className="bullet-dot" />
                <span>{guardrail}</span>
              </div>
            ))}
          </div>
        </section>
      </section>
    </section>
  );
}

function ProbabilityTile({
  label,
  value,
  hint
}: {
  label: string;
  value: number;
  hint: string;
}) {
  const band = describeProbabilityBand(value);

  return (
    <div className={`probability-tile ${band.className}`}>
      <div className="tile-head">
        <span>{label}</span>
        <em>{band.label}</em>
      </div>
      <strong>{formatPercent(value)}</strong>
      <p>{hint}</p>
      <small>{band.note}</small>
    </div>
  );
}

function Metric({
  label,
  value,
  hint
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{value}</strong>
      {hint && <small className="metric-note">{hint}</small>}
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


function PostureLadder({ current }: { current: AssessmentSnapshot["posture"] }) {
  return (
    <div className="posture-ladder">
      {POSTURE_STEPS.map((step) => {
        const active = step.id === current;
        return (
          <div className={active ? "posture-step active" : "posture-step"} key={step.id}>
            <div className="posture-step-head">
              <strong>{step.label}</strong>
              {active && <span>当前</span>}
            </div>
            <p>{step.description}</p>
          </div>
        );
      })}
    </div>
  );
}

function BudgetBar({
  label,
  value,
  note,
  tone
}: {
  label: string;
  value: number;
  note: string;
  tone: "risk" | "cash" | "hedge" | "leverage" | "option";
}) {
  return (
    <div className="budget-bar">
      <div className="budget-bar-head">
        <strong>{label}</strong>
        <span>{formatNumber(value, "%")}</span>
      </div>
      <div className="track budget-track">
        <div className={`fill budget-fill tone-${tone}`} style={{ width: `${value}%` }} />
      </div>
      <span className="budget-note">{note}</span>
    </div>
  );
}

const POSTURE_STEPS: Array<{
  id: AssessmentSnapshot["posture"];
  label: string;
  description: string;
}> = [
  { id: "normal", label: "正常观察", description: "没有看到近端风险窗口，保持监控。 " },
  { id: "prepare", label: "提前准备", description: "脆弱性在积累，先准备现金和对冲工具。" },
  { id: "hedge", label: "保护性对冲", description: "未来几周风险升高，保护动作需要前置。" },
  { id: "defend", label: "防守优先", description: "短期窗口已打开，先保流动性和资本。 " }
];

const RISK_SCORE_BANDS = [
  {
    label: "常态区",
    min: 0,
    maxExclusive: 45,
    rangeText: "0 - 45",
    note: "缓冲因素占优，系统通常不会给出高 posture。"
  },
  {
    label: "积累区",
    min: 45,
    maxExclusive: 60,
    rangeText: "45 - 60",
    note: "脆弱性开始积累，需要盯触发因子。"
  },
  {
    label: "高压区",
    min: 60,
    maxExclusive: 75,
    rangeText: "60 - 75",
    note: "系统会结合概率和数据可信度考虑保护动作。"
  },
  {
    label: "危机样态区",
    min: 75,
    maxExclusive: 101,
    rangeText: "75 - 100",
    note: "更接近历史危机高压样态，但仍不等于危机已经发生。"
  }
] as const;

function describeProbabilityBand(value: number) {
  if (value < 0.15) {
    return {
      label: "低位",
      className: "band-low",
      note: "更像常态观察区，通常不需要明显收缩仓位。"
    };
  }
  if (value < 0.3) {
    return {
      label: "准备区",
      className: "band-prepare",
      note: "开始准备流动性和保护工具，避免被动离场。"
    };
  }
  if (value < 0.5) {
    return {
      label: "对冲区",
      className: "band-hedge",
      note: "未来几周风险抬升，保护动作通常要前置。"
    };
  }
  return {
    label: "防守区",
    className: "band-defend",
    note: "系统更倾向先保现金、流动性和保护覆盖。"
  };
}

function describeRiskScoreBand(score: number) {
  const band =
    RISK_SCORE_BANDS.find((item) => score >= item.min && score < item.maxExclusive) ??
    RISK_SCORE_BANDS[RISK_SCORE_BANDS.length - 1];

  return {
    label: band.label,
    description: `当前位于${band.label}。${band.note}`
  };
}

function describeTimeBucket(bucket: AssessmentSnapshot["time_to_risk_bucket"]) {
  const mapping: Record<AssessmentSnapshot["time_to_risk_bucket"], string> = {
    normal: "系统还没有看到可交易的近端风险窗口，更偏向常态监控。",
    months: "脆弱性在积累，但更像数月级风险，而不是马上发生的冲击。",
    weeks: "风险已经压缩到数周级别，应该提前准备流动性和保护动作。",
    now: "短期风险窗口已经打开，更接近历史危机前 1 到 4 周或当下冲击区间。"
  };

  return mapping[bucket];
}

function describeAnalogWindow(
  analog: AssessmentSnapshot["historical_analogs"][number] | undefined,
  bucket: AssessmentSnapshot["time_to_risk_bucket"]
) {
  if (!analog) {
    return describeTimeBucket(bucket);
  }

  if (analog.lead_time_days === null) {
    return `当前最接近 ${analog.name} 的压力阶段，但该历史样本没有可用提前量估计。`;
  }

  return `当前最接近 ${analog.name} 的风险窗口，历史上大约提前 ${analog.lead_time_days} 天进入类似高压阶段。`;
}

function buildProbabilityTrendOption(history: AssessmentHistoryPoint[]) {
  return {
    animation: false,
    grid: { left: 42, right: 18, top: 24, bottom: 36 },
    legend: {
      bottom: 0,
      textStyle: { color: "#5d6972" }
    },
    tooltip: {
      trigger: "axis"
    },
    xAxis: {
      type: "category",
      data: history.map((point) => formatDate(point.as_of_date)),
      axisLine: { lineStyle: { color: "#cfd7dc" } },
      axisLabel: { color: "#5d6972" }
    },
    yAxis: {
      type: "value",
      max: 1,
      axisLabel: {
        color: "#5d6972",
        formatter: (value: number) => `${Math.round(value * 100)}%`
      },
      splitLine: { lineStyle: { color: "#edf1f4" } }
    },
    series: [
      {
        name: "5d",
        type: "line",
        smooth: true,
        symbol: "none",
        lineStyle: { width: 3, color: "#b45309" },
        data: history.map((point) => point.p_5d)
      },
      {
        name: "20d",
        type: "line",
        smooth: true,
        symbol: "none",
        lineStyle: { width: 3, color: "#2563eb" },
        data: history.map((point) => point.p_20d)
      },
      {
        name: "60d",
        type: "line",
        smooth: true,
        symbol: "none",
        areaStyle: { color: "rgba(17, 94, 89, 0.08)" },
        lineStyle: { width: 3, color: "#115e59" },
        data: history.map((point) => point.p_60d)
      }
    ]
  };
}

function buildLayerScoreOption(assessment: AssessmentSnapshot) {
  return {
    animation: false,
    grid: { left: 42, right: 18, top: 16, bottom: 24 },
    xAxis: {
      type: "value",
      max: 100,
      axisLabel: { color: "#5d6972" },
      splitLine: { lineStyle: { color: "#edf1f4" } }
    },
    yAxis: {
      type: "category",
      axisLabel: { color: "#334048" },
      data: ["结构性", "触发性", "外部冲击", "总风险强度"]
    },
    series: [
      {
        type: "bar",
        barWidth: 16,
        itemStyle: {
          color: ({ dataIndex }: { dataIndex: number }) =>
            ["#115e59", "#2563eb", "#8b5cf6", "#b45309"][dataIndex]
        },
        data: [
          assessment.scores.structural_score,
          assessment.scores.trigger_score,
          assessment.scores.external_shock_score,
          assessment.scores.overall_score
        ]
      }
    ]
  };
}

function buildAnalogOption(
  assessment: AssessmentSnapshot,
  backtests: BacktestScenarioSummary[]
) {
  const analogNames = assessment.historical_analogs.map((analog) => analog.name);
  const peakScores = assessment.historical_analogs.map((analog) => analog.peak_score);
  const scenarioPeaks = analogNames.map((name) => {
    const match = backtests.find((scenario) => scenario.name === name);
    return match?.max_score ?? 0;
  });

  return {
    animation: false,
    grid: { left: 42, right: 16, top: 20, bottom: 30 },
    legend: {
      bottom: 0,
      textStyle: { color: "#5d6972" }
    },
    xAxis: {
      type: "category",
      axisLabel: { color: "#5d6972" },
      data: analogNames
    },
    yAxis: {
      type: "value",
      max: 100,
      axisLabel: { color: "#5d6972" },
      splitLine: { lineStyle: { color: "#edf1f4" } }
    },
    series: [
      {
        name: "当前总风险强度",
        type: "bar",
        itemStyle: { color: "#1d4ed8" },
        data: analogNames.map(() => assessment.scores.overall_score)
      },
      {
        name: "历史峰值",
        type: "bar",
        itemStyle: { color: "#b45309" },
        data: scenarioPeaks.length > 0 ? scenarioPeaks : peakScores
      }
    ]
  };
}
