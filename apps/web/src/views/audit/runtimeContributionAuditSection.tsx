import { Activity } from "lucide-react";
import {
  compactFileReference,
  formatDate,
  formatDateTime,
  formatNumber,
  formatProbabilityPercentExact,
  formatSignedNumber,
  releaseIdLabel,
  releaseReviewHistoryModeLabel
} from "../../format";
import type {
  ResearchAuditResponse,
  RuntimeContributionBaseContributionSummary,
  RuntimeContributionSemanticAnomaly
} from "../../types";
import {
  DetailRows,
  MetricGrid,
  ResponsiveTable,
  RuleBox,
  SurfaceHeader
} from "../shared/panelHelpers";
import { auditContent } from "./content";

function anomalyLabel(code: string): string {
  return (
    {
      usdjpy_high_tail_negative: "USDJPY 高位 tail 负贡献",
      usdjpy_change_negative: "USDJPY 20日变化负贡献"
    }[code] ?? code
  );
}

function compactGroupLabel(value: string): string {
  return value
    .replace("bucket=", "")
    .replace("|posture=", " / ")
    .replace("|floor=", " / ")
    .replaceAll("_", " ");
}

function contributionLabel(row: RuntimeContributionBaseContributionSummary | undefined): string {
  if (!row) {
    return "—";
  }
  return `${row.name} ${formatSignedNumber(row.mean_contribution, 2)}`;
}

function anomalySummary(rows: RuntimeContributionSemanticAnomaly[]): string {
  if (rows.length === 0) {
    return "无";
  }
  return rows.map((row) => anomalyLabel(row.code)).join("；");
}

export function RuntimeContributionAuditSection({ audit }: { audit: ResearchAuditResponse }) {
  const latestRuntimeAudit = audit.latest_runtime_contribution_audit;
  const latestRuntimeAuditSource = latestRuntimeAudit
    ? compactFileReference(latestRuntimeAudit.source)
    : null;

  const metrics = latestRuntimeAudit
    ? [
        {
          label: "审计时间",
          value: formatDateTime(latestRuntimeAudit.generated_at)
        },
        {
          label: "审计窗口",
          value: `${formatDate(latestRuntimeAudit.from_date)} - ${formatDate(latestRuntimeAudit.to_date)}`
        },
        {
          label: "共同日期",
          value: `${latestRuntimeAudit.common_date_count}`,
          hint: "baseline 与 candidate 可比较的运行窗口日期数，不是训练样本数。"
        },
        {
          label: "审计期限",
          value: latestRuntimeAudit.horizons.map((row) => `${row.horizon_days}d`).join(" / "),
          hint: "这里按离线审计 horizon 分组，不是当前页面三期限正式概率结论。"
        },
        {
          label: "审计归因",
          value: `${latestRuntimeAudit.takeaways.length}`,
          hint: "artifact 输出的模型诊断结论条数，不是 Go/No-Go 放行项。"
        }
      ]
    : [];

  const contextRows = latestRuntimeAudit
    ? [
        {
          id: "runtime-contribution-releases",
          title: "基线 / 候选",
          detail: `${releaseIdLabel(latestRuntimeAudit.baseline_release_id).value} vs ${releaseIdLabel(latestRuntimeAudit.candidate_release_id).value}`,
          note: `history mode: ${releaseReviewHistoryModeLabel(latestRuntimeAudit.history_mode)}`
        },
        {
          id: "runtime-contribution-slices",
          title: "Runtime slice",
          detail: compactFileReference(latestRuntimeAudit.baseline_slice_path).value,
          note: compactFileReference(latestRuntimeAudit.candidate_slice_path).value
        },
        {
          id: "runtime-contribution-thresholds",
          title: "Threshold source",
          detail: latestRuntimeAudit.runtime_threshold_source,
          note: `baseline ${latestRuntimeAudit.baseline_threshold_source} / candidate ${latestRuntimeAudit.candidate_threshold_source}`
        }
      ]
    : [];

  const horizonRows =
    latestRuntimeAudit?.horizons.map((row) => ({
      id: `runtime-contribution-horizon-${row.horizon_days}`,
      horizon: `${row.horizon_days}d`,
      baseline: `${formatProbabilityPercentExact(row.baseline_avg_runtime_probability)} / ${formatProbabilityPercentExact(row.baseline_decision_threshold)}`,
      candidate: `${formatProbabilityPercentExact(row.candidate_avg_runtime_probability)} / ${formatProbabilityPercentExact(row.candidate_decision_threshold)}`,
      touchline: `${formatProbabilityPercentExact(row.baseline_touchline_ratio)} -> ${formatProbabilityPercentExact(row.candidate_touchline_ratio)}`,
      anomalies: `baseline ${row.baseline_semantic_anomalies.length} / candidate ${row.candidate_semantic_anomalies.length}`,
      candidateGroups:
        row.runtime_group_summaries.length > 0
          ? row.runtime_group_summaries
              .map(
                (group) =>
                  `${compactGroupLabel(group.label)}: ${group.date_count}天, ${formatProbabilityPercentExact(
                    group.candidate_avg_runtime_probability
                  )}`
              )
              .join("；")
          : "未分组",
      topCandidateDrag: contributionLabel(row.candidate_top_negative_base_contributions[0])
    })) ?? [];

  const anomalyRows =
    latestRuntimeAudit?.horizons.flatMap((row) =>
      [...row.baseline_semantic_anomalies, ...row.candidate_semantic_anomalies].map(
        (anomaly, index) => ({
          id: `runtime-contribution-anomaly-${row.horizon_days}-${index}-${anomaly.code}`,
          horizon: `${row.horizon_days}d`,
          label: anomalyLabel(anomaly.code),
          feature: anomaly.feature,
          raw: formatNumber(anomaly.mean_raw_value),
          contribution: formatSignedNumber(anomaly.mean_contribution, 2),
          message: anomaly.message
        })
      )
    ) ?? [];

  const groupRows =
    latestRuntimeAudit?.horizons.flatMap((row) =>
      row.runtime_group_summaries.map((group) => ({
        id: `runtime-contribution-group-${row.horizon_days}-${group.group}`,
        horizon: `${row.horizon_days}d`,
        group: compactGroupLabel(group.label),
        days: `${group.date_count}`,
        candidate: `${formatProbabilityPercentExact(group.candidate_avg_runtime_probability)} / ${formatProbabilityPercentExact(group.candidate_decision_threshold)}`,
        touchline: formatProbabilityPercentExact(group.candidate_touchline_ratio),
        anomalies: anomalySummary(group.candidate_semantic_anomalies),
        topDrag: contributionLabel(group.candidate_top_negative_base_contributions[0])
      }))
    ) ?? [];

  const latestDateRows =
    latestRuntimeAudit?.horizons.map((row) => {
      const latest = row.date_rows.at(-1);
      return {
        id: `runtime-contribution-date-${row.horizon_days}`,
        horizon: `${row.horizon_days}d`,
        date: latest ? formatDate(latest.as_of_date) : "—",
        baseline: latest
          ? formatProbabilityPercentExact(latest.baseline_runtime_probability)
          : "—",
        candidate: latest
          ? formatProbabilityPercentExact(latest.candidate_runtime_probability)
          : "—",
        touchline: latest
          ? `${formatProbabilityPercentExact(latest.baseline_touchline_ratio)} -> ${formatProbabilityPercentExact(latest.candidate_touchline_ratio)}`
          : "—",
        state: latest
          ? `${latest.baseline_time_to_risk_bucket}/${latest.baseline_posture} -> ${latest.candidate_time_to_risk_bucket}/${latest.candidate_posture}`
          : "—"
      };
    }) ?? [];

  return (
    <section className="surface">
      <SurfaceHeader title="Runtime Contribution / 触线审计" icon={Activity} />
      <p className="legend-note">{auditContent.runtimeContributionSummary}</p>
      {latestRuntimeAudit ? (
        <>
          <MetricGrid items={metrics} className="audit-review-metrics" />
          <RuleBox label="工件来源">
            <span title={latestRuntimeAuditSource?.hint}>
              {latestRuntimeAuditSource?.value ?? "未登记"}
            </span>
          </RuleBox>
          <RuleBox label="审计上下文">
            <DetailRows items={contextRows} compact />
          </RuleBox>
          {latestRuntimeAudit.methodology_limitations.length > 0 ? (
            <RuleBox label="方法边界">
              <DetailRows
                compact
                items={latestRuntimeAudit.methodology_limitations.map((limitation, index) => ({
                  id: `runtime-contribution-limitation-${index}`,
                  title: `限制 ${index + 1}`,
                  detail: limitation
                }))}
              />
            </RuleBox>
          ) : null}
          <ResponsiveTable
            className="wide-table xwide-table"
            columns={["窗口", "Baseline 均值/运行线", "Candidate 均值/运行线", "入线占比（审计）", "语义异常", "候选 runtime group", "候选最大拖累"]}
            note={auditContent.runtimeContributionHorizonTableNote}
          >
            {horizonRows.map((row) => (
              <tr key={row.id}>
                <td>{row.horizon}</td>
                <td>{row.baseline}</td>
                <td>{row.candidate}</td>
                <td>{row.touchline}</td>
                <td>{row.anomalies}</td>
                <td>{row.candidateGroups}</td>
                <td>{row.topCandidateDrag}</td>
              </tr>
            ))}
          </ResponsiveTable>
          {groupRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={["窗口", "Runtime group", "天数", "候选均值/运行线", "候选入线占比", "语义异常", "候选最大拖累"]}
              note={auditContent.runtimeContributionGroupTableNote}
            >
              {groupRows.map((row) => (
                <tr key={row.id}>
                  <td>{row.horizon}</td>
                  <td>{row.group}</td>
                  <td>{row.days}</td>
                  <td>{row.candidate}</td>
                  <td>{row.touchline}</td>
                  <td>{row.anomalies}</td>
                  <td>{row.topDrag}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          {latestDateRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table"
              columns={["窗口", "最新日期", "Baseline", "Candidate", "入线占比（审计）", "状态迁移"]}
              note={auditContent.runtimeContributionLatestDateTableNote}
            >
              {latestDateRows.map((row) => (
                <tr key={row.id}>
                  <td>{row.horizon}</td>
                  <td>{row.date}</td>
                  <td>{row.baseline}</td>
                  <td>{row.candidate}</td>
                  <td>{row.touchline}</td>
                  <td>{row.state}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          {anomalyRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={["窗口", "异常", "特征", "均值", "贡献", "说明"]}
              note={auditContent.runtimeContributionAnomalyTableNote}
            >
              {anomalyRows.map((row) => (
                <tr key={row.id}>
                  <td>{row.horizon}</td>
                  <td>{row.label}</td>
                  <td>{row.feature}</td>
                  <td>{row.raw}</td>
                  <td>{row.contribution}</td>
                  <td>{row.message}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          {latestRuntimeAudit.takeaways.length > 0 ? (
            <RuleBox label="Takeaways">
              <DetailRows
                compact
                items={latestRuntimeAudit.takeaways.slice(0, 8).map((takeaway, index) => ({
                  id: `runtime-contribution-takeaway-${index}`,
                  title: `#${index + 1}`,
                  detail: takeaway
                }))}
              />
            </RuleBox>
          ) : null}
        </>
      ) : (
        <RuleBox label="当前状态">{auditContent.runtimeContributionEmpty}</RuleBox>
      )}
    </section>
  );
}
