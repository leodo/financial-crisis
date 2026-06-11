import { Landmark } from "lucide-react";
import {
  compactFileReference,
  formatDate,
  formatDateTime,
  formatNumber,
  formatPercent,
  formatProbabilityPercentExact,
  formatSignedNumber,
  releaseIdLabel
} from "../../format";
import type { FundingStressFeatureGap, ResearchAuditResponse } from "../../types";
import {
  DetailRows,
  MetricGrid,
  ResponsiveTable,
  RuleBox,
  StackedTableCell,
  SurfaceHeader
} from "../shared/panelHelpers";
import { auditContent } from "./content";
import type {
  FundingStressBaseContribution,
  FundingStressOverlayContribution
} from "../../types";

function diagnosisLabel(value: string): string {
  return (
    {
      no_runtime_floor_signal: "运行阈值未形成",
      partial_runtime_signal: "已有部分运行信号",
      evaluation_only_window: "仅评估 split",
      trainable_or_mixed_split_window: "存在可训练 split",
      mixed_systemic_proxy_missing: "缺 mixed-systemic proxy",
      mixed_systemic_proxy_present: "已有 mixed-systemic proxy",
      candidate_margin_erosion: "候选边际弱化",
      candidate_margin_preserved_or_improved: "候选边际未弱化",
      mixed_systemic_proxy_active: "mixed-systemic proxy 活跃",
      mixed_systemic_proxy_inactive: "mixed-systemic proxy 不活跃"
    }[value] ?? value
  );
}

function countSummary(rows: Array<{ value: string; count: number }>, empty = "无"): string {
  return rows.length > 0 ? rows.map((row) => `${row.value}=${row.count}`).join(" / ") : empty;
}

function formatOptionalProbability(value: number | null): string {
  return value === null ? "—" : formatProbabilityPercentExact(value);
}

function formatOptionalPercent(value: number | null): string {
  return value === null ? "—" : formatPercent(value);
}

function formatGap(value: number | null): string {
  return value === null ? "—" : formatSignedNumber(value * 100, 2, "%");
}

function featureLabel(feature: string): string {
  return (
    {
      us_nfci_level: "NFCI 金融条件",
      us_stlfsi_level: "STLFSI 压力",
      us_baa_10y_spread_level: "BAA-10Y 信用利差",
      us_curve_10y2y_level: "10Y-2Y 曲线",
      us_vix_level: "VIX",
      us_vix_change_5d: "VIX 5d 变化",
      us_usdjpy_level: "USDJPY",
      us_usdjpy_change_20d: "USDJPY 20d 变化",
      us_fed_funds_level: "联邦基金利率",
      us_housing_starts_level: "新屋开工",
      family_proxy__mixed_systemic: "Mixed-systemic proxy",
      family_proxy__systemic_credit: "Systemic-credit proxy",
      family_proxy__rate_shock: "Rate-shock proxy",
      family_proxy__jpy_carry: "JPY carry proxy",
      family_proxy__acute_liquidity: "Acute-liquidity proxy",
      interaction__external_dimension_score__us_usdjpy_level: "外部维度 × USDJPY",
      interaction__us_curve_10y2y_level__us_fed_funds_level: "曲线 × 联邦基金利率",
      tail_pos__us_usdjpy_level__145: "USDJPY 高位 tail",
      tail_abs_pos__us_usdjpy_change_20d__4: "USDJPY 20d 变化 tail",
      external_dimension_score: "外部维度分",
      structural_score: "结构分",
      trigger_score: "触发分",
      overall_score: "总风险分"
    }[feature] ?? feature
  );
}

function topFeatureRows(audit: NonNullable<ResearchAuditResponse["latest_funding_stress_audit"]>) {
  const separation = audit.feature_context?.separation ?? {};
  const preferred = separation.positive_window_vs_normal_20d;
  const fallback = separation.candidate_top20_vs_rest;
  return (preferred && preferred.length > 0 ? preferred : fallback ?? []).slice(0, 8);
}

function featureRowId(row: FundingStressFeatureGap, index: number): string {
  return `${row.left_group}-${row.right_group}-${row.feature}-${index}`;
}

function contributionRowId(
  row: FundingStressBaseContribution | FundingStressOverlayContribution,
  index: number
): string {
  return "name" in row
    ? `${row.name}-${index}`
    : `${row.family_id}-${row.gate_feature}-${index}`;
}

export function FundingStressAuditSection({ audit }: { audit: ResearchAuditResponse }) {
  const latestFundingStressAudit = audit.latest_funding_stress_audit;
  const source = latestFundingStressAudit
    ? compactFileReference(latestFundingStressAudit.source)
    : null;

  const fullWindow = latestFundingStressAudit?.probability_evidence.full_window;
  const dataset = latestFundingStressAudit?.dataset_evidence;
  const diagnosis = latestFundingStressAudit?.diagnosis;

  const metrics = latestFundingStressAudit && fullWindow && dataset
    ? [
        { label: "审计时间", value: formatDateTime(latestFundingStressAudit.generated_at) },
        {
          label: "样本行数（历史）",
          value: `${latestFundingStressAudit.row_count}`,
          hint: "2011 审计窗口中的 dataset 行数，不是当前模型训练样本总数。"
        },
        {
          label: "20d 历史峰值 / 入线",
          value: `${formatOptionalProbability(fullWindow.candidate_max_p20d.value)} / ${formatOptionalProbability(latestFundingStressAudit.thresholds.candidate_20d)}`,
          hint: "历史窗口 candidate 峰值对离线运行线，不是当前 20d 风险距离。"
        },
        {
          label: "60d 历史峰值 / 入线",
          value: `${formatOptionalProbability(fullWindow.candidate_max_p60d.value)} / ${formatOptionalProbability(latestFundingStressAudit.thresholds.candidate_60d)}`,
          hint: "历史窗口 candidate 峰值对离线运行线，不是当前 60d 风险距离。"
        },
        {
          label: "Split（历史）",
          value: countSummary(dataset.split_counts),
          hint: "审计 dataset 的 split 分布，不是上线状态。"
        },
        {
          label: "Family Context",
          value: diagnosisLabel(diagnosis?.family_context_class ?? "—")
        }
      ]
    : [];

  const contextRows = latestFundingStressAudit
    ? [
        {
          id: "funding-stress-window",
          title: "审计窗口",
          detail: `${formatDate(latestFundingStressAudit.from_date)} - ${formatDate(latestFundingStressAudit.to_date)}`,
          note: `Dataset: ${latestFundingStressAudit.dataset_key}`
        },
        {
          id: "funding-stress-releases",
          title: "基线 / 候选",
          detail: `${releaseIdLabel(latestFundingStressAudit.baseline_release_id).value} vs ${releaseIdLabel(latestFundingStressAudit.candidate_release_id).value}`,
          note: latestFundingStressAudit.market_scope
        },
        {
          id: "funding-stress-coverage",
          title: "免费数据覆盖",
          detail: `${latestFundingStressAudit.coverage.coverage_grade} · ${latestFundingStressAudit.coverage.recommended_role}`,
          note: latestFundingStressAudit.coverage.free_sources.join(" / ")
        }
      ]
    : [];

  const diagnosisRows = latestFundingStressAudit && diagnosis
    ? [
        {
          id: "funding-stress-diagnosis",
          title: diagnosisLabel(diagnosis.primary_class),
          detail: [
            diagnosisLabel(diagnosis.trainability_class),
            diagnosisLabel(diagnosis.family_context_class),
            diagnosisLabel(diagnosis.candidate_margin_class)
          ].join(" · "),
          note: diagnosis.reasons.join("；")
        },
        {
          id: "funding-stress-next-actions",
          title: "下一步",
          detail: diagnosis.next_actions.join("；") || "—",
          note: "这块是训练/特征治理建议，不是仓位建议。"
        }
      ]
    : [];

  const thresholdRows = latestFundingStressAudit && fullWindow
    ? [
        {
          id: "20d",
          horizon: "20d",
          max: formatOptionalProbability(fullWindow.candidate_max_p20d.value),
          floor: formatOptionalProbability(latestFundingStressAudit.thresholds.candidate_20d),
          gap: formatGap(
            fullWindow.candidate_max_p20d.value !== null &&
              latestFundingStressAudit.thresholds.candidate_20d !== null
              ? fullWindow.candidate_max_p20d.value -
                  latestFundingStressAudit.thresholds.candidate_20d
              : null
          ),
          hits: `${fullWindow.candidate_hit_20d.hit_count}`,
          near: `${fullWindow.near_candidate_20d_5pp.count}`
        },
        {
          id: "60d",
          horizon: "60d",
          max: formatOptionalProbability(fullWindow.candidate_max_p60d.value),
          floor: formatOptionalProbability(latestFundingStressAudit.thresholds.candidate_60d),
          gap: formatGap(
            fullWindow.candidate_max_p60d.value !== null &&
              latestFundingStressAudit.thresholds.candidate_60d !== null
              ? fullWindow.candidate_max_p60d.value -
                  latestFundingStressAudit.thresholds.candidate_60d
              : null
          ),
          hits: `${fullWindow.candidate_hit_60d.hit_count}`,
          near: `${fullWindow.near_candidate_60d_5pp.count}`
        }
      ]
    : [];

  const featureRows = latestFundingStressAudit ? topFeatureRows(latestFundingStressAudit) : [];
  const contributionGroup =
    latestFundingStressAudit?.feature_context?.candidate_absolute_contributions?.full_window_20d;
  const positiveContributionRows = contributionGroup?.top_positive_base ?? [];
  const negativeContributionRows = contributionGroup?.top_negative_base ?? [];
  const contributionRows = contributionGroup
    ? [
        ...positiveContributionRows.slice(0, 6).map((row) => ({
          ...row,
          direction: "推高"
        })),
        ...negativeContributionRows.slice(0, 6).map((row) => ({
          ...row,
          direction: "压低"
        }))
      ]
    : [];
  const overlayRows = (contributionGroup?.overlay_contributions ?? []).slice(0, 6);

  return (
    <section className="surface">
      <SurfaceHeader title="2011 Funding Stress 审计" icon={Landmark} />
      <p className="legend-note">{auditContent.fundingStressSummary}</p>
      {latestFundingStressAudit ? (
        <>
          <MetricGrid items={metrics} className="audit-review-metrics" />
          <RuleBox label="工件来源">
            <span title={source?.hint}>{source?.value ?? "未登记"}</span>
          </RuleBox>
          <RuleBox label="审计上下文">
            <DetailRows items={contextRows} compact />
          </RuleBox>
          <RuleBox label="诊断">
            <DetailRows items={diagnosisRows} compact />
          </RuleBox>
          <ResponsiveTable
            className="wide-table"
            columns={["期限", "候选历史峰值", "离线运行线", "离入线（审计）", "历史命中点", "近线点"]}
          >
            {thresholdRows.map((row) => (
              <tr key={row.id}>
                <td>{row.horizon}</td>
                <td>{row.max}</td>
                <td>{row.floor}</td>
                <td>{row.gap}</td>
                <td>{row.hits}</td>
                <td>{row.near}</td>
              </tr>
            ))}
          </ResponsiveTable>
          {dataset ? (
            <RuleBox label="Dataset 证据（历史样本）">
              {[
                `action ${countSummary(dataset.action_level_counts)}`,
                `regime20 ${countSummary(dataset.regime_20d_counts)}`,
                `protected ${dataset.protected_row_count}`,
                `数据覆盖 ${formatOptionalPercent(dataset.avg_coverage_score)}`,
                `raw features ${dataset.raw_feature_name_count || dataset.feature_name_count}`,
                `resolved features ${dataset.resolved_feature_name_count || 0}`,
                dataset.missing_relevant_features.length > 0
                  ? `缺少 ${dataset.missing_relevant_features.slice(0, 4).join(" / ")}`
                  : "关键 family/context 特征齐全"
              ].join(" · ")}
            </RuleBox>
          ) : null}
          {featureRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={["特征", "分组", "均值差", "标准化差距"]}
              note={auditContent.fundingStressFeatureTableNote}
            >
              {featureRows.map((row, index) => (
                <tr key={featureRowId(row, index)}>
                  <StackedTableCell title={featureLabel(row.feature)} details={row.feature} />
                  <td>{`${row.left_group} vs ${row.right_group}`}</td>
                  <td>{row.mean_delta === null ? "—" : formatNumber(row.mean_delta)}</td>
                  <td>{row.standardized_gap === null ? "—" : formatNumber(row.standardized_gap)}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          {contributionRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table xwide-table"
              columns={["方向", "特征", "均值贡献", "raw / normalized", "权重"]}
              note={auditContent.fundingStressContributionTableNote}
            >
              {contributionRows.map((row, index) => (
                <tr key={contributionRowId(row, index)}>
                  <td>{row.direction}</td>
                  <StackedTableCell title={featureLabel(row.name)} details={row.name} />
                  <td>{row.mean_contribution === null ? "—" : formatSignedNumber(row.mean_contribution, 4)}</td>
                  <td>{`${formatNumber(row.mean_raw_value)} / ${formatNumber(row.mean_normalized_value)}`}</td>
                  <td>{row.mean_weight === null ? "—" : formatSignedNumber(row.mean_weight, 4)}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
          {overlayRows.length > 0 ? (
            <ResponsiveTable
              className="wide-table"
              columns={["Overlay", "Gate", "Blend", "均值贡献"]}
              note={auditContent.fundingStressOverlayTableNote}
            >
              {overlayRows.map((row, index) => (
                <tr key={contributionRowId(row, index)}>
                  <td>{row.family_id}</td>
                  <StackedTableCell
                    title={featureLabel(row.gate_feature)}
                    details={`gate value ${formatNumber(row.mean_gate_value)} / gate ${formatNumber(row.mean_gate)}`}
                  />
                  <td>{formatPercent(row.mean_blend)}</td>
                  <td>{row.mean_contribution === null ? "—" : formatSignedNumber(row.mean_contribution, 4)}</td>
                </tr>
              ))}
            </ResponsiveTable>
          ) : null}
        </>
      ) : (
        <RuleBox label="当前状态">{auditContent.fundingStressEmpty}</RuleBox>
      )}
    </section>
  );
}
