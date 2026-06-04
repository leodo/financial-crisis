import { Activity, Database, History, ShieldCheck } from "lucide-react";
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
    methodSummary,
    overlayHeadlineMetrics,
    overlayHorizonRows,
    overlayAuditRows,
    overlaySummary,
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
