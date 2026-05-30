import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import ReactECharts from "echarts-for-react";
import {
  Activity,
  AlertTriangle,
  BarChart3,
  Database,
  Gauge,
  Info,
  LineChart,
  ShieldCheck,
  Table2
} from "lucide-react";
import { api } from "./api";
import {
  formatDate,
  formatNumber,
  levelClass,
  levelLabel,
  levelPlainText,
  qualityLabel
} from "./format";
import type {
  AlertEvent,
  BacktestScenarioSummary,
  DataSource,
  DimensionScore,
  IndicatorRisk,
  RiskLevel,
  RiskSnapshot
} from "./types";

type View = "overview" | "indicators" | "alerts" | "sources" | "backtests";

const navItems: Array<{ id: View; label: string; icon: typeof Activity }> = [
  { id: "overview", label: "总览", icon: Activity },
  { id: "indicators", label: "指标", icon: Table2 },
  { id: "alerts", label: "预警", icon: AlertTriangle },
  { id: "sources", label: "数据源", icon: Database },
  { id: "backtests", label: "回测", icon: BarChart3 }
];

export default function App() {
  const [view, setView] = useState<View>("overview");
  const overview = useQuery({ queryKey: ["overview"], queryFn: api.overview });
  const indicators = useQuery({ queryKey: ["indicators"], queryFn: api.indicators });
  const alerts = useQuery({ queryKey: ["alerts"], queryFn: api.alerts });
  const sources = useQuery({ queryKey: ["sources"], queryFn: api.sources });
  const backtests = useQuery({ queryKey: ["backtests"], queryFn: api.backtests });

  const isLoading =
    overview.isLoading ||
    indicators.isLoading ||
    alerts.isLoading ||
    sources.isLoading ||
    backtests.isLoading;
  const error =
    overview.error ?? indicators.error ?? alerts.error ?? sources.error ?? backtests.error;

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <ShieldCheck size={22} />
          <div>
            <strong>金融危机预警</strong>
            <span>Risk Console</span>
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
      </aside>

      <main className="main">
        <header className="topbar">
          <div>
            <h1>金融系统风险面板</h1>
            <p>美国金融系统 MVP · 规则评分卡 · 免费数据源验证版</p>
          </div>
          {overview.data && (
            <div className="meta-strip">
              <span>As of {overview.data.as_of_date}</span>
              <span>{overview.data.method_version}</span>
              <span>Data {qualityLabel(overview.data.data_quality_summary.grade)}</span>
            </div>
          )}
        </header>

        {isLoading && <div className="notice">正在加载风险数据…</div>}
        {error && <div className="notice error">API 请求失败：{error.message}</div>}

        {!isLoading && !error && overview.data && (
          <>
            {view === "overview" && (
              <Overview
                overview={overview.data}
                indicators={indicators.data ?? []}
                alerts={alerts.data ?? []}
                sources={sources.data ?? []}
                backtests={backtests.data ?? []}
              />
            )}
            {view === "indicators" && <Indicators indicators={indicators.data ?? []} />}
            {view === "alerts" && <Alerts alerts={alerts.data ?? []} />}
            {view === "sources" && <Sources sources={sources.data ?? []} />}
            {view === "backtests" && <Backtests backtests={backtests.data ?? []} />}
          </>
        )}
      </main>
    </div>
  );
}

function Overview({
  overview,
  indicators,
  alerts,
  sources,
  backtests
}: {
  overview: RiskSnapshot;
  indicators: IndicatorRisk[];
  alerts: AlertEvent[];
  sources: DataSource[];
  backtests: BacktestScenarioSummary[];
}) {
  const trendOption = useMemo(() => buildTrendOption(overview), [overview]);
  const historyOption = useMemo(
    () => buildHistoryComparisonOption(overview, backtests),
    [overview, backtests]
  );
  const sourceIssues = sources.filter((source) => source.health.status !== "healthy");
  const topDimension = overview.dimensions[0];

  return (
    <section className="view-stack">
      <section className="explain-banner">
        <Info size={18} />
        <div>
          <strong>这里的 0-100 是风险强度分，不是金融危机概率。</strong>
          <span>
            分数越高，说明当前指标组合越接近历史压力区间；是否认定“已经危机”还需要事件确认、多个维度共振和数据质量支持。
          </span>
        </div>
      </section>

      <div className="overview-grid">
        <section className={`risk-hero ${levelClass(overview.overall_level)}`}>
          <span className="eyebrow">Risk Score · 风险强度</span>
          <div className="risk-score">{formatNumber(overview.overall_score)}</div>
          <div className="risk-level">{levelLabel(overview.overall_level)}</div>
          <p>{riskConclusion(overview)}</p>
          <div className="risk-footnote">
            当前最高风险维度：{topDimension?.label ?? "暂无"} · 数据质量{" "}
            {qualityLabel(overview.data_quality_summary.grade)}
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <h2>分数怎么读</h2>
            <Gauge size={18} />
          </div>
          <ScoreGuide />
        </section>

        <section className="panel">
          <div className="panel-heading">
            <h2>历史危机对照</h2>
            <BarChart3 size={18} />
          </div>
          <ReactECharts option={historyOption} style={{ height: 250 }} notMerge lazyUpdate />
          <div className="history-caption">
            当前样例分数与历史峰值并排显示，用来辅助理解风险强度，不代表历史情景已经重演。
          </div>
        </section>
      </div>

      <section className="panel">
        <div className="panel-heading">
          <h2>结构性风险、触发性风险与趋势</h2>
          <LineChart size={18} />
        </div>
        <div className="trend-layout">
          <div className="split-metrics">
            <Metric label="结构性风险" value={overview.structural_score} />
            <Metric label="触发性风险" value={overview.trigger_score} />
            <Metric
              label="数据质量"
              value={overview.data_quality_summary.overall_score}
              suffix=""
            />
          </div>
          <ReactECharts option={trendOption} style={{ height: 240 }} notMerge lazyUpdate />
        </div>
      </section>

      <section className="dimension-grid">
        {overview.dimensions.map((dimension) => (
          <DimensionTile key={dimension.dimension} dimension={dimension} />
        ))}
      </section>

      <section className="content-grid">
        <section className="panel">
          <div className="panel-heading">
            <h2>主要风险贡献</h2>
            <Activity size={18} />
          </div>
          <div className="contributor-list">
            {overview.top_contributors.map((contributor) => (
              <div className="contributor-row" key={contributor.indicator_id}>
                <div>
                  <strong>{contributor.display_name}</strong>
                  <span>{contributor.explanation}</span>
                </div>
                <b>{formatNumber(contributor.score)}</b>
              </div>
            ))}
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <h2>最新预警</h2>
            <AlertTriangle size={18} />
          </div>
          <div className="event-list">
            {alerts.slice(0, 4).map((alert) => (
              <div className="event-row" key={alert.alert_id}>
                <span className={`badge ${levelClass(alert.level)}`}>{levelLabel(alert.level)}</span>
                <div>
                  <strong>{alert.trigger_reason}</strong>
                  <span>{formatDate(alert.triggered_at)} · {alert.status}</span>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="panel">
          <div className="panel-heading">
            <h2>数据源异常</h2>
            <Database size={18} />
          </div>
          <div className="event-list">
            {sourceIssues.map((source) => (
              <div className="event-row" key={source.source_id}>
                <span className="badge neutral">{source.health.status}</span>
                <div>
                  <strong>{source.display_name}</strong>
                  <span>{source.health.message}</span>
                </div>
              </div>
            ))}
          </div>
        </section>
      </section>

      <section className="panel">
        <div className="panel-heading">
          <h2>当前重点指标</h2>
          <Table2 size={18} />
        </div>
        <div className="table-help">
          按当前风险贡献排序。风险分表示该指标在自身历史区间里的压力强度，不同指标的原始数值不能直接相加。
        </div>
        <IndicatorTable indicators={indicators.slice(0, 6)} compact />
      </section>
    </section>
  );
}

function Metric({ label, value, suffix = "" }: { label: string; value: number; suffix?: string }) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong>{formatNumber(value, suffix)}</strong>
    </div>
  );
}

function DimensionTile({ dimension }: { dimension: DimensionScore }) {
  return (
    <section className="dimension-tile">
      <div className="dimension-title">
        <strong>{dimension.label}</strong>
        <span className={`badge ${levelClass(dimension.level)}`}>{levelLabel(dimension.level)}</span>
      </div>
      <div className="bar-track">
        <div className={`bar-fill ${levelClass(dimension.level)}`} style={{ width: `${dimension.score}%` }} />
      </div>
      <div className="dimension-meta">
        <span>风险 {formatNumber(dimension.score)}</span>
        <span>质量 {formatNumber(dimension.quality_score)}</span>
      </div>
    </section>
  );
}

function Indicators({ indicators }: { indicators: IndicatorRisk[] }) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <h2>指标库</h2>
        <Table2 size={18} />
      </div>
      <div className="table-help">
        单指标风险分来自历史分位和方向规则。例如“越高越危险”的指标处于历史高分位时分数较高；“越低越危险”的指标处于历史低分位时分数较高。
      </div>
      <IndicatorTable indicators={indicators} />
    </section>
  );
}

function IndicatorTable({ indicators, compact = false }: { indicators: IndicatorRisk[]; compact?: boolean }) {
  const rows = [...indicators].sort((a, b) => b.score - a.score);
  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            <th>指标</th>
            {!compact && <th>维度</th>}
            <th>最新值</th>
            <th>风险分</th>
            <th>历史分位</th>
            <th>等级</th>
            <th>质量</th>
            {!compact && <th>解读</th>}
            {!compact && <th>来源</th>}
          </tr>
        </thead>
        <tbody>
          {rows.map((risk) => (
            <tr key={risk.indicator.indicator_id}>
              <td>
                <strong>{risk.indicator.display_name}</strong>
                <span>{risk.indicator.indicator_id}</span>
              </td>
              {!compact && <td>{risk.indicator.dimension}</td>}
              <td>{formatNumber(risk.latest_observation?.value)} {risk.indicator.unit}</td>
              <td>{formatNumber(risk.score)}</td>
              <td>{formatNumber(risk.percentile, "%")}</td>
              <td><span className={`badge ${levelClass(risk.level)}`}>{levelLabel(risk.level)}</span></td>
              <td>{qualityLabel(risk.quality_grade)}</td>
              {!compact && <td className="interpretation">{indicatorMeaning(risk)}</td>}
              {!compact && <td>{risk.indicator.default_source_id}</td>}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Alerts({ alerts }: { alerts: AlertEvent[] }) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <h2>预警记录</h2>
        <AlertTriangle size={18} />
      </div>
      <div className="event-list roomy">
        {alerts.map((alert) => (
          <div className="event-row" key={alert.alert_id}>
            <span className={`badge ${levelClass(alert.level)}`}>{levelLabel(alert.level)}</span>
            <div>
              <strong>{alert.trigger_reason}</strong>
              <span>
                {alert.event_type} · {alert.scope} · {formatDate(alert.triggered_at)} · {alert.status}
              </span>
            </div>
            <b>{formatNumber(alert.score)}</b>
          </div>
        ))}
      </div>
    </section>
  );
}

function Sources({ sources }: { sources: DataSource[] }) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <h2>数据源状态</h2>
        <Database size={18} />
      </div>
      <div className="table-wrap">
        <table>
          <thead>
            <tr>
              <th>数据源</th>
              <th>优先级</th>
              <th>状态</th>
              <th>质量</th>
              <th>生产可用</th>
              <th>最近成功</th>
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
                <td>{source.priority.toUpperCase()}</td>
                <td><span className="badge neutral">{source.health.status}</span></td>
                <td>{formatNumber(source.health.quality_score)}</td>
                <td>{source.production_allowed ? "是" : "否"}</td>
                <td>{formatDate(source.health.last_success_at)}</td>
                <td>{source.license_note}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function Backtests({ backtests }: { backtests: BacktestScenarioSummary[] }) {
  return (
    <section className="panel">
      <div className="panel-heading">
        <h2>回测场景</h2>
        <BarChart3 size={18} />
      </div>
      <div className="table-help">
        回测页用于理解历史危机附近的分数范围。第一版是场景摘要，后续会接入完整历史曲线、触发点和误报统计。
      </div>
      <div className="scenario-grid">
        {backtests.map((scenario) => (
          <section className="scenario" key={scenario.scenario_id}>
            <div>
              <strong>{scenario.name}</strong>
              <span>{scenario.crisis_start} 至 {scenario.crisis_end}</span>
            </div>
            <div className="scenario-metrics">
              <Metric label="最高分" value={scenario.max_score} />
              <Metric label="误报数" value={scenario.false_positive_count} />
              <Metric label="提前天数" value={scenario.lead_time_days ?? 0} />
            </div>
            <span className={`badge ${levelClass(scenario.max_level)}`}>{levelLabel(scenario.max_level)}</span>
          </section>
        ))}
      </div>
    </section>
  );
}

const scoreBands: Array<{
  level: RiskLevel;
  range: string;
  title: string;
  description: string;
}> = [
  { level: "normal", range: "0-29", title: "正常", description: "多数指标处于历史常态。" },
  { level: "watch", range: "30-49", title: "观察", description: "少数指标开始偏离。" },
  { level: "stress", range: "50-69", title: "压力", description: "多个信号提示风险积累。" },
  { level: "warning", range: "70-84", title: "预警", description: "多维度共振，需要人工关注。" },
  { level: "crisis", range: "85-100", title: "危机态", description: "接近历史危机区间，需要事件确认。" }
];

function ScoreGuide() {
  return (
    <div className="score-guide">
      {scoreBands.map((band) => (
        <div className="score-band" key={band.level}>
          <span className={`band-dot ${levelClass(band.level)}`} />
          <div>
            <strong>{band.range} · {band.title}</strong>
            <span>{band.description}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

function riskConclusion(overview: RiskSnapshot): string {
  if (overview.overall_level === "crisis") {
    return "当前分数进入危机态区间，表示风险组合接近历史危机压力区间；仍需结合事件层确认，不能单独视为官方危机判定。";
  }
  if (overview.overall_level === "warning") {
    return "当前处于预警区间，说明多个风险维度已经共振，建议重点查看贡献指标和数据质量。";
  }
  if (overview.overall_level === "stress") {
    return "当前处于压力区间，风险已经高于常态，但尚未达到危机态，需要持续观察触发性指标。";
  }
  if (overview.overall_level === "watch") {
    return "当前处于观察区间，少数指标偏离正常水平，还不足以形成系统性预警。";
  }
  return "当前处于正常区间，指标组合没有显示明显系统性压力。";
}

function indicatorMeaning(risk: IndicatorRisk): string {
  const percentile = risk.percentile === null ? "暂无足够历史分位" : `历史分位 ${formatNumber(risk.percentile, "%")}`;
  return `${percentile}；${directionLabel(risk.indicator.risk_direction)}；当前为 ${levelPlainText(risk.level)}。`;
}

function directionLabel(direction: string): string {
  const labels: Record<string, string> = {
    higher_is_riskier: "越高越危险",
    lower_is_riskier: "越低越危险",
    two_sided: "偏离正常区间越远越危险",
    falling_fast_is_riskier: "快速下降越危险",
    rising_fast_is_riskier: "快速上升越危险",
    manual_rule: "使用专项规则"
  };
  return labels[direction] ?? direction;
}

function buildTrendOption(overview: RiskSnapshot) {
  const dates = ["T-150", "T-120", "T-90", "T-60", "T-30", overview.as_of_date];
  const current = overview.overall_score;
  const values = [current - 24, current - 18, current - 14, current - 8, current - 4, current].map(
    (value) => Math.max(0, Math.round(value * 10) / 10)
  );
  return {
    grid: { top: 20, right: 20, bottom: 28, left: 36 },
    tooltip: { trigger: "axis" },
    xAxis: {
      type: "category",
      data: dates,
      axisLine: { lineStyle: { color: "#c8d0d7" } },
      axisTick: { show: false }
    },
    yAxis: {
      type: "value",
      min: 0,
      max: 100,
      splitLine: { lineStyle: { color: "#e7ebef" } }
    },
    series: [
      {
        name: "风险分",
        type: "line",
        smooth: true,
        symbolSize: 8,
        lineStyle: { width: 3, color: "#1f8a70" },
        areaStyle: { color: "rgba(31, 138, 112, 0.12)" },
        data: values
      }
    ]
  };
}

function buildHistoryComparisonOption(overview: RiskSnapshot, backtests: BacktestScenarioSummary[]) {
  const rows = [
    { name: "当前", score: overview.overall_score, level: overview.overall_level },
    ...backtests.map((scenario) => ({
      name: scenario.name.replace(" 全球金融危机", "").replace(" 美国区域银行危机", ""),
      score: scenario.max_score,
      level: scenario.max_level
    }))
  ].sort((a, b) => a.score - b.score);

  return {
    grid: { top: 18, right: 18, bottom: 26, left: 84 },
    tooltip: { trigger: "axis" },
    xAxis: {
      type: "value",
      min: 0,
      max: 100,
      splitLine: { lineStyle: { color: "#e7ebef" } }
    },
    yAxis: {
      type: "category",
      data: rows.map((row) => row.name),
      axisTick: { show: false },
      axisLine: { lineStyle: { color: "#c8d0d7" } }
    },
    series: [
      {
        name: "风险强度",
        type: "bar",
        data: rows.map((row) => ({
          value: row.score,
          itemStyle: { color: levelColor(row.level) }
        })),
        label: {
          show: true,
          position: "right",
          formatter: "{c}"
        },
        barWidth: 16
      }
    ]
  };
}

function levelColor(level: RiskLevel): string {
  const colors: Record<RiskLevel, string> = {
    normal: "#1f8a70",
    watch: "#628395",
    stress: "#c88a1d",
    warning: "#c75f2a",
    crisis: "#b23a48"
  };
  return colors[level];
}
