import {
  compactFileReference,
  describePostureClause,
  formatDate,
  formatDateTime,
  formatPercent,
  freshnessLabel,
  historyEvidenceTierLabel,
  historySourceLabel,
  humanizeAuditNote,
  pointInTimeModeLabel,
  postureLabel,
  probabilityModeLabel,
  releaseReviewActionTypeLabel,
  releaseReviewAttributionLabel,
  releaseReviewScenarioCoveragePitLabel,
  releaseReviewScenarioFamilyLabel,
  releaseReviewHistoryModeLabel,
  releaseReviewScenarioRoleLabel,
  releaseReviewScenarioTrainingRoleLabel,
  releaseReviewVerdictLabel,
  releaseReviewWorkstreamLabel,
  releaseManifestStatusLabel,
  releaseIdLabel,
  releaseServingStatusLabel,
  scenarioPackBlockerLabel,
  scenarioPackOutcomeLabel,
  timeBucketLabel
} from "../../format";
import type {
  AssessmentSnapshot,
  DecisionPosture,
  FreshnessStatus,
  ResearchAuditResponse,
  TimeToRiskBucket
} from "../../types";
import type { DetailRowItem, MetricItem } from "../shared/panelHelpers";
import { buildProbabilityOverlayViewModel } from "../shared/probabilityOverlay";
import { auditContent } from "./content";

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
      label: "回放批次",
      value: `${audit.replay_runs.length}`,
      hint: audit.latest_replay_run_id ? `最新 run: ${audit.latest_replay_run_id}` : "当前没有可展示的 replay run"
    },
    {
      label: "快照覆盖",
      value: `${uniqueSnapshotDates} 天`,
      hint: `${audit.snapshots.length} 条历史预测记录`
    }
  ];
  const provenanceMetrics: MetricItem[] = [
    {
      label: "历史证据等级",
      value: historyEvidenceTierLabel(audit.history_provenance.evidence_tier),
      hint: audit.history_provenance.note
    },
    {
      label: "PIT 快照支撑",
      value: `${audit.history_provenance.feature_backed_points}/${audit.history_provenance.total_points}`
    },
    {
      label: "沿用旧 PIT",
      value: `${audit.history_provenance.reused_feature_snapshot_points}`
    },
    {
      label: "原始观测过渡",
      value: `${audit.history_provenance.raw_observation_points}`
    },
    {
      label: "旧快照桥接",
      value: `${audit.history_provenance.snapshot_bridge_points}`
    }
  ];
  const provenanceRows: DetailRowItem[] = audit.history_provenance.sources
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

  const methodSummary = `当前运行的是 ${probabilityModeLabel(assessment.method.probability_mode)}，服务状态 ${releaseServingStatusLabel(assessment.method.release_status)}，对应版本 ${releaseIdLabel(assessment.method.release_id).value}。`;
  const {
    overlayHeadlineMetrics,
    overlayHorizonRows,
    overlayAuditRows,
    configuredOverlayCount,
    activeContributionCount
  } = buildProbabilityOverlayViewModel(assessment);
  const overlaySummary =
    configuredOverlayCount > 0
      ? `当前 active release 已挂载 ${configuredOverlayCount} 个 overlay，其中本次快照实际参与 ${activeContributionCount} 个。`
      : "当前 active release 还没有挂载真正参与 runtime 的 overlay head。";

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

  const latestReleaseReview = audit.latest_release_review;
  const latestReleaseReviewMetrics: MetricItem[] = latestReleaseReview
    ? [
        {
          label: "评审时间",
          value: formatDateTime(latestReleaseReview.reviewed_at)
        },
        {
          label: "Guard 结论",
          value: releaseReviewVerdictLabel(latestReleaseReview.overall_guard_passed),
          hint: "这里只代表离线 release review 的 guard 结论，不等于可以自动上线。",
          valueClassName: "metric-value-token"
        },
        {
          label: "Baseline",
          value: releaseIdLabel(latestReleaseReview.baseline_release_id).value,
          hint: releaseIdLabel(latestReleaseReview.baseline_release_id).hint,
          valueClassName: "metric-value-token"
        },
        {
          label: "Candidate",
          value: releaseIdLabel(latestReleaseReview.candidate_release_id).value,
          hint: releaseIdLabel(latestReleaseReview.candidate_release_id).hint,
          valueClassName: "metric-value-token"
        },
        {
          label: "历史模式",
          value: releaseReviewHistoryModeLabel(latestReleaseReview.history_mode)
        },
        {
          label: "动作条目",
          value: `${latestReleaseReview.historical_audit_actions.length}`,
          hint: `${latestReleaseReview.historical_audit_attribution.length} 个工作流归因已落库`
        }
      ]
    : [];

  const latestReleaseReviewContextRows = latestReleaseReview
    ? [
        {
          id: "active-release",
          title: "当前线上 active release",
          detail: activeRelease.value,
          note: activeRelease.hint
        },
        {
          id: "review-release-link",
          title: "这次 review 对比",
          detail: `${releaseIdLabel(latestReleaseReview.baseline_release_id).value} vs ${releaseIdLabel(latestReleaseReview.candidate_release_id).value}`,
          note:
            latestReleaseReview.original_active_release_id === latestReleaseReview.baseline_release_id
              ? "原始 active release 与 baseline 一致。"
              : `原始 active release 为 ${releaseIdLabel(latestReleaseReview.original_active_release_id).value}`
        },
        {
          id: "review-restore-link",
          title: "运行态恢复版本",
          detail: releaseIdLabel(latestReleaseReview.restored_release_id).value,
          note: "若 review 过程切换过运行态，这里显示最终恢复到的 release。"
        }
      ]
    : [];

  const latestReleaseReviewCoverageSource = latestReleaseReview
    ? compactFileReference(latestReleaseReview.scenario_coverage_catalog.source)
    : null;

  const latestReleaseReviewCoverageMetrics: MetricItem[] =
    latestReleaseReview && latestReleaseReview.scenario_coverages.length > 0
      ? [
          {
            label: "覆盖目录",
            value: latestReleaseReview.scenario_coverage_catalog.catalog_id || "未登记",
            hint: latestReleaseReviewCoverageSource?.hint
          },
          {
            label: "回测覆盖",
            value: `${latestReleaseReview.scenario_coverage_catalog.covered_backtest_scenario_count}/${latestReleaseReview.scenario_coverage_catalog.backtest_scenario_count}`
          },
          {
            label: "重点覆盖",
            value: `${latestReleaseReview.scenario_coverage_catalog.covered_focus_scenario_count}/${latestReleaseReview.scenario_coverage_catalog.focus_scenario_count}`
          },
          {
            label: "主训练可用",
            value: `${latestReleaseReview.scenario_coverage_catalog.main_training_eligible_count}`
          },
          {
            label: "扩展可用",
            value: `${latestReleaseReview.scenario_coverage_catalog.extension_training_eligible_count}`
          },
          {
            label: "受保护压力",
            value: `${latestReleaseReview.scenario_coverage_catalog.protected_stress_eligible_count}`,
            hint: `历史类比可用 ${latestReleaseReview.scenario_coverage_catalog.historical_analog_eligible_count} 个`
          }
        ]
      : [];

  const latestReleaseReviewActionRows =
    latestReleaseReview?.historical_audit_actions.map((row, index) => ({
      id: `${row.workstream}-${row.action_type}-${index}`,
      workstream: releaseReviewWorkstreamLabel(row.workstream),
      attribution: releaseReviewAttributionLabel(row.attribution),
      actionType: releaseReviewActionTypeLabel(row.action_type),
      scenarioSummary: `${row.scenario_count} 个场景 / ${row.protected_count} 个 protected window`,
      recommendation: row.recommendation
    })) ?? [];

  const latestReleaseReviewAttributionRows =
    latestReleaseReview?.historical_audit_attribution.map((row, index) => ({
      id: `${row.workstream}-${row.attribution}-${index}`,
      workstream: releaseReviewWorkstreamLabel(row.workstream),
      attribution: releaseReviewAttributionLabel(row.attribution),
      matchSummary: `baseline ${row.baseline_count} / candidate ${row.candidate_count}`,
      scenarioSummary: `${row.scenario_count} 个场景 / ${row.protected_count} 个 protected window`,
      explanation: row.explanation,
      scenarioDetail: [
        row.baseline_scenarios.length > 0 ? `Baseline: ${row.baseline_scenarios.join(" / ")}` : null,
        row.candidate_scenarios.length > 0 ? `Candidate: ${row.candidate_scenarios.join(" / ")}` : null
      ]
    })) ?? [];

  const latestReleaseReviewCoverageRows =
    latestReleaseReview?.scenario_coverages
      .slice()
      .sort((left, right) => {
        return (
          Number(right.in_focus_review) - Number(left.in_focus_review) ||
          Number(right.in_backtest_comparison) - Number(left.in_backtest_comparison) ||
          Number(right.usable_for_main_training) - Number(left.usable_for_main_training) ||
          left.scenario_name.localeCompare(right.scenario_name, "zh-CN")
        );
      })
      .map((row) => {
        const allowedRoles = [
          row.usable_for_main_training ? releaseReviewScenarioRoleLabel("main_training") : null,
          row.usable_for_extension_training ? releaseReviewScenarioRoleLabel("extension_training") : null,
          row.usable_for_protected_stress ? releaseReviewScenarioRoleLabel("protected_stress") : null,
          row.usable_for_historical_analog ? releaseReviewScenarioRoleLabel("historical_analog_only") : null
        ].filter((item): item is string => item !== null);
        const scenarioTags = [
          row.in_focus_review ? "重点复核" : null,
          row.in_backtest_comparison ? "回测对比" : null,
          row.protected_window ? "受保护窗口" : null
        ].filter((item): item is string => item !== null);

        return {
          id: row.scenario_id,
          scenarioLabel: row.scenario_name,
          scenarioDetails: [
            row.scenario_id,
            scenarioTags.length > 0 ? scenarioTags.join(" / ") : null
          ].filter((item): item is string => item !== null),
          familySummary: releaseReviewScenarioFamilyLabel(row.scenario_family),
          trainingRoleSummary: releaseReviewScenarioTrainingRoleLabel(row.training_role),
          coverageRoleSummary: releaseReviewScenarioRoleLabel(row.recommended_role),
          allowedSummary:
            allowedRoles.length > 0
              ? `可用: ${allowedRoles.join("、")}`
              : "当前没有可用目录角色。",
          gradeSummary: `${row.coverage_grade} / ${releaseReviewScenarioCoveragePitLabel(row.point_in_time_mode)}`,
          sourceSummary: row.free_sources.length > 0 ? row.free_sources.join("、") : "未登记免费主源",
          statusSummary: row.current_status,
          gapSummary:
            row.blocking_gaps.length > 0
              ? row.blocking_gaps.join("；")
              : "当前没有额外阻断缺口。"
        };
      }) ?? [];

  const latestScenarioPackAudit = audit.latest_scenario_pack_audit;
  const latestScenarioPackAuditSource = latestScenarioPackAudit
    ? compactFileReference(latestScenarioPackAudit.source)
    : null;
  const scenarioPackBlockerCount = (key: string) =>
    latestScenarioPackAudit?.blocker_counts.find((row) => row.key === key)?.count ?? 0;
  const latestScenarioPackAuditMetrics: MetricItem[] = latestScenarioPackAudit
    ? [
        {
          label: "场景 compare 覆盖",
          value: `${latestScenarioPackAudit.compare_ok_count}/${latestScenarioPackAudit.scenario_summaries.length}`
        },
        {
          label: "稳定通过",
          value: `${scenarioPackBlockerCount("stable_pass")}`
        },
        {
          label: "通过但边际变弱",
          value: `${scenarioPackBlockerCount("stable_pass_with_margin_erosion")}`
        },
        {
          label: "共享漏报",
          value: `${scenarioPackBlockerCount("shared_missed_signal")}`
        },
        {
          label: "共享无信号",
          value: `${scenarioPackBlockerCount("shared_no_signal")}`
        },
        {
          label: "执行连续性问题",
          value: `${scenarioPackBlockerCount("posture_continuity")}`
        }
      ]
    : [];

  const blockerPriority: Record<string, number> = {
    candidate_regression: 0,
    posture_continuity: 1,
    review_gate_gap: 2,
    residual_review_l3: 3,
    stable_pass_with_margin_erosion: 4,
    shared_missed_signal: 5,
    shared_no_signal: 6,
    stable_pass: 7,
    candidate_improvement: 8
  };

  const latestScenarioPackAuditRows =
    latestScenarioPackAudit?.scenario_summaries
      .slice()
      .sort((left, right) => {
        return (
          (blockerPriority[left.blocker_class] ?? 99) - (blockerPriority[right.blocker_class] ?? 99) ||
          left.scenario_label.localeCompare(right.scenario_label, "zh-CN")
        );
      })
      .map((row) => {
        const timingSummary =
          row.candidate_actionable_lead_time_days !== null
            ? `候选动作提前 ${row.candidate_actionable_lead_time_days} 天`
            : row.candidate_lead_time_days !== null
              ? `候选 L2 提前 ${row.candidate_lead_time_days} 天`
              : "当前没有有效 lead time";
        const timingDetails = [
          scenarioPackOutcomeLabel(row.outcome),
          row.positive_window_retention_20d !== null
            ? `20d 连续命中保留 ${formatPercent(row.positive_window_retention_20d)}`
            : null,
          row.overall_avg_delta_p_20d !== null
            ? `20d 均值变化 ${formatPercent(row.overall_avg_delta_p_20d)}`
            : null
        ].filter((item): item is string => item !== null);
        const scenarioTags = [
          releaseReviewScenarioFamilyLabel(row.family),
          releaseReviewScenarioTrainingRoleLabel(row.training_role),
          row.protected_window ? "受保护窗口" : null
        ].filter((item): item is string => item !== null);
        const coverageDetails = [
          row.current_status,
          row.compare_dataset_key
            ? `Dataset: ${row.compare_dataset_key}`
            : row.attempted_datasets.length > 0
              ? `Tried: ${row.attempted_datasets.join(" / ")}`
              : null,
          row.free_sources.length > 0 ? `免费主源: ${row.free_sources.join("、")}` : null
        ].filter((item): item is string => item !== null);
        const blockerDetails = [
          row.primary_workstream ? releaseReviewWorkstreamLabel(row.primary_workstream) : null,
          row.candidate_primary_failure_mode ?? null,
          row.suggested_review ?? null
        ].filter((item): item is string => item !== null);

        return {
          id: row.scenario_id,
          scenarioLabel: row.scenario_label,
          scenarioDetails: [row.scenario_id, ...scenarioTags],
          blockerSummary: scenarioPackBlockerLabel(row.blocker_class),
          blockerDetails,
          timingSummary,
          timingDetails,
          coverageSummary: `${row.coverage_grade} / ${releaseReviewScenarioCoveragePitLabel(row.point_in_time_mode)}`,
          coverageDetails,
          takeaway: row.takeaway,
          gapSummary:
            row.blocking_gaps.length > 0 ? row.blocking_gaps.join("；") : "当前没有额外缺口。"
        };
      }) ?? [];

  return {
    auditNote: audit.note ? humanizeAuditNote(audit.note) : auditContent.noteSummary,
    runtimeMetrics,
    summaryMetrics,
    provenanceMetrics,
    provenanceRows,
    provenanceNote: audit.history_provenance.note,
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
    releaseRows,
    snapshotRows
  };
}
