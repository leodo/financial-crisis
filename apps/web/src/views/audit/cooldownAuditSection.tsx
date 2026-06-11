import { ClipboardCheck } from "lucide-react";
import {
  compactFileReference,
  formatDate,
  formatDateTime,
  formatPercent,
  releaseIdLabel,
  releaseReviewHistoryModeLabel
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

function cooldownNoGoReasonLabel(code: string): string {
  return (
    {
      actionable_precision_regression: "动作精度退化",
      longest_false_positive_episode_regression: "最长误报变长",
      runtime_floor_hit_count_regression: "运行阈值命中减少",
      candidate_20d_cooldown_bleed: "20d cooldown bleed",
      candidate_20d_cooldown_not_below_positive: "20d 冷却期未低于正例窗口"
    }[code] ?? code
  );
}

function cooldownEpisodeRegressionLabel(kind: string): string {
  return (
    {
      candidate_only: "候选新增误报",
      extended_candidate_episode: "候选拉长误报",
      shared_or_shorter_episode: "未恶化"
    }[kind] ?? kind
  );
}

function cooldownDiagnosisLabel(value: string | null): string {
  return (
    {
      cooldown_bleed: "冷却期外溢",
      usable_early_warning_separation: "可用提前分离",
      separated_but_below_runtime_floor: "分离但低于运行线",
      cold_across_all_regimes: "全 regime 偏冷",
      weak_regime_separation: "分离较弱"
    }[value ?? ""] ?? value ?? "—"
  );
}

function formatOptionalPercent(value: number | null): string {
  return value === null ? "—" : formatPercent(value);
}

export function CooldownAuditSection({ audit }: { audit: ResearchAuditResponse }) {
  const latestCooldownAudit = audit.latest_cooldown_audit;
  const latestCooldownAuditSource = latestCooldownAudit
    ? compactFileReference(latestCooldownAudit.source)
    : null;

  const metrics = latestCooldownAudit
    ? [
        {
          label: "审计时间",
          value: formatDateTime(latestCooldownAudit.generated_at)
        },
        {
          label: "审计结论（离线）",
          value: latestCooldownAudit.recommendation,
          hint: "candidate 晋升审计结论，不是当前面板动作建议。"
        },
        {
          label: "No-Go 原因（离线）",
          value: `${latestCooldownAudit.no_go_reasons.length}`,
          hint: "release review 阻断项数量，不是线上自动放行结果。"
        },
        {
          label: "候选误报变化（历史）",
          value: `${latestCooldownAudit.false_positive_episodes.candidate_regressions.length}`,
          hint: "历史回放 episode 变化，不是今天新增误报。"
        },
        {
          label: "场景误报 delta（历史）",
          value: `${latestCooldownAudit.scenario_false_positive_deltas.length}`,
          hint: "历史场景维度的误报变化条数，不是当前概率准确率。"
        }
      ]
    : [];

  const contextRows = latestCooldownAudit
    ? [
        {
          id: "cooldown-releases",
          title: "基线 / 候选",
          detail: `${releaseIdLabel(latestCooldownAudit.baseline_release_id).value} vs ${releaseIdLabel(latestCooldownAudit.candidate_release_id).value}`,
          note: `history mode: ${releaseReviewHistoryModeLabel(latestCooldownAudit.history_mode)}`
        },
        {
          id: "cooldown-review-artifact",
          title: "Release review",
          detail: latestCooldownAudit.reviewed_at
            ? formatDateTime(latestCooldownAudit.reviewed_at)
            : "未记录评审时间",
          note: compactFileReference(latestCooldownAudit.release_review_artifact).value
        }
      ]
    : [];

  const noGoRows =
    latestCooldownAudit?.no_go_reasons.map((reason) => ({
      id: `cooldown-no-go-${reason.code}`,
      title: cooldownNoGoReasonLabel(reason.code),
      detail: reason.summary,
      note: reason.code
    })) ?? [];

  const runtimeRows =
    latestCooldownAudit?.runtime_cooldown_rows.map((row) => ({
      id: `cooldown-runtime-${row.horizon_days}`,
      horizon: `${row.horizon_days}d`,
      diagnosis: `${cooldownDiagnosisLabel(row.baseline_diagnosis)} -> ${cooldownDiagnosisLabel(row.candidate_diagnosis)}`,
      cooldownVsPositive: formatOptionalPercent(row.candidate_cooldown_minus_positive),
      cooldownVsNormal: formatOptionalPercent(row.candidate_cooldown_minus_normal)
    })) ?? [];

  const episodeRows =
    latestCooldownAudit?.false_positive_episodes.candidate_regressions.map((row, index) => ({
      id: `cooldown-episode-${index}-${row.episode.start_date}`,
      kind: cooldownEpisodeRegressionLabel(row.kind),
      window: `${formatDate(row.episode.start_date)} - ${formatDate(row.episode.end_date)}`,
      duration: `${row.episode.duration_days} 天 / ${row.episode.signal_count} 个信号`,
      overlap:
        row.overlapping_baseline_episodes.length > 0
          ? row.overlapping_baseline_episodes
              .map(
                (episode) =>
                  `${formatDate(episode.start_date)}-${formatDate(episode.end_date)} (${episode.duration_days} 天)`
              )
              .join("；")
          : "baseline 无重叠误报",
      note: row.episode.note
    })) ?? [];

  const scenarioRows =
    latestCooldownAudit?.scenario_false_positive_deltas.map((row) => ({
      id: `cooldown-scenario-${row.scenario_id}`,
      scenario: row.name,
      detail: row.scenario_id,
      falsePositiveDelta: `${row.baseline_false_positive_count} -> ${row.candidate_false_positive_count} (${row.delta >= 0 ? "+" : ""}${row.delta})`,
      outcome: row.outcome ?? "—"
    })) ?? [];

  return (
    <section className="surface">
      <SurfaceHeader title="Cooldown / 误报治理审计" icon={ClipboardCheck} />
      <p className="legend-note">{auditContent.cooldownSummary}</p>
      {latestCooldownAudit ? (
        <>
          <MetricGrid items={metrics} className="audit-review-metrics" />
          <RuleBox label="工件来源">
            <span title={latestCooldownAuditSource?.hint}>
              {latestCooldownAuditSource?.value ?? "未登记"}
            </span>
          </RuleBox>
          <RuleBox label="审计上下文">
            <DetailRows items={contextRows} compact />
          </RuleBox>
          {noGoRows.length > 0 ? (
            <RuleBox label="No-Go 原因">
              <DetailRows items={noGoRows} compact />
            </RuleBox>
          ) : null}
          {runtimeRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table"
              columns={["窗口", "诊断", "冷却期 - 正例窗（历史）", "冷却期 - 常态窗（历史）"]}
              note={auditContent.cooldownRuntimeTableNote}
            >
              {runtimeRows.map((row) => (
                <tr key={row.id}>
                  <td>{row.horizon}</td>
                  <td>{row.diagnosis}</td>
                  <td>{row.cooldownVsPositive}</td>
                  <td>{row.cooldownVsNormal}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          {episodeRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={["变化", "窗口", "持续", "Baseline 重叠", "说明"]}
              note={auditContent.cooldownEpisodeTableNote}
            >
              {episodeRows.map((row) => (
                <tr key={row.id}>
                  <td>{row.kind}</td>
                  <td>{row.window}</td>
                  <td>{row.duration}</td>
                  <td>{row.overlap}</td>
                  <td>{row.note}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          {scenarioRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table"
              columns={["历史场景", "误报数量（历史）", "结果"]}
              note={auditContent.cooldownScenarioTableNote}
            >
              {scenarioRows.map((row) => (
                <tr key={row.id}>
                  <StackedTableCell title={row.scenario} details={row.detail} />
                  <td>{row.falsePositiveDelta}</td>
                  <td>{row.outcome}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
        </>
      ) : (
        <RuleBox label="当前状态">{auditContent.cooldownEmpty}</RuleBox>
      )}
    </section>
  );
}
