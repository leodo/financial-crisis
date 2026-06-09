import { SearchCheck } from "lucide-react";
import {
  compactFileReference,
  formatDate,
  formatDateTime,
  formatProbabilityPercentExact,
  formatSignedNumber,
  releaseIdLabel
} from "../../format";
import type { ResearchAuditResponse } from "../../types";
import {
  DetailRows,
  MetricGrid,
  ResponsiveTable,
  RuleBox,
  StackedTableCell,
  SurfaceHeader
} from "../shared/panelHelpers";
import { auditContent } from "./content";

function gapClassLabel(value: string): string {
  return (
    {
      candidate_margin_erosion: "候选边际弱化",
      no_runtime_floor_signal: "运行阈值未形成",
      protected_context_signal_present: "已有受保护上下文",
      weak_signal_continuity: "弱信号连续性不足"
    }[value] ?? value
  );
}

function formatOptionalProbability(value: number | null): string {
  return value === null ? "—" : formatProbabilityPercentExact(value);
}

function formatProbabilityDelta(value: number | null): string {
  return value === null ? "—" : formatSignedNumber(value * 100, 2, "%");
}

function compactCounts(rows: string[], empty = "无"): string {
  return rows.length > 0 ? rows.join(" / ") : empty;
}

function nextActionLabel(value: string | null, fallbackReasons: string[]): string {
  if (!value && fallbackReasons.length === 0) {
    return "—";
  }
  return (
    {
      "Treat this as candidate margin erosion before using it as a new baseline.":
        "候选相对基线边际变弱；在把候选当新基线前，先复核候选训练、特征权重和 gate 变化。",
      "Keep as usable historical evidence and focus next on false positives and cross-scenario generalization.":
        "已有可用历史证据；下一步重点看误报治理和跨场景泛化，而不是继续归因到缺数据。",
      "Audit feature separation and family context before changing thresholds.":
        "先审计 feature separation 和 family context，再考虑改阈值。"
    }[value ?? ""] ?? value ?? fallbackReasons.join("；")
  );
}

export function PrewarningGapAuditSection({ audit }: { audit: ResearchAuditResponse }) {
  const latestPrewarningGapAudit = audit.latest_prewarning_gap_audit;
  const source = latestPrewarningGapAudit
    ? compactFileReference(latestPrewarningGapAudit.source)
    : null;

  const totalRows =
    latestPrewarningGapAudit?.scenario_summaries.reduce(
      (sum, row) => sum + row.dataset_evidence.row_count,
      0
    ) ?? 0;
  const totalCandidate20dHits =
    latestPrewarningGapAudit?.scenario_summaries.reduce(
      (sum, row) => sum + row.probability_evidence.candidate_hit_20d.hit_count,
      0
    ) ?? 0;
  const trueRuntimeGapCount =
    latestPrewarningGapAudit?.scenario_summaries.filter(
      (row) => row.diagnosis.gap_class === "no_runtime_floor_signal"
    ).length ?? 0;

  const metrics = latestPrewarningGapAudit
    ? [
        {
          label: "审计时间",
          value: formatDateTime(latestPrewarningGapAudit.generated_at)
        },
        {
          label: "场景数",
          value: `${latestPrewarningGapAudit.scenario_count}`
        },
        {
          label: "数据行",
          value: `${totalRows}`
        },
        {
          label: "候选 20d 命中",
          value: `${totalCandidate20dHits}`
        },
        {
          label: "真缺运行信号",
          value: `${trueRuntimeGapCount}`
        }
      ]
    : [];

  const contextRows = latestPrewarningGapAudit
    ? [
        {
          id: "prewarning-gap-releases",
          title: "基线 / 候选",
          detail: `${releaseIdLabel(latestPrewarningGapAudit.baseline_release_id).value} vs ${releaseIdLabel(latestPrewarningGapAudit.candidate_release_id).value}`,
          note: latestPrewarningGapAudit.market_scope
        },
        {
          id: "prewarning-gap-counts",
          title: "缺口分类",
          detail: latestPrewarningGapAudit.gap_counts
            .map((item) => {
              const [key, count] = item.split("=");
              return `${gapClassLabel(key)} ${count ?? ""}`.trim();
            })
            .join(" / "),
          note: "不是所有场景都等于没有数据或没有预警"
        }
      ]
    : [];

  const scenarioRows =
    latestPrewarningGapAudit?.scenario_summaries.map((row) => {
      const dataset = row.dataset_evidence;
      const probability = row.probability_evidence;
      return {
        id: row.scenario_id,
        scenarioLabel: row.scenario_label,
        scenarioDetails: [
          row.scenario_id,
          gapClassLabel(row.diagnosis.gap_class),
          `${formatDate(row.pre_warning_start)} - ${formatDate(row.crisis_end)}`
        ].join(" · "),
        datasetSummary: `${dataset.row_count} 行 · ${row.coverage_grade} · ${row.coverage_pit_mode}`,
        datasetDetails: [
          dataset.dataset_key ?? "未绑定 dataset",
          compactCounts(dataset.split_counts),
          `coverage ${formatOptionalProbability(dataset.avg_coverage_score)}`
        ].join(" · "),
        labelSummary: `20d ${dataset.label_20d_count} / 60d ${dataset.label_60d_count} / protected ${dataset.protected_row_count}`,
        coverageDetails: [
          row.coverage_role,
          `特征 ${dataset.feature_name_count}`,
          row.blocking_gaps.length > 0 ? `缺口 ${row.blocking_gaps.join("；")}` : "无主要缺口"
        ].join(" · "),
        p20Summary: `${probability.baseline_hit_20d.hit_count} -> ${probability.candidate_hit_20d.hit_count}`,
        p20Details: [
          `候选均值 ${formatOptionalProbability(probability.candidate_avg_p_20d)}`,
          `峰值 ${formatOptionalProbability(probability.candidate_max_p_20d)}`,
          `Δ ${formatProbabilityDelta(probability.avg_delta_p_20d)}`
        ].join(" · "),
        p60Summary: `${probability.baseline_hit_60d.hit_count} -> ${probability.candidate_hit_60d.hit_count}`,
        p60Details: [
          `候选均值 ${formatOptionalProbability(probability.candidate_avg_p_60d)}`,
          `峰值 ${formatOptionalProbability(probability.candidate_max_p_60d)}`,
          `近阈值 ${probability.candidate_near_threshold_60d_5pp_count}`
        ].join(" · "),
        nextAction: nextActionLabel(row.diagnosis.next_action, row.diagnosis.reasons)
      };
    }) ?? [];

  return (
    <section className="surface">
      <SurfaceHeader title="提前预警缺口审计" icon={SearchCheck} />
      <p className="legend-note">{auditContent.prewarningGapSummary}</p>
      {latestPrewarningGapAudit ? (
        <>
          <MetricGrid items={metrics} className="audit-review-metrics" />
          <RuleBox label="工件来源">
            <span title={source?.hint}>{source?.value ?? "未登记"}</span>
          </RuleBox>
          <RuleBox label="审计上下文">
            <DetailRows items={contextRows} compact />
          </RuleBox>
          {scenarioRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={["场景 / 诊断", "Dataset / 覆盖", "标签 / 缺口", "20d 命中", "60d 命中", "下一步"]}
              note={auditContent.prewarningGapTableNote}
            >
              {scenarioRows.map((row) => (
                <tr key={row.id}>
                  <StackedTableCell title={row.scenarioLabel} details={row.scenarioDetails} />
                  <StackedTableCell title={row.datasetSummary} details={row.datasetDetails} />
                  <StackedTableCell title={row.labelSummary} details={row.coverageDetails} />
                  <StackedTableCell title={row.p20Summary} details={row.p20Details} />
                  <StackedTableCell title={row.p60Summary} details={row.p60Details} />
                  <td>{row.nextAction}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
        </>
      ) : (
        <RuleBox label="当前状态">{auditContent.prewarningGapEmpty}</RuleBox>
      )}
    </section>
  );
}
