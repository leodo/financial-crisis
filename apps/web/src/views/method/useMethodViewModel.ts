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
import {
  currentMvpRiskState,
  mvpProbabilityInputIsAuditOnly
} from "../decision/mvpRiskState";
import { probabilityDiagnosticAnomalyHorizons } from "../decision/probabilityDiagnostics";
import {
  probabilityModelFinalSnapshotValue,
  probabilityRuntimeReferenceNote,
  probabilitySnapshotValue
} from "../decision/signalLayerBuilders";
import { methodContent } from "./content";

function methodUserFacingCopy(text: string) {
  return humanizeMethodNote(text)
    .replaceAll("formal history 审计的正式证据层", "正式历史证据层")
    .replaceAll("formal history 审计证据", "正式历史证据")
    .replaceAll("formal history 审计", "正式历史证据复核")
    .replaceAll("滚动审计", "滚动历史复核")
    .replaceAll("replay 审计", "replay 复核")
    .replaceAll("审计元数据", "训练覆盖元数据")
    .replaceAll("审计", "复核");
}

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
  const probabilityAnomalyHorizons = probabilityDiagnosticAnomalyHorizons(assessment);
  const probabilityAuditOnly = mvpProbabilityInputIsAuditOnly(assessment);
  const mvpRiskState = currentMvpRiskState(assessment);
  const runtimeReferenceNote = probabilityRuntimeReferenceNote(assessment);
  const modelFinalSnapshot = probabilityModelFinalSnapshotValue(assessment);

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
      hint: probabilityAuditOnly
        ? "正式概率包已加载，但当前仅作参考输入；主结论仍看 MVP 规则层。"
        : heuristicMode
          ? "当前仍是启发式过渡层。"
          : "当前已经切到正式概率包。"
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
      value: pointInTimeModeLabel(assessment.method.point_in_time_mode),
      hint: "这是历史特征可见性的构建口径，不是数据新鲜度或结论可信度。"
    },
    {
      label: "运行状态",
      value: releaseServingStatusLabel(assessment.method.release_status),
      hint: probabilityAuditOnly
        ? "这里只说明服务和 bundle 可加载，不代表正式概率已恢复为当前主结论。"
        : "这是服务状态，不等同于模型结论可信度。"
    }
  ];

  const priorActionRows: Array<[string, string]> = [
    [
      probabilityAuditOnly ? "危机先验（参考）" : "危机先验",
      probabilityAuditOnly
        ? `当前页面参考值 ${probabilitySnapshotValue(assessment.probabilities)}。${
            runtimeReferenceNote ? `${runtimeReferenceNote} ` : ""
          }${
            probabilityAnomalyHorizons.length > 0
              ? `命中 ${probabilityAnomalyHorizons.join(" / ")} 模型语义异常`
              : "已被后端 MVP 状态降为参考输入"
          }，只作为参考证据；当前不要把这组三期限直接理解成风险时距，主结论看 MVP 风险状态 ${mvpRiskState.label}。`
        : runtimeReferenceNote
          ? `当前页面值 ${probabilitySnapshotValue(
              assessment.probabilities
            )}；模型原始输出 ${modelFinalSnapshot}。当前回答“风险窗口离现在有多近”时，应优先按模型原始输出和异常诊断解释，不把运行口径参考值直接当成正式结论。`
          : `当前是 ${formatProbabilityPercentExact(assessment.probabilities.p_5d)} / ${formatProbabilityPercentExact(assessment.probabilities.p_20d)} / ${formatProbabilityPercentExact(assessment.probabilities.p_60d)}，回答“风险窗口离现在有多近”。`
    ],
    [
      probabilityAuditOnly || !assessment.method.actionability_enabled
        ? "动作信号（辅助）"
        : "动作概率",
      probabilityAuditOnly
        ? `当前显示 ${formatMethodActionProbability(assessment.actionability.prepare)} / ${formatMethodActionProbability(assessment.actionability.hedge)} / ${formatMethodActionProbability(assessment.actionability.defend)}，它回答“现在该不该准备、对冲或防守”，但在参考态下仍要让位于 MVP 规则层主结论。`
        : !assessment.method.actionability_enabled
          ? `当前显示 ${formatMethodActionProbability(assessment.actionability.prepare)} / ${formatMethodActionProbability(assessment.actionability.hedge)} / ${formatMethodActionProbability(assessment.actionability.defend)}，但这一层仍由危机先验和评分层过渡映射而来，只适合作为辅助执行信号，不应当成正式校准后的独立动作概率。`
          : `当前是 ${formatMethodActionProbability(assessment.actionability.prepare)} / ${formatMethodActionProbability(assessment.actionability.hedge)} / ${formatMethodActionProbability(assessment.actionability.defend)}，回答“现在该不该准备、对冲或防守”；它和 60d / 20d / 5d 的危机先验不是一一对应关系。`
    ],
    [
      "最终执行节奏",
      probabilityAuditOnly
        ? `当前正式概率只作参考输入，页面主结论改用 MVP 风险状态：${mvpRiskState.label}。${mvpRiskState.summary}`
        : `当前执行节奏为 ${postureLabel(assessment.posture)}，它是把危机先验、动作层、数据可信度和事件确认压缩后的执行结论。`
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
      hint: methodUserFacingCopy(historyProvenance.note)
    },
    {
      label: "历史轨迹点数",
      value: `${historyProvenance.total_points}`,
      hint: "默认历史窗口的回放点数，不是训练样本数量，也不是 Go/No-Go 通过次数。"
    },
    {
      label: "PIT 快照支撑",
      value: `${historyProvenance.feature_backed_points}/${historyProvenance.total_points || 0}`,
      hint:
        historyProvenance.latest_feature_backed_date !== null
          ? `最近一条当天 PIT 快照支撑点日期：${formatDate(historyProvenance.latest_feature_backed_date)}；这是历史证据层，不代表当前正式概率可作主结论。`
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
      note: methodUserFacingCopy(source.note),
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
      record.blocking_gaps.length > 0
        ? record.blocking_gaps.map(methodUserFacingCopy).join("；")
        : "当前没有额外阻断缺口。"
  }));

  const limitations = [
    methodContent.runtimeBoundarySummary,
    methodUserFacingCopy(historyProvenance.note),
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
    scenarioCoverageCatalogNote: methodUserFacingCopy(method.scenario_data_coverage_catalog.note),
    historyProvenanceMetrics,
    historyProvenanceRows,
    historyProvenanceNote: methodUserFacingCopy(historyProvenance.note),
    historyProvenanceReplayRunId: historyProvenance.latest_replay_run_id,
    limitations,
    historyPolicyVersion,
    protectedCatalogId,
    protectedCatalogSource,
    protectedCatalogNote: methodUserFacingCopy(method.protected_stress_window_catalog.note)
  };
}
