import {
  compactFileReference,
  compactTechnicalId,
  describePostureClause,
  formatDate,
  formatPercent,
  formatProbabilityPercentExact,
  formatProbabilityPercent,
  historyEvidenceTierLabel,
  historySourceLabel,
  humanizeMethodNote,
  methodVersionFieldLabel,
  pointInTimeModeLabel,
  postureLabel,
  probabilityModeLabel,
  releaseIdLabel,
  releaseServingStatusLabel,
  runtimeThresholdLabel
} from "../../format";
import type {
  AssessmentMethodResponse,
  AssessmentSnapshot,
  PostureGuidance
} from "../../types";
import type { DetailRowItem, MetricItem, VersionRowItem } from "../shared/panelHelpers";
import { buildProbabilityOverlayViewModel } from "../shared/probabilityOverlay";
import { methodContent } from "./content";

export function useMethodViewModel({
  assessment,
  posture,
  method
}: {
  assessment: AssessmentSnapshot;
  posture: PostureGuidance;
  method: AssessmentMethodResponse;
}) {
  const formatMethodActionProbability = (value: number) =>
    value === 0 && !assessment.method.actionability_enabled
      ? "未触发"
      : formatProbabilityPercent(value);

  const heuristicMode = assessment.method.probability_mode === "heuristic_mvp";
  const degradedRelease = assessment.method.release_status === "degraded";
  const compactReleaseId = releaseIdLabel(assessment.method.release_id);
  const historyPolicyVersion = compactTechnicalId(
    method.runtime_thresholds.history_runtime_policy_version
  );
  const protectedCatalogId = compactTechnicalId(method.protected_stress_window_catalog.catalog_id);
  const protectedCatalogSource = compactFileReference(method.protected_stress_window_catalog.source);
  const scenarioCoverageCatalogId = compactTechnicalId(
    method.scenario_data_coverage_catalog.catalog_id
  );
  const scenarioCoverageCatalogSource = compactFileReference(
    method.scenario_data_coverage_catalog.source
  );
  const actionModelVersion = assessment.method.actionability_model_version
    ? compactTechnicalId(assessment.method.actionability_model_version)
    : null;

  const buildVersionRow = (label: string, rawValue: string): VersionRowItem => {
    const compact = compactTechnicalId(rawValue);
    return {
      label,
      value: compact.value,
      hint: compact.hint,
      valueClassName: compact.hint ? "metric-value-token" : undefined
    };
  };

  const versionRows: VersionRowItem[] = [
    buildVersionRow(methodVersionFieldLabel("score"), assessment.method.score_method_version),
    buildVersionRow(methodVersionFieldLabel("prob"), assessment.method.prob_model_version),
    buildVersionRow(
      methodVersionFieldLabel("calibration"),
      assessment.method.calibration_version
    ),
    buildVersionRow(methodVersionFieldLabel("feature"), assessment.method.feature_set_version),
    buildVersionRow(methodVersionFieldLabel("label"), assessment.method.label_version),
    buildVersionRow(
      methodVersionFieldLabel("posture"),
      assessment.method.posture_policy_version
    ),
    buildVersionRow(
      methodVersionFieldLabel("playbook"),
      assessment.method.action_playbook_version
    ),
    {
      label: methodVersionFieldLabel("prob mode"),
      value: probabilityModeLabel(assessment.method.probability_mode),
      hint: assessment.method.probability_mode
    },
    {
      label: methodVersionFieldLabel("release"),
      value: releaseServingStatusLabel(assessment.method.release_status),
      hint: assessment.method.release_status
    },
    {
      label: methodVersionFieldLabel("release id"),
      value: compactReleaseId.value,
      hint: compactReleaseId.hint,
      valueClassName: compactReleaseId.hint ? "metric-value-token" : undefined
    },
    {
      label: methodVersionFieldLabel("pit mode"),
      value: pointInTimeModeLabel(assessment.method.point_in_time_mode),
      hint: assessment.method.point_in_time_mode
    }
  ];

  const headlineMetrics: MetricItem[] = [
    {
      label: "概率模式",
      value: probabilityModeLabel(assessment.method.probability_mode),
      hint: heuristicMode ? "当前仍是启发式过渡层。" : "当前已经切到正式概率包。"
    },
    {
      label: "动作层",
      value: assessment.method.actionability_enabled ? "独立动作模型" : "过渡动作映射",
      hint: assessment.method.actionability_enabled
        ? actionModelVersion?.value ?? "已启用"
        : "动作概率仍有一部分来自危机先验和评分层映射。"
    },
    {
      label: "点位可见性",
      value: pointInTimeModeLabel(assessment.method.point_in_time_mode)
    },
    {
      label: "运行状态",
      value: releaseServingStatusLabel(assessment.method.release_status)
    }
  ];

  const priorActionRows: Array<[string, string]> = [
    [
      "危机先验",
      `当前是 ${formatProbabilityPercentExact(assessment.probabilities.p_5d)} / ${formatProbabilityPercentExact(assessment.probabilities.p_20d)} / ${formatProbabilityPercentExact(assessment.probabilities.p_60d)}，回答“风险窗口离现在有多近”。`
    ],
    [
      "动作概率",
      `当前是 ${formatMethodActionProbability(assessment.actionability.prepare)} / ${formatMethodActionProbability(assessment.actionability.hedge)} / ${formatMethodActionProbability(assessment.actionability.defend)}，回答“现在该不该准备、对冲或防守”；它和 60d / 20d / 5d 的危机先验不是一一对应关系。`
    ],
    [
      "最终执行节奏",
      `当前执行节奏为 ${postureLabel(assessment.posture)}，它是把危机先验、动作层、数据可信度和事件确认压缩后的执行结论。`
    ],
    [
      "动作头状态",
      assessment.method.actionability_enabled
        ? "当前生效版本已经启用独立动作模型，动作概率不再只是从危机先验直接映射过来。"
        : "当前生效版本还没有独立动作模型，页面里的动作概率仍有一部分来自危机先验和评分层的过渡映射。"
    ]
  ];

  const runtimeMetrics = [
    [runtimeThresholdLabel("prepare floor"), formatPercent(method.runtime_thresholds.prepare_p60d)],
    [runtimeThresholdLabel("hedge floor"), formatPercent(method.runtime_thresholds.hedge_p20d)],
    [runtimeThresholdLabel("defend floor"), formatPercent(method.runtime_thresholds.defend_p5d)],
    [runtimeThresholdLabel("weeks bridge"), formatPercent(method.runtime_thresholds.elevated_weeks_p60d)],
    [runtimeThresholdLabel("external bridge"), formatPercent(method.runtime_thresholds.external_prepare_p20d)],
    [runtimeThresholdLabel("carry bridge"), formatPercent(method.runtime_thresholds.carry_prepare_p60d)]
  ] as Array<[string, string]>;

  const triggerClauses = posture.trigger_codes.map((code) => describePostureClause(code));
  const blockerClauses = posture.blocker_codes.map((code) => describePostureClause(code));
  const { overlayHeadlineMetrics, overlayHorizonRows, overlayAuditRows } =
    buildProbabilityOverlayViewModel(assessment);
  const historyProvenance = method.history_provenance;
  const historyProvenanceMetrics: MetricItem[] = [
    {
      label: "证据等级",
      value: historyEvidenceTierLabel(historyProvenance.evidence_tier),
      hint: historyProvenance.note
    },
    {
      label: "历史轨迹点数",
      value: `${historyProvenance.total_points}`
    },
    {
      label: "PIT 快照支撑",
      value: `${historyProvenance.feature_backed_points}/${historyProvenance.total_points || 0}`,
      hint:
        historyProvenance.latest_feature_backed_date !== null
          ? `最近一条当天 PIT 快照支撑点日期：${formatDate(historyProvenance.latest_feature_backed_date)}`
          : "当前默认历史窗口里还没有 PIT 快照支撑点。"
    },
    {
      label: "沿用旧 PIT",
      value: `${historyProvenance.reused_feature_snapshot_points}`,
      hint:
        historyProvenance.latest_reused_feature_snapshot_date !== null
          ? `最近一条沿用旧 PIT 的点日期：${formatDate(historyProvenance.latest_reused_feature_snapshot_date)}`
          : "当前默认历史窗口里没有沿用旧 PIT 的点。"
    },
    {
      label: "旧快照桥接",
      value: `${historyProvenance.snapshot_bridge_points}`,
      hint:
        historyProvenance.latest_snapshot_bridge_date !== null
          ? `最近一条 bridge 点日期：${formatDate(historyProvenance.latest_snapshot_bridge_date)}`
          : "当前默认历史窗口里没有 bridge 点。"
    }
  ];
  const historyProvenanceRows: DetailRowItem[] = historyProvenance.sources
    .filter((source) => source.count > 0)
    .map((source) => ({
      id: source.source_id,
      title: historySourceLabel(source.source_id),
      detail:
        source.latest_as_of_date !== null
          ? `共 ${source.count} 个点，最近日期 ${formatDate(source.latest_as_of_date)}`
          : `共 ${source.count} 个点`,
      note: source.note,
      meta: `${source.count}`
    }));
  const scenarioCoverageRecords = method.scenario_data_coverage_catalog.records;
  const scenarioCoverageMetrics: MetricItem[] = [
    {
      label: "正式主训练",
      value: `${scenarioCoverageRecords.filter((record) => record.usable_for_main_training).length}`
    },
    {
      label: "扩展训练",
      value: `${scenarioCoverageRecords.filter((record) => record.usable_for_extension_training).length}`
    },
    {
      label: "受保护压力",
      value: `${scenarioCoverageRecords.filter((record) => record.usable_for_protected_stress).length}`
    },
    {
      label: "历史类比",
      value: `${scenarioCoverageRecords.filter((record) => record.usable_for_historical_analog).length}`
    }
  ];
  const scenarioCoverageRows = scenarioCoverageRecords.map((record) => ({
    id: record.scenario_id,
    scenarioLabel: record.scenario_label,
    scenarioId: record.scenario_id,
    roleSummary: record.recommended_role,
    gradeSummary: `${record.coverage_grade} / ${record.point_in_time_mode}`,
    sourceSummary: record.free_sources.join("、"),
    statusSummary: record.current_status,
    gapSummary:
      record.blocking_gaps.length > 0 ? record.blocking_gaps.join("；") : "当前没有额外阻断缺口。"
  }));

  const limitations = [
    methodContent.runtimeBoundarySummary,
    historyProvenance.note,
    heuristicMode
      ? `当前概率模式是 ${probabilityModeLabel(assessment.method.probability_mode)}，${methodContent.limitationModeHeuristic}`
      : `当前概率模式是 ${probabilityModeLabel(assessment.method.probability_mode)}，${methodContent.limitationModeFormal}`,
    degradedRelease
      ? `当前运行状态是 ${releaseServingStatusLabel(assessment.method.release_status)}，${methodContent.limitationReleaseDegraded}`
      : `当前运行状态是 ${releaseServingStatusLabel(assessment.method.release_status)}，${methodContent.limitationReleaseHealthy}`,
    posture.summary
  ];

  return {
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
    scenarioCoverageCatalogNote: humanizeMethodNote(method.scenario_data_coverage_catalog.note),
    historyProvenanceMetrics,
    historyProvenanceRows,
    historyProvenanceNote: historyProvenance.note,
    historyProvenanceReplayRunId: historyProvenance.latest_replay_run_id,
    limitations,
    historyPolicyVersion,
    protectedCatalogId,
    protectedCatalogSource,
    protectedCatalogNote: humanizeMethodNote(method.protected_stress_window_catalog.note)
  };
}
