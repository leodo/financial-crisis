import { BadgeInfo, History, ShieldCheck } from "lucide-react";
import { formatDate } from "../../format";
import type {
  AssessmentMethodResponse,
  AssessmentSnapshot,
  PostureGuidance
} from "../../types";
import {
  BulletList,
  DetailRows,
  GuideList,
  MetricGrid,
  MetricPairsGrid,
  ResponsiveTable,
  renderClauseBulletRows,
  RuleBox,
  StackedTableCell,
  SurfaceHeader,
  VersionRow
} from "../shared/panelHelpers";
import { methodContent } from "./content";
import { useMethodViewModel } from "./useMethodViewModel";

export default function MethodView({
  assessment,
  posture,
  method
}: {
  assessment: AssessmentSnapshot;
  posture: PostureGuidance;
  method: AssessmentMethodResponse;
}) {
  const {
    headlineMetrics,
    versionRows,
    priorActionRows,
    runtimeMetrics,
    triggerClauses,
    blockerClauses,
    overlayHeadlineMetrics,
    overlayHorizonRows,
    overlayAuditRows,
    scenarioCoverageMetrics,
    scenarioCoverageRows,
    scenarioCoverageCatalogId,
    scenarioCoverageCatalogSource,
    scenarioCoverageCatalogNote,
    historyProvenanceMetrics,
    historyProvenanceRows,
    historyProvenanceNote,
    historyProvenanceReplayRunId,
    limitations,
    historyPolicyVersion,
    protectedCatalogId,
    protectedCatalogSource,
    protectedCatalogNote
  } = useMethodViewModel({
    assessment,
    posture,
    method
  });

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="当前方法摘要" icon={BadgeInfo} />
          <MetricGrid items={headlineMetrics} />
        </section>

        <section className="surface">
          <SurfaceHeader title="方法分层" icon={BadgeInfo} />
          <GuideList rows={methodContent.layerGuideRows} />
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="版本与边界" icon={History} />
          <div className="version-list">
            {versionRows.map((row) => (
              <VersionRow key={row.label} {...row} />
            ))}
          </div>
      </section>

      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="先验和动作概率怎么区分" icon={BadgeInfo} />
          <GuideList rows={priorActionRows} />
        </section>

        <section className="surface">
          <SurfaceHeader title="当前运行阈值" icon={History} />
          <RuleBox label="怎么看这些百分比">{methodContent.runtimeThresholdNote}</RuleBox>
          <MetricPairsGrid pairs={runtimeMetrics} />
          <RuleBox label="历史评估策略版本">
            <span title={historyPolicyVersion.hint}>{historyPolicyVersion.value}</span>
          </RuleBox>
          <RuleBox label="当前执行条款">
            <div className="list-stack compact">
              {renderClauseBulletRows({
                clauses: triggerClauses,
                emptyText: methodContent.clauseTriggerEmpty
              })}
              {blockerClauses.length > 0 && (
                renderClauseBulletRows({
                  clauses: blockerClauses,
                  leadText: methodContent.blockerLead
                })
              )}
            </div>
          </RuleBox>
        </section>
      </section>

      <section className="surface">
        <SurfaceHeader title="历史轨迹证据来源" icon={History} />
        <RuleBox label="怎么看">{historyProvenanceNote}</RuleBox>
        <MetricGrid items={historyProvenanceMetrics} />
        {historyProvenanceReplayRunId ? (
          <RuleBox label="最近 replay run">
            <span title={historyProvenanceReplayRunId}>{historyProvenanceReplayRunId}</span>
          </RuleBox>
        ) : null}
        {historyProvenanceRows.length > 0 ? (
          <DetailRows items={historyProvenanceRows} />
        ) : (
          <RuleBox label="当前状态">当前默认历史窗口里还没有可用 provenance 统计。</RuleBox>
        )}
      </section>

      <section className="surface">
        <SurfaceHeader title="当前结论的限制" icon={BadgeInfo} />
        <BulletList items={limitations} />
      </section>

      <section className="surface">
        <SurfaceHeader title="Family Overlay 诊断" icon={BadgeInfo} />
        <RuleBox label="怎么看">{methodContent.overlayIntro}</RuleBox>
        <MetricGrid items={overlayHeadlineMetrics} />
        {overlayHorizonRows.length > 0 ? (
          <DetailRows items={overlayHorizonRows} />
        ) : (
          <RuleBox label="当前状态">{methodContent.overlayEmpty}</RuleBox>
        )}
        {overlayAuditRows.length > 0 ? (
          <ResponsiveTable
            className="wide-table"
            columns={["窗口", "Family", "场景 / 正例", "Train / Cal / Eval", "Gate active", "说明"]}
            note={methodContent.overlayTableNote}
          >
            {overlayAuditRows.map((row) => (
              <tr key={row.id}>
                <td>{row.horizonLabel}</td>
                <td>{row.familyLabel}</td>
                <td>{row.scenarioSummary}</td>
                <td>{row.splitSummary}</td>
                <td>{row.gateSummary}</td>
                <td>{row.note}</td>
              </tr>
            ))}
          </ResponsiveTable>
        ) : null}
      </section>

      <section className="surface">
        <SurfaceHeader title="历史场景数据覆盖" icon={History} />
        <RuleBox label="怎么看">{methodContent.scenarioCoverageIntro}</RuleBox>
        <MetricPairsGrid
          pairs={[
            ["覆盖版本", scenarioCoverageCatalogId.value],
            ["市场范围", method.scenario_data_coverage_catalog.market_scope.toUpperCase()],
            ["场景数量", `${method.scenario_data_coverage_catalog.records.length}`]
          ]}
        />
        <MetricGrid items={scenarioCoverageMetrics} />
        <RuleBox label="配置来源">
          <span title={scenarioCoverageCatalogSource.hint}>
            {scenarioCoverageCatalogSource.value}
          </span>
        </RuleBox>
        <RuleBox label="目录说明">{scenarioCoverageCatalogNote}</RuleBox>
        {method.scenario_data_coverage_catalog.warning ? (
          <RuleBox label="配置告警">{method.scenario_data_coverage_catalog.warning}</RuleBox>
        ) : null}
        <ResponsiveTable
          className="wide-table"
          columns={["场景", "推荐角色", "覆盖 / PIT", "免费主源", "当前状态 / 主要缺口"]}
          note={methodContent.scenarioCoverageTableNote}
        >
          {scenarioCoverageRows.map((row) => (
            <tr key={row.id}>
              <StackedTableCell title={row.scenarioLabel} details={row.scenarioId} />
              <td>{row.roleSummary}</td>
              <td>{row.gradeSummary}</td>
              <td>{row.sourceSummary}</td>
              <StackedTableCell title={row.statusSummary} details={row.gapSummary} />
            </tr>
          ))}
        </ResponsiveTable>
      </section>

      <section className="surface">
        <SurfaceHeader title="受保护压力窗口目录" icon={ShieldCheck} />
        <RuleBox label="目录说明">{protectedCatalogNote}</RuleBox>
        <MetricPairsGrid
          pairs={[
            ["目录版本", protectedCatalogId.value],
            ["市场范围", method.protected_stress_window_catalog.market_scope.toUpperCase()],
            ["窗口数量", `${method.protected_stress_window_catalog.windows.length}`]
          ]}
        />
        <RuleBox label="配置来源">
          <span title={protectedCatalogSource.hint}>{protectedCatalogSource.value}</span>
        </RuleBox>
        {method.protected_stress_window_catalog.warning ? (
          <RuleBox label="配置告警">{method.protected_stress_window_catalog.warning}</RuleBox>
        ) : null}
        <ResponsiveTable
          className="wide-table"
          columns={["窗口", "开始", "结束", "说明"]}
          note={methodContent.tableNote}
        >
          {method.protected_stress_window_catalog.windows.map((window) => (
            <tr key={window.window_id}>
              <StackedTableCell title={window.label} details={window.window_id} />
              <td>{formatDate(window.start_date)}</td>
              <td>{formatDate(window.end_date)}</td>
              <td>{window.note}</td>
            </tr>
          ))}
        </ResponsiveTable>
      </section>
    </section>
  );
}
