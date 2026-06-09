import { Activity, ClipboardCheck, Database, History, ShieldCheck } from "lucide-react";
import type { AssessmentSnapshot, ResearchAuditResponse } from "../../types";
import {
  DetailRows,
  GuideList,
  MetricGrid,
  ResponsiveTable,
  StackedTableCell,
  RuleBox,
  SurfaceHeader
} from "../shared/panelHelpers";
import { auditContent } from "./content";
import { useAuditViewModel } from "./useAuditViewModel";

export default function AuditView({
  assessment,
  audit
}: {
  assessment: AssessmentSnapshot;
  audit: ResearchAuditResponse;
}) {
  const {
    auditNote,
    runtimeMetrics,
    summaryMetrics,
    provenanceMetrics,
    provenanceRows,
    provenanceNote,
    methodSummary,
    overlayHeadlineMetrics,
    overlayHorizonRows,
    overlayAuditRows,
    overlaySummary,
    latestReleaseReview,
    latestReleaseReviewMetrics,
    latestReleaseReviewContextRows,
    latestReleaseReviewCoverageSource,
    latestReleaseReviewCoverageMetrics,
    latestReleaseReviewCoverageRows,
    latestReleaseReviewActionRows,
    latestReleaseReviewAttributionRows,
    latestScenarioPackAudit,
    latestScenarioPackAuditSource,
    latestScenarioPackAuditMetrics,
    latestScenarioPackAuditRows,
    latestWorkstreamAudit,
    latestWorkstreamAuditSource,
    latestWorkstreamAuditReport,
    latestWorkstreamAuditMetrics,
    latestWorkstreamAuditContextRows,
    latestWorkstreamSummaryRows,
    latestWorkstreamScenarioRows,
    latestRateShockAudit,
    latestRateShockAuditSource,
    latestRateShockAuditMetrics,
    latestRateShockAuditContextRows,
    latestRateShockContinuityRows,
    latestRateShockPhaseRows,
    latestRateShockActionRows,
    releaseRows,
    snapshotRows
  } = useAuditViewModel({
    assessment,
    audit
  });

  return (
    <section className="workspace">
      <section className="band-grid">
        <section className="surface">
          <SurfaceHeader title="当前线上版本" icon={History} />
          <MetricGrid items={runtimeMetrics} />
          <RuleBox label="审计说明">{auditNote}</RuleBox>
          <RuleBox label="当前评估方法">{methodSummary}</RuleBox>
        </section>

        <section className="surface">
          <SurfaceHeader title="如何看这页" icon={ShieldCheck} />
          <GuideList rows={auditContent.guideRows} />
        </section>
      </section>

      {!audit.supported ? (
        <section className="surface">
          <SurfaceHeader title="当前环境" icon={Database} />
          <p className="body-copy">
            {auditContent.unsupportedPrefix} {audit.storage_mode}
            {auditContent.unsupportedSuffix}
          </p>
        </section>
      ) : (
        <>
          <section className="surface">
            <SurfaceHeader title="审计摘要" icon={Database} />
            <MetricGrid items={summaryMetrics} />
            <p className="legend-note">{auditContent.summaryNote}</p>
            <RuleBox label="历史证据层">{provenanceNote}</RuleBox>
            <MetricGrid items={provenanceMetrics} />
            {provenanceRows.length > 0 ? <DetailRows items={provenanceRows} compact /> : null}
          </section>

          <section className="surface">
            <SurfaceHeader title="最近一次 Release Review" icon={ClipboardCheck} />
            <p className="legend-note">{auditContent.releaseReviewSummary}</p>
            {latestReleaseReview ? (
              <>
                <div className="audit-review-layout">
                  <div className="audit-review-stack">
                    <MetricGrid items={latestReleaseReviewMetrics} className="audit-review-metrics" />
                    <RuleBox label="评审建议">{latestReleaseReview.recommendation}</RuleBox>
                  </div>
                  <div className="audit-review-stack">
                    <RuleBox label="评审上下文">
                      <DetailRows items={latestReleaseReviewContextRows} compact />
                    </RuleBox>
                    {latestReleaseReviewCoverageMetrics.length > 0 ? (
                      <>
                        <RuleBox label="场景覆盖说明">{auditContent.releaseReviewCoverageSummary}</RuleBox>
                        <MetricGrid
                          items={latestReleaseReviewCoverageMetrics}
                          className="audit-review-metrics"
                        />
                        <RuleBox label="覆盖来源">
                          <span title={latestReleaseReviewCoverageSource?.hint}>
                            {latestReleaseReviewCoverageSource?.value ?? "未登记"}
                          </span>
                        </RuleBox>
                        {latestReleaseReview.scenario_coverage_catalog.warning ? (
                          <RuleBox label="配置告警">
                            {latestReleaseReview.scenario_coverage_catalog.warning}
                          </RuleBox>
                        ) : null}
                      </>
                    ) : null}
                  </div>
                </div>

                {latestReleaseReviewCoverageRows.length > 0 ? (
                  <ResponsiveTable
                    className="wide-table xwide-table"
                    columns={[
                      "场景",
                      "Family / 原始角色",
                      "目录结论 / 可用范围",
                      "覆盖 / 免费主源",
                      "当前状态 / 主要缺口"
                    ]}
                    note={auditContent.releaseReviewCoverageTableNote}
                  >
                    {latestReleaseReviewCoverageRows.map((row) => (
                      <tr key={row.id}>
                        <StackedTableCell title={row.scenarioLabel} details={row.scenarioDetails} />
                        <StackedTableCell
                          title={row.familySummary}
                          details={row.trainingRoleSummary}
                        />
                        <StackedTableCell
                          title={row.coverageRoleSummary}
                          details={row.allowedSummary}
                        />
                        <StackedTableCell title={row.gradeSummary} details={row.sourceSummary} />
                        <StackedTableCell title={row.statusSummary} details={row.gapSummary} />
                      </tr>
                    ))}
                  </ResponsiveTable>
                ) : null}

                {latestReleaseReviewActionRows.length > 0 ? (
                  <ResponsiveTable
                    className="wide-table xwide-table"
                    columns={["Action", "Workstream", "归因", "场景覆盖", "下一步"]}
                    note={auditContent.releaseReviewActionTableNote}
                  >
                    {latestReleaseReviewActionRows.map((row) => (
                      <tr key={row.id}>
                        <td>{row.actionType}</td>
                        <td>{row.workstream}</td>
                        <td>{row.attribution}</td>
                        <td>{row.scenarioSummary}</td>
                        <td>{row.recommendation}</td>
                      </tr>
                    ))}
                  </ResponsiveTable>
                ) : null}

                {latestReleaseReviewAttributionRows.length > 0 ? (
                  <ResponsiveTable
                    className="wide-table xwide-table"
                    columns={["Workstream", "归因", "Baseline/Candidate", "场景覆盖", "解释"]}
                    note={auditContent.releaseReviewAttributionTableNote}
                  >
                    {latestReleaseReviewAttributionRows.map((row) => (
                      <tr key={row.id}>
                        <td>{row.workstream}</td>
                        <td>{row.attribution}</td>
                        <td>{row.matchSummary}</td>
                        <td>
                          <strong>{row.scenarioSummary}</strong>
                          {row.scenarioDetail
                            .filter((item) => item !== null)
                            .map((item, index) => (
                              <span key={`${row.id}-scenario-${index}`}>{item}</span>
                            ))}
                        </td>
                        <td>{row.explanation}</td>
                      </tr>
                    ))}
                  </ResponsiveTable>
                ) : null}
              </>
            ) : (
              <RuleBox label="当前状态">{auditContent.releaseReviewEmpty}</RuleBox>
            )}
          </section>

          <section className="surface">
            <SurfaceHeader title="历史场景包审计" icon={ClipboardCheck} />
            <p className="legend-note">{auditContent.scenarioPackSummary}</p>
            {latestScenarioPackAudit ? (
              <>
                <MetricGrid items={latestScenarioPackAuditMetrics} className="audit-review-metrics" />
                <RuleBox label="工件来源">
                  <span title={latestScenarioPackAuditSource?.hint}>
                    {latestScenarioPackAuditSource?.value ?? "未登记"}
                  </span>
                </RuleBox>
                <ResponsiveTable
                  className="wide-table xwide-table"
                  columns={["场景", "当前判读", "结果 / 提前量", "覆盖 / 数据集", "结论"]}
                  note={auditContent.scenarioPackTableNote}
                >
                  {latestScenarioPackAuditRows.map((row) => (
                    <tr key={row.id}>
                      <StackedTableCell title={row.scenarioLabel} details={row.scenarioDetails} />
                      <StackedTableCell title={row.blockerSummary} details={row.blockerDetails} />
                      <StackedTableCell title={row.timingSummary} details={row.timingDetails} />
                      <StackedTableCell title={row.coverageSummary} details={row.coverageDetails} />
                      <StackedTableCell title={row.takeaway} details={row.gapSummary} />
                    </tr>
                  ))}
                </ResponsiveTable>
              </>
            ) : (
              <RuleBox label="当前状态">{auditContent.scenarioPackEmpty}</RuleBox>
            )}
          </section>

          <section className="surface">
            <SurfaceHeader title="Residual Workstream 审计" icon={ClipboardCheck} />
            <p className="legend-note">{auditContent.workstreamSummary}</p>
            {latestWorkstreamAudit ? (
              <>
                <MetricGrid items={latestWorkstreamAuditMetrics} className="audit-review-metrics" />
                <div className="audit-review-layout">
                  <div className="audit-review-stack">
                    <RuleBox label="工件来源">
                      <span title={latestWorkstreamAuditSource?.hint}>
                        {latestWorkstreamAuditSource?.value ?? "未登记"}
                      </span>
                    </RuleBox>
                    <RuleBox label="关联 review">
                      <span title={latestWorkstreamAuditReport?.hint}>
                        {latestWorkstreamAuditReport?.value ?? "未登记"}
                      </span>
                    </RuleBox>
                  </div>
                  <div className="audit-review-stack">
                    <RuleBox label="审计上下文">
                      <DetailRows items={latestWorkstreamAuditContextRows} compact />
                    </RuleBox>
                  </div>
                </div>
                {latestWorkstreamSummaryRows.length > 0 ? (
                  <RuleBox label="工作流摘要">
                    <DetailRows items={latestWorkstreamSummaryRows} compact />
                  </RuleBox>
                ) : null}
                {latestWorkstreamScenarioRows.length > 0 ? (
                  <ResponsiveTable
                    className="wide-table xwide-table"
                    columns={["场景 / Workstream", "Dataset / 窗口", "Split / Regime", "标签 / 动作", "覆盖 / 特征", "结论"]}
                    note={auditContent.workstreamTableNote}
                  >
                    {latestWorkstreamScenarioRows.map((row) => (
                      <tr key={row.id}>
                        <StackedTableCell title={row.scenarioLabel} details={row.scenarioDetails} />
                        <StackedTableCell title={row.datasetSummary} details={row.datasetDetails} />
                        <StackedTableCell title={row.splitSummary} details={row.regimeSummary} />
                        <StackedTableCell title={row.labelSummary} details={row.actionSummary} />
                        <StackedTableCell title={row.coverageSummary} details={row.coverageDetails} />
                        <td>{row.takeaway}</td>
                      </tr>
                    ))}
                  </ResponsiveTable>
                ) : null}
              </>
            ) : (
              <RuleBox label="当前状态">{auditContent.workstreamEmpty}</RuleBox>
            )}
          </section>

          <section className="surface">
            <SurfaceHeader title="2022 利率冲击专项审计" icon={ClipboardCheck} />
            <p className="legend-note">{auditContent.rateShockSummary}</p>
            {latestRateShockAudit ? (
              <>
                <MetricGrid items={latestRateShockAuditMetrics} className="audit-review-metrics" />
                <RuleBox label="工件来源">
                  <span title={latestRateShockAuditSource?.hint}>
                    {latestRateShockAuditSource?.value ?? "未登记"}
                  </span>
                </RuleBox>
                <RuleBox label="审计上下文">
                  <DetailRows items={latestRateShockAuditContextRows} compact />
                </RuleBox>
                {latestRateShockContinuityRows.length > 0 ? (
                  <RuleBox label="连续性焦点窗口">
                    <DetailRows items={latestRateShockContinuityRows} compact />
                  </RuleBox>
                ) : null}
                {latestRateShockPhaseRows.length > 0 ? (
                  <ResponsiveTable
                    className="wide-table xwide-table"
                    columns={["阶段", "样本", "20d 均值 / 连续性", "60d 均值 / 连续性", "阈值距离"]}
                    note={auditContent.rateShockPhaseTableNote}
                  >
                    {latestRateShockPhaseRows.map((row) => (
                      <tr key={row.id}>
                        <td>{row.label}</td>
                        <td>{row.rowCount}</td>
                        <StackedTableCell title={row.p20Summary} details={row.p20Continuity} />
                        <StackedTableCell title={row.p60Summary} details={row.p60Continuity} />
                        <td>{row.thresholdGap}</td>
                      </tr>
                    ))}
                  </ResponsiveTable>
                ) : null}
                {latestRateShockActionRows.length > 0 ? (
                  <ResponsiveTable
                    className="wide-table xwide-table"
                    columns={["动作层", "样本", "20d 均值", "连续性", "阈值附近", "峰值"]}
                    note={auditContent.rateShockActionTableNote}
                  >
                    {latestRateShockActionRows.map((row) => (
                      <tr key={row.id}>
                        <td>{row.label}</td>
                        <td>{row.rowCount}</td>
                        <td>{row.p20Summary}</td>
                        <td>{row.continuitySummary}</td>
                        <td>{row.nearThresholdSummary}</td>
                        <td>{row.maxSummary}</td>
                      </tr>
                    ))}
                  </ResponsiveTable>
                ) : null}
              </>
            ) : (
              <RuleBox label="当前状态">{auditContent.rateShockEmpty}</RuleBox>
            )}
          </section>

          <section className="surface">
            <SurfaceHeader title="Overlay 运行审计" icon={Activity} />
            <MetricGrid items={overlayHeadlineMetrics} />
            <RuleBox label="怎么看">{auditContent.overlaySummary}</RuleBox>
            <RuleBox label="当前结论">{overlaySummary}</RuleBox>
            {overlayHorizonRows.length > 0 ? (
              <DetailRows items={overlayHorizonRows} />
            ) : (
              <RuleBox label="当前状态">{auditContent.overlayEmpty}</RuleBox>
            )}
            {overlayAuditRows.length > 0 ? (
              <ResponsiveTable
                className="wide-table xwide-table"
                columns={["窗口", "Family", "场景/正例", "Train/Calib/Eval", "Gate active", "说明"]}
                note={auditContent.overlayTableNote}
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
            <SurfaceHeader title="版本登记册" icon={History} />
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={["版本", "登记状态", "概率层", "服务状态", "训练区间", "回测评估", "创建时间"]}
              note={auditContent.releaseTableNote}
            >
              {releaseRows.map((release) => (
                <tr key={release.id}>
                  <StackedTableCell
                    title={release.releaseId}
                    details={[
                      <span key={`${release.id}-bundle`} title={release.bundleUriHint}>
                        {release.bundleUri}
                      </span>,
                      release.pointInTimeMode
                    ]}
                  />
                  <td>{release.status}</td>
                  <td>{release.probabilityMode}</td>
                  <td>{release.servingStatus}</td>
                  <td>{release.trainingRange}</td>
                  <StackedTableCell title={release.evaluation} details={release.evaluationDetail} />
                  <td className="table-nowrap">{release.createdAt}</td>
                </tr>
              ))}
            </ResponsiveTable>
          </section>

          <section className="surface">
            <SurfaceHeader title="历史预测快照" icon={Database} />
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={[
                "日期",
                "版本",
                "概率层 / 服务",
                "5d / 20d / 60d",
                "执行档位",
                "新鲜度",
                "覆盖",
                "记录时间"
              ]}
              note={auditContent.snapshotTableNote}
            >
              {snapshotRows.map((snapshot) => (
                <tr key={snapshot.id}>
                  <td className="table-nowrap">{snapshot.asOfDate}</td>
                  <StackedTableCell title={snapshot.releaseId} details={snapshot.pointInTimeMode} />
                  <StackedTableCell title={snapshot.probabilityMode} details={snapshot.releaseStatus} />
                  <StackedTableCell
                    title={snapshot.calibratedSummary}
                    details={`原始轨迹 ${snapshot.rawSummary}`}
                  />
                  <StackedTableCell
                    title={snapshot.posture}
                    details={[
                      snapshot.timeBucket,
                      snapshot.triggerLabels.length > 0
                        ? `触发: ${snapshot.triggerLabels.join(" / ")}`
                        : null,
                      snapshot.blockerLabels.length > 0
                        ? `阻断: ${snapshot.blockerLabels.join(" / ")}`
                        : null
                    ]}
                  />
                  <td>{snapshot.freshnessStatus}</td>
                  <td>{snapshot.coverage}</td>
                  <td className="table-nowrap">{snapshot.recordedAt}</td>
                </tr>
              ))}
            </ResponsiveTable>
          </section>
        </>
      )}
    </section>
  );
}
