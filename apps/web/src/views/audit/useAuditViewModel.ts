import {
  compactFileReference,
  describePostureClause,
  formatDate,
  formatDateTime,
  formatPercent,
  freshnessLabel,
  pointInTimeModeLabel,
  postureLabel,
  probabilityModeLabel,
  releaseManifestStatusLabel,
  releaseIdLabel,
  releaseServingStatusLabel,
  timeBucketLabel
} from "../../format";
import type {
  AssessmentSnapshot,
  DecisionPosture,
  FreshnessStatus,
  ResearchAuditResponse,
  TimeToRiskBucket
} from "../../types";
import type { MetricItem } from "../shared/panelHelpers";
import { auditContent } from "./content";

function humanizeAuditNote(note: string) {
  return note
    .replaceAll("release registry", "版本登记册")
    .replaceAll("historical replay run / point", "历史回放结果")
    .replaceAll("prediction snapshot", "预测快照")
    .replaceAll("runtime probability mode", "运行中的概率层")
    .replaceAll("release manifest", "版本登记状态")
    .replaceAll("heuristic", "启发式过渡层");
}

export function useAuditViewModel({
  assessment,
  audit
}: {
  assessment: AssessmentSnapshot;
  audit: ResearchAuditResponse;
}) {
  const activeRelease = releaseIdLabel(audit.active_release_id);
  const activeLikeStatuses = new Set(["active", "approved"]);
  const inactiveStatuses = new Set(["archived", "rolled_back", "retired"]);
  const uniqueSnapshotDates = new Set(audit.snapshots.map((snapshot) => snapshot.as_of_date)).size;

  const runtimeMetrics: MetricItem[] = [
    {
      label: "概率层",
      value: probabilityModeLabel(audit.runtime_probability_mode),
      valueClassName: "metric-value-token"
    },
    {
      label: "服务状态",
      value: releaseServingStatusLabel(audit.runtime_release_status),
      valueClassName: "metric-value-token"
    },
    {
      label: "当前生效版本",
      value: activeRelease.value,
      valueClassName: "metric-value-token"
    },
    { label: "最新快照", value: formatDate(audit.latest_snapshot_date) }
  ];

  const summaryMetrics: MetricItem[] = [
    { label: "登记版本数", value: `${audit.releases.length}` },
    {
      label: "当前 / 已批准",
      value: `${audit.releases.filter((release) => activeLikeStatuses.has(release.status)).length}`,
      hint: "仍属于当前可运行版本或已批准候选。"
    },
    {
      label: "已归档 / 回退",
      value: `${audit.releases.filter((release) => inactiveStatuses.has(release.status)).length}`,
      hint: "这些版本已退出当前候选集合。"
    },
    {
      label: "快照覆盖",
      value: `${uniqueSnapshotDates} 天`,
      hint: `${audit.snapshots.length} 条历史预测记录`
    }
  ];

  const methodSummary = `当前运行的是 ${probabilityModeLabel(assessment.method.probability_mode)}，服务状态 ${releaseServingStatusLabel(assessment.method.release_status)}，对应版本 ${releaseIdLabel(assessment.method.release_id).value}。`;

  const releaseRows = audit.releases.map((release) => {
    const compact = releaseIdLabel(release.release_id);
    const bundleReference = compactFileReference(release.bundle_uri);
    return {
      id: release.release_id,
      releaseId: compact.value,
      bundleUri: "模型包文件已登记",
      bundleUriHint: bundleReference.hint,
      status: releaseManifestStatusLabel(release.status),
      pointInTimeMode: pointInTimeModeLabel(release.point_in_time_mode),
      probabilityMode: probabilityModeLabel(release.probability_mode),
      servingStatus: releaseServingStatusLabel(release.serving_status),
      trainingRange: `${formatDate(release.training_range_start)} - ${formatDate(release.training_range_end)}`,
      evaluation: `概率误差 ${release.brier_score !== null ? release.brier_score.toFixed(4) : "—"}`,
      evaluationDetail: `损失 ${release.log_loss !== null ? release.log_loss.toFixed(4) : "—"} / 校准误差 ${release.ece !== null ? release.ece.toFixed(4) : "—"}`,
      createdAt: formatDateTime(release.created_at)
    };
  });

  const snapshotRows = audit.snapshots.map((snapshot) => {
    const compact = releaseIdLabel(snapshot.release_id);
    return {
      id: `${snapshot.as_of_date}-${snapshot.release_id ?? "inline"}-${snapshot.recorded_at}`,
      asOfDate: formatDate(snapshot.as_of_date),
      releaseId: snapshot.release_id ? compact.value : "内联快照",
      pointInTimeMode: pointInTimeModeLabel(snapshot.point_in_time_mode),
      probabilityMode: probabilityModeLabel(snapshot.probability_mode),
      releaseStatus: releaseServingStatusLabel(snapshot.release_status),
      calibratedSummary: `${formatPercent(snapshot.calibrated_p_5d)} / ${formatPercent(snapshot.calibrated_p_20d)} / ${formatPercent(snapshot.calibrated_p_60d)}`,
      rawSummary: `${formatPercent(snapshot.raw_p_5d)} / ${formatPercent(snapshot.raw_p_20d)} / ${formatPercent(snapshot.raw_p_60d)}`,
      posture: postureLabel(snapshot.posture as DecisionPosture),
      timeBucket: timeBucketLabel(snapshot.time_to_risk_bucket as TimeToRiskBucket),
      triggerLabels: snapshot.posture_trigger_codes.map((code) => describePostureClause(code).label),
      blockerLabels: snapshot.posture_blocker_codes.map((code) => describePostureClause(code).label),
      freshnessStatus: freshnessLabel(snapshot.freshness_status as FreshnessStatus),
      coverage: formatPercent(snapshot.coverage_score),
      recordedAt: formatDateTime(snapshot.recorded_at)
    };
  });

  return {
    auditNote: audit.note ? humanizeAuditNote(audit.note) : auditContent.noteSummary,
    runtimeMetrics,
    summaryMetrics,
    methodSummary,
    releaseRows,
    snapshotRows
  };
}
