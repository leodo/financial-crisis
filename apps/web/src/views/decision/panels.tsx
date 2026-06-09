import {
  Activity,
  ArrowUpRight,
  ChartColumnIncreasing,
  Database,
  GitCompareArrows,
  ShieldCheck
} from "lucide-react";
import {
  humanizeNarrativeCopy,
  eventStateLabel,
  formatNumber,
  formatPercent,
  jpyStateLabel
} from "../../format";
import { SimpleGroupedBarChart } from "../../simpleCharts";
import type { AssessmentSnapshot, RiskSnapshot } from "../../types";
import {
  BulletList,
  DetailRows,
  DriverList,
  MetricGrid,
  PillTableCell,
  ResponsiveTable,
  RuleBox,
  StateSummary,
  SurfaceHeader,
  type MetricItem
} from "../shared/panelHelpers";
import { BudgetBar } from "./components";
import { decisionContent } from "./content";
import type { GroupedBarChartModel } from "./charts";
import type {
  DecisionAnalogRow,
  DecisionRollingAuditEpisodeRow
} from "./useDecisionViewModel";

function backtestSummaryCopy(assessment: AssessmentSnapshot) {
  const summary = assessment.backtest_summary;
  if (summary.real_scenario_count !== 0 && summary.timely_warning_rate !== 0) {
    return summary.summary;
  }

  const localCoverage =
    summary.real_scenario_count === 0
      ? `当前危机场景目录共 ${summary.scenario_count} 个样本，本地 SQLite 历史窗口暂未直接覆盖这些危机场景；${summary.fallback_scenario_count} 个样本仍作为模板参照。`
      : `当前危机场景目录共 ${summary.scenario_count} 个样本，其中 ${summary.real_scenario_count} 个已被本地历史窗口覆盖。`;
  const structural = `结构性抬升至少提前 7 天出现的比例约为 ${formatPercent(
    summary.structural_warning_rate
  )}。`;
  const action =
    summary.timely_warning_rate === 0
      ? "动作级预警暂未形成，页面下方会把它显示为“未形成动作预警”，不能解释成采集失败。"
      : `动作级预警至少提前 7 天出现的比例约为 ${formatPercent(summary.timely_warning_rate)}。`;

  return `${localCoverage}${structural}${action}`;
}

export function DecisionWhyNowPanel({
  assessment,
  posture
}: {
  assessment: AssessmentSnapshot;
  posture: { reasons: string[]; upgrade_condition: string };
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="为什么是现在" icon={Activity} />
      <BulletList items={posture.reasons.map(humanizeNarrativeCopy)} />
      <div className="driver-preview">
        <strong>{decisionContent.panels.whyNowTopDrivers}</strong>
        <DriverList rows={assessment.top_risk_drivers.slice(0, 3)} />
      </div>
      <RuleBox label="升级条件">{humanizeNarrativeCopy(posture.upgrade_condition)}</RuleBox>
    </section>
  );
}

export function DecisionReliefPanel({
  assessment,
  posture,
  overview
}: {
  assessment: AssessmentSnapshot;
  posture: { downgrade_condition: string };
  overview: RiskSnapshot;
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="为什么还没更糟" icon={ShieldCheck} />
      <p className="body-copy">{decisionContent.panels.reliefBody}</p>
      <DriverList rows={assessment.top_relief_drivers.slice(0, 3)} reverse />
      <RuleBox label="降级条件">{humanizeNarrativeCopy(posture.downgrade_condition)}</RuleBox>
      <RuleBox label="旧版评分层辅助解释">{humanizeNarrativeCopy(overview.level_reason)}</RuleBox>
    </section>
  );
}

export function DecisionAnalogPanel({
  analogChart,
  analogRows
}: {
  analogChart: GroupedBarChartModel;
  analogRows: DecisionAnalogRow[];
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="历史类比" icon={GitCompareArrows} />
      <SimpleGroupedBarChart model={analogChart} height={300} />
      <div className="legend-note">
        蓝柱表示当前总风险强度，橙柱表示对应历史场景的压力峰值。先看当前距离历史峰值还有多远，再看下面每个样本给过多长提前量。
      </div>
      <DetailRows
        items={analogRows.map((analog) => ({
          id: analog.id,
          title: analog.title,
          detail: analog.detail,
          meta: analog.score
        }))}
      />
    </section>
  );
}

export function DecisionActionPlanPanel({
  assessment,
  actionPlanMetrics
}: {
  assessment: AssessmentSnapshot;
  actionPlanMetrics: MetricItem[];
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="组合动作建议" icon={ChartColumnIncreasing} />
      <p className="body-copy">{humanizeNarrativeCopy(assessment.position_guidance.action_summary)}</p>
      <MetricGrid items={actionPlanMetrics} />
      <RuleBox label="执行节奏">{humanizeNarrativeCopy(assessment.position_guidance.execution_urgency)}</RuleBox>
      <RuleBox label="执行确认门槛">{humanizeNarrativeCopy(assessment.position_guidance.confidence_gate)}</RuleBox>
      {assessment.position_guidance.capital_preservation_overlay_enabled ? (
        <RuleBox label="资本保全叠加已打开">
          {decisionContent.panels.actionPlanCapitalPreservation}
        </RuleBox>
      ) : null}
      <div className="surface-grid">
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
          valueLabel={
            assessment.position_guidance.hedge_ratio_pct === 0 ? "暂不对冲" : undefined
          }
          note={
            assessment.position_guidance.hedge_ratio_pct === 0
              ? "当前未进入对冲或防守节奏，系统暂不建议增加保护覆盖。"
              : "核心仓位应有多少保护覆盖。"
          }
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
      <div className="surface-grid">
        <RuleBox label="建议动作">
          <BulletList items={assessment.position_guidance.actions.map(humanizeNarrativeCopy)} compact />
        </RuleBox>
        <RuleBox label="当前先不要做什么">
          <BulletList items={assessment.position_guidance.forbidden_actions.map(humanizeNarrativeCopy)} compact />
        </RuleBox>
      </div>
      <RuleBox label="什么情况下再恢复仓位">
        <BulletList items={assessment.position_guidance.reentry_conditions.map(humanizeNarrativeCopy)} compact />
      </RuleBox>
      <RuleBox label="执行护栏">
        <BulletList items={assessment.position_guidance.guardrails.map(humanizeNarrativeCopy)} compact />
      </RuleBox>
      <RuleBox label="系统边界">
        <span>{decisionContent.panels.actionPlanGovernance}</span>
        <span>
          {assessment.position_guidance.governance.system_budget_only
            ? "当前输出是系统预算建议，不是个性化投资建议。"
            : "当前输出可直接执行。"}
        </span>
        <span>
          {assessment.position_guidance.governance.auto_execution_allowed
            ? "当前版本允许自动执行。"
            : "当前版本禁止自动执行，仍需人工确认。"}
        </span>
        <span>
          {assessment.position_guidance.governance.policy_change_requires_release_review &&
          assessment.position_guidance.governance.policy_change_requires_go_no_go
            ? "任何动作规则升级都必须先过 release review，再满足正式 Go/No-Go。"
            : "当前版本没有额外的 release review / Go-No-Go 约束。"}
        </span>
      </RuleBox>
      <RuleBox label="执行前人工复核">
        <span>{decisionContent.panels.actionPlanChecks}</span>
        <BulletList
          items={assessment.position_guidance.governance.required_operator_checks.map(
            humanizeNarrativeCopy
          )}
          compact
        />
      </RuleBox>
    </section>
  );
}

export function DecisionEventPanel({
  assessment
}: {
  assessment: AssessmentSnapshot;
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="事件层确认" icon={Activity} />
      <StateSummary
        pillLabel={eventStateLabel(assessment.event_assessment.state)}
        score={formatNumber(assessment.event_assessment.confirmation_score)}
        summary={humanizeNarrativeCopy(assessment.event_assessment.summary)}
      />
      <div className="surface-grid">
        <RuleBox label={decisionContent.panels.eventConfirmedTitle}>
          <BulletList
            items={assessment.event_assessment.confirmed_signals.map(humanizeNarrativeCopy)}
            compact
            emptyText={decisionContent.panels.eventConfirmedEmpty}
            emptyVariant="inline"
          />
        </RuleBox>
        <RuleBox label={decisionContent.panels.eventPendingTitle}>
          <BulletList
            items={assessment.event_assessment.pending_gaps.map(humanizeNarrativeCopy)}
            compact
            emptyText={decisionContent.panels.eventPendingEmpty}
            emptyVariant="inline"
          />
        </RuleBox>
      </div>
    </section>
  );
}

export function DecisionJpyCarryPanel({
  assessment,
  jpyCarryMetrics
}: {
  assessment: AssessmentSnapshot;
  jpyCarryMetrics: MetricItem[];
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="日元套息放大器" icon={ArrowUpRight} />
      <StateSummary
        pillLabel={jpyStateLabel(assessment.jpy_carry.state)}
        pillClassName={`state-${assessment.jpy_carry.state}`}
        score={formatNumber(assessment.jpy_carry.score)}
        summary={humanizeNarrativeCopy(assessment.jpy_carry.reason)}
      />
      <MetricGrid items={jpyCarryMetrics} />
      <div className="legend-note">
        {decisionContent.panels.jpyCarryLegend}
      </div>
    </section>
  );
}

export function DecisionBacktestSummaryPanel({
  assessment,
  backtestSummaryMetrics,
  historyCoverageText,
  coverageScopeText
}: {
  assessment: AssessmentSnapshot;
  backtestSummaryMetrics: MetricItem[];
  historyCoverageText: string;
  coverageScopeText: string;
}) {
  return (
    <section className="surface">
      <SurfaceHeader title="历史表现与当前约束" icon={Database} />
      <RuleBox label="历史表现摘要">
        {humanizeNarrativeCopy(backtestSummaryCopy(assessment))}
      </RuleBox>
      <MetricGrid items={backtestSummaryMetrics} />
      <RuleBox label="口径区分">{humanizeNarrativeCopy(coverageScopeText)}</RuleBox>
      <div className="surface-grid">
        <RuleBox label="场景回测历史窗口">{historyCoverageText}</RuleBox>
        <RuleBox label="当前组合约束">
          {`风险档位 ${assessment.user_preferences.profile === "neutral" ? "中性" : assessment.user_preferences.profile === "conservative" ? "保守" : "进取"}，现金底线 ${assessment.user_preferences.cash_floor_pct.toFixed(0)}%，风险资产上限 ${assessment.user_preferences.max_equity_cap_pct.toFixed(0)}%，杠杆上限 ${assessment.user_preferences.max_leverage_pct.toFixed(0)}%，期权保护偏好 ${assessment.user_preferences.option_overlay_preference_pct.toFixed(0)}%。`}
        </RuleBox>
      </div>
    </section>
  );
}

export function DecisionRollingAuditPanel({
  assessment,
  rollingAuditMetrics,
  rollingAuditHistoryText,
  rollingAuditScopeText,
  rollingAuditBoundaryText,
  rollingAuditEpisodes
}: {
  assessment: AssessmentSnapshot;
  rollingAuditMetrics: MetricItem[];
  rollingAuditHistoryText: string;
  rollingAuditScopeText: string;
  rollingAuditBoundaryText: string;
  rollingAuditEpisodes: DecisionRollingAuditEpisodeRow[];
}) {
  const rollingAudit = assessment.backtest_summary.rolling_audit;
  const rollingAuditSummary =
    rollingAudit.actionable_signal_count === 0
      ? `全历史滚动审计覆盖 ${rollingAudit.history_start} 到 ${rollingAudit.history_end}；当前运行口径没有发出准备/对冲/防守动作信号，因此不能把“动作信号精度”解释为 0% 命中率，只能说明本窗口没有可评估的动作信号。`
      : rollingAudit.summary;

  return (
    <section className="surface">
      <SurfaceHeader title="滚动审计与误报边界" icon={Database} />
      <RuleBox label="历史滚动审计结论">
        {humanizeNarrativeCopy(rollingAuditSummary)}
      </RuleBox>
      <MetricGrid items={rollingAuditMetrics} />
      <div className="surface-grid">
        <RuleBox label="滚动审计历史窗口">{rollingAuditHistoryText}</RuleBox>
        <RuleBox label="口径区分">{humanizeNarrativeCopy(rollingAuditScopeText)}</RuleBox>
      </div>
      <RuleBox label="统计口径">{decisionContent.panels.rollingAuditDefinition}</RuleBox>
      <RuleBox label="这组结果怎么用">{rollingAuditBoundaryText}</RuleBox>
      {rollingAuditEpisodes.length > 0 ? (
        <ResponsiveTable columns={["类型", "区间", "持续", "信号点", "说明"]}>
          {rollingAuditEpisodes.map((episode) => (
              <tr key={episode.key}>
                <PillTableCell
                  className={episode.classificationClass}
                  label={episode.classificationLabel}
                />
                <td>{episode.interval}</td>
                <td>{episode.duration}</td>
                <td>{episode.signalCount}</td>
                <td>{humanizeNarrativeCopy(episode.note)}</td>
              </tr>
            ))}
        </ResponsiveTable>
      ) : null}
    </section>
  );
}
