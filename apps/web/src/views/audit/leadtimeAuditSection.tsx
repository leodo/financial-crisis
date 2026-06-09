import { TimerReset } from "lucide-react";
import {
  compactFileReference,
  formatDateTime,
  formatPercent,
  formatProbabilityPercentExact,
  formatSignedNumber,
  releaseIdLabel
} from "../../format";
import type { LeadtimeFocusRow, LeadtimeRuntimeRow, ResearchAuditResponse } from "../../types";
import {
  DetailRows,
  MetricGrid,
  ResponsiveTable,
  RuleBox,
  SurfaceHeader,
} from "../shared/panelHelpers";
import { auditContent } from "./content";

function metricValue(audit: NonNullable<ResearchAuditResponse["latest_leadtime_audit"]>, metric: string) {
  return audit.metric_rows.find((row) => row.metric === metric) ?? null;
}

function formatMetricDelta(value: number | null, kind: "percent" | "count" = "count"): string {
  if (value === null) {
    return "—";
  }
  return kind === "percent" ? formatSignedNumber(value * 100, 1, "pp") : formatSignedNumber(value, 0);
}

function formatMetricValue(value: number | null, kind: "percent" | "count" = "count"): string {
  if (value === null) {
    return "—";
  }
  return kind === "percent" ? formatPercent(value) : `${Math.round(value)}`;
}

function formatProbability(value: number | null): string {
  return value === null ? "—" : formatProbabilityPercentExact(value);
}

function diagnosisLabel(value: string | null): string {
  if (!value) {
    return "—";
  }
  return (
    {
      usable_early_warning_separation: "已有提前分离",
      separated_but_below_runtime_floor: "有分离但低于 floor",
      cooldown_bleed: "冷却期外溢",
      weak_regime_separation: "分离偏弱",
      late_only_no_early_warning: "只会过晚确认",
      no_early_warning_separation: "没有提前分离"
    }[value] ?? value
  );
}

function failureModeLabel(value: string | null): string {
  if (!value) {
    return "—";
  }
  return (
    {
      strict_gate_mismatch: "strict gate 不匹配",
      posture_continuity_failure: "posture 连续性不足",
      score_confirmation_failure: "分数确认不足",
      no_runtime_signal: "无 runtime 信号"
    }[value] ?? value
  );
}

function compactCategory(value: string | null): string {
  if (!value) {
    return "—";
  }
  return value
    .replaceAll("posture_bucket_normal", "posture normal")
    .replaceAll("review_gate_gap", "review gate gap")
    .replaceAll("prepare_weeks_score_confirmation", "prepare weeks score")
    .replaceAll("review_l3_gate_not_satisfied", "L3 gate");
}

function scenarioLabel(row: LeadtimeFocusRow): string {
  return row.name || row.scenario_id;
}

function sortFocusRows(rows: LeadtimeFocusRow[]): LeadtimeFocusRow[] {
  return [...rows]
    .sort((left, right) => {
      const leftCount = left.candidate_dominant_runtime_block_count ?? 0;
      const rightCount = right.candidate_dominant_runtime_block_count ?? 0;
      return rightCount - leftCount;
    })
    .slice(0, 8);
}

function runtimeRowId(row: LeadtimeRuntimeRow): string {
  return `${row.horizon_days}-${row.baseline_diagnosis}-${row.candidate_diagnosis}`;
}

export function LeadtimeAuditSection({ audit }: { audit: ResearchAuditResponse }) {
  const latestLeadtimeAudit = audit.latest_leadtime_audit;
  const source = latestLeadtimeAudit ? compactFileReference(latestLeadtimeAudit.source) : null;

  const metrics = latestLeadtimeAudit
    ? [
        { label: "审计时间", value: formatDateTime(latestLeadtimeAudit.generated_at) },
        {
          label: "Timely warning",
          value: formatMetricValue(metricValue(latestLeadtimeAudit, "timely_warning_rate")?.candidate ?? null, "percent"),
          hint: `Δ ${formatMetricDelta(metricValue(latestLeadtimeAudit, "timely_warning_rate")?.delta ?? null, "percent")}`
        },
        {
          label: "Strict actionable",
          value: formatMetricValue(metricValue(latestLeadtimeAudit, "strict_actionable_point_count")?.candidate ?? null),
          hint: `Δ ${formatMetricDelta(metricValue(latestLeadtimeAudit, "strict_actionable_point_count")?.delta ?? null)}`
        },
        {
          label: "Runtime floor hits",
          value: formatMetricValue(metricValue(latestLeadtimeAudit, "runtime_floor_hit_count")?.candidate ?? null),
          hint: `Δ ${formatMetricDelta(metricValue(latestLeadtimeAudit, "runtime_floor_hit_count")?.delta ?? null)}`
        },
        {
          label: "Action precision",
          value: formatMetricValue(metricValue(latestLeadtimeAudit, "actionable_precision")?.candidate ?? null, "percent"),
          hint: `Δ ${formatMetricDelta(metricValue(latestLeadtimeAudit, "actionable_precision")?.delta ?? null, "percent")}`
        },
        {
          label: "最长纯误报",
          value: `${formatMetricValue(metricValue(latestLeadtimeAudit, "longest_false_positive_episode_days")?.candidate ?? null)} 天`,
          hint: `Δ ${formatMetricDelta(metricValue(latestLeadtimeAudit, "longest_false_positive_episode_days")?.delta ?? null)} 天`
        }
      ]
    : [];

  const contextRows = latestLeadtimeAudit
    ? [
        {
          id: "leadtime-source",
          title: "工件来源",
          detail: source?.value ?? "未登记",
          note: source?.hint
        },
        {
          id: "leadtime-review",
          title: "基线 / 候选",
          detail: `${releaseIdLabel(latestLeadtimeAudit.baseline_release_id).value} vs ${releaseIdLabel(latestLeadtimeAudit.candidate_release_id).value}`,
          note: `${latestLeadtimeAudit.history_mode} · ${latestLeadtimeAudit.market_scope}`
        },
        {
          id: "leadtime-review-artifact",
          title: "Release review",
          detail: compactFileReference(latestLeadtimeAudit.release_review_artifact).value,
          note: latestLeadtimeAudit.reviewed_at ? `reviewed ${formatDateTime(latestLeadtimeAudit.reviewed_at)}` : undefined
        }
      ]
    : [];

  const topFocusRows = latestLeadtimeAudit ? sortFocusRows(latestLeadtimeAudit.focus_rows) : [];
  const leadtimeGaps = latestLeadtimeAudit?.leadtime_gap_rows ?? [];

  return (
    <section className="surface audit-section">
      <SurfaceHeader
        title="可执行提前量转化审计"
        icon={TimerReset}
      />
      <p className="legend-note">{auditContent.leadtimeSummary}</p>
      {latestLeadtimeAudit ? (
        <>
          <MetricGrid items={metrics} className="audit-review-metrics" />
          <DetailRows items={contextRows} />
          <RuleBox label="关键结论">
            {latestLeadtimeAudit.takeaways.slice(0, 5).join("；") || "当前 artifact 没有输出新增结论。"}
          </RuleBox>
          <ResponsiveTable
            className="wide-table"
            columns={["期限", "基线诊断", "候选诊断", "候选 EW/Normal", "候选 floor gap", "候选命中率"]}
            note={auditContent.leadtimeRuntimeTableNote}
          >
            {latestLeadtimeAudit.runtime_rows.map((row) => (
              <tr key={runtimeRowId(row)}>
                <td>{`${row.horizon_days}d`}</td>
                <td>{diagnosisLabel(row.baseline_diagnosis)}</td>
                <td>{diagnosisLabel(row.candidate_diagnosis)}</td>
                <td>{`${formatProbability(row.candidate_early_warning_avg_probability)} / ${formatProbability(row.candidate_normal_avg_probability)}`}</td>
                <td>{row.candidate_floor_gap === null ? "—" : formatSignedNumber(row.candidate_floor_gap * 100, 2, "%")}</td>
                <td>{row.candidate_threshold_hit_rate === null ? "—" : formatPercent(row.candidate_threshold_hit_rate)}</td>
              </tr>
            ))}
          </ResponsiveTable>
          {leadtimeGaps.length > 0 ? (
            <ResponsiveTable
              className="wide-table"
              columns={["场景", "结果", "候选 L2", "候选 L3", "缺口"]}
              note="L2 表示提前量已经出现，L3 表示达到可执行动作口径；有 L2 无 L3 时，下一步要查 posture、gate 和 sustained-hit。"
            >
              {leadtimeGaps.map((row) => (
                <tr key={row.scenario_id}>
                  <td>{row.name}</td>
                  <td>{row.outcome ?? "—"}</td>
                  <td>{row.candidate_lead_time_days === null ? "—" : `${row.candidate_lead_time_days} 天`}</td>
                  <td>{row.candidate_actionable_lead_time_days === null ? "未转化" : `${row.candidate_actionable_lead_time_days} 天`}</td>
                  <td>{row.signal_source ?? "posture / gate / sustained-hit"}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          <ResponsiveTable
            className="wide-table xwide-table"
            columns={["场景", "候选失败模式", "候选主阻塞", "候选连续性 facet", "候选 runtime / strict"]}
            note="这张表回答“为什么已经有 runtime floor hit，却没有变成更高的 timely warning”。"
          >
            {topFocusRows.map((row) => (
              <tr key={row.scenario_id}>
                <td>{scenarioLabel(row)}</td>
                <td>{failureModeLabel(row.candidate_primary_failure_mode)}</td>
                <td>{compactCategory(row.candidate_dominant_runtime_block)}</td>
                <td>{compactCategory(row.candidate_dominant_continuity_facet)}</td>
                <td>{`${row.candidate_runtime_floor_hit_point_count ?? 0} / ${row.candidate_actionable_point_count ?? 0}`}</td>
              </tr>
            ))}
          </ResponsiveTable>
        </>
      ) : (
        <RuleBox label="当前状态">{auditContent.leadtimeEmpty}</RuleBox>
      )}
    </section>
  );
}
