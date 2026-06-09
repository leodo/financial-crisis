import {
  describePostureClause,
  formatNumber,
  formatProbabilityBasisPoints,
  formatProbabilityDecimal,
  formatPercentPrecise,
  formatProbabilityPercentExact
} from "../../format";
import type { AssessmentSnapshot, ProbabilityHorizonOverlayDiagnostics } from "../../types";
import { RuleBox } from "../shared/panelHelpers";
import type { DecisionSignalLayerRowModel } from "./useDecisionViewModel";

const POSTURE_STEPS: Array<{
  id: AssessmentSnapshot["posture"];
  label: string;
  description: string;
}> = [
  { id: "normal", label: "正常观察", description: "没有看到近端风险窗口，保持监控。 " },
  { id: "prepare", label: "提前准备", description: "脆弱性在积累，先准备现金和对冲工具。" },
  { id: "hedge", label: "保护性对冲", description: "未来几周风险升高，保护动作需要前置。" },
  { id: "defend", label: "防守优先", description: "短期窗口已打开，先保流动性和资本。 " }
];

function formatPercentagePointGap(value: number): string {
  return formatPercentPrecise(value).replace("%", " 个百分点");
}

function formatThresholdMultiple(value: number): string {
  if (!Number.isFinite(value)) {
    return "—";
  }
  if (value >= 1000) {
    return `${Math.round(value).toLocaleString("zh-CN")} 倍`;
  }
  if (value >= 100) {
    return `${Math.round(value)} 倍`;
  }
  if (value >= 10) {
    return `${value.toFixed(1)} 倍`;
  }
  return `${value.toFixed(2)} 倍`;
}

function describeProbabilityBand(value: number) {
  if (value < 0.15) {
    return {
      label: "低位",
      className: "band-low",
      note: "更像常态观察区，通常不需要明显收缩仓位。"
    };
  }
  if (value < 0.3) {
    return {
      label: "准备区",
      className: "band-prepare",
      note: "开始准备流动性和保护工具，避免被动离场。"
    };
  }
  if (value < 0.5) {
    return {
      label: "对冲区",
      className: "band-hedge",
      note: "未来几周风险抬升，保护动作通常要前置。"
    };
  }
  return {
    label: "防守区",
    className: "band-defend",
    note: "近端窗口已明显打开，应优先考虑保护和降杠杆。"
  };
}

function familyLabel(familyId: string): string {
  const labels: Record<string, string> = {
    systemic_credit: "系统性信用",
    mixed_systemic: "混合系统",
    rate_shock: "利率冲击",
    acute_liquidity: "急性流动性",
    jpy_carry: "日元套息"
  };
  return labels[familyId] ?? familyId;
}

function formatGateValue(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return value.toFixed(3).replace(/(?:\.0+|(\.\d*?[1-9])0+)$/, "$1");
}

function gateSignalLabel(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "未返回";
  }
  if (value === 0) {
    return "当前未触发";
  }
  return `proxy ${formatGateValue(value)}`;
}

function buildGateRows(diagnostic?: ProbabilityHorizonOverlayDiagnostics) {
  if (!diagnostic || diagnostic.contributions.length === 0) {
    return [];
  }

  return diagnostic.contributions.map((contribution) => {
    const audit = diagnostic.overlay_audits.find(
      (row) => row.family_id === contribution.family_id
    );
    const threshold = audit?.gate_active_threshold;
    const isActive = threshold !== undefined && contribution.gate_value >= threshold;
    return {
      familyId: contribution.family_id,
      label: familyLabel(contribution.family_id),
      value: gateSignalLabel(contribution.gate_value),
      threshold: threshold === undefined ? "—" : formatGateValue(threshold),
      status: isActive ? "打开" : "未打开",
      className: isActive ? "gate-active" : "gate-quiet"
    };
  });
}

function probabilityReadingNote(value: number): string {
  if (value === 0) {
    return "当前接口精确返回 0，需要结合数据日期、release 状态和模型链路复核；它不等于市场风险被证明为零。";
  }
  if (value < 0.01) {
    return `当前是低位小概率但不是 0；原始接口值为 ${formatProbabilityDecimal(
      value
    )}，也就是 ${formatProbabilityBasisPoints(value)}。`;
  }
  return "当前概率已高于 1%，应重点看是否接近对应进入线和动作档位。";
}

function ProbabilityDiagnosticsBlock({
  diagnostic
}: {
  diagnostic?: ProbabilityHorizonOverlayDiagnostics;
}) {
  if (!diagnostic) {
    return (
      <div className="probability-diagnostics">
        <span>模型诊断</span>
        <strong>未返回</strong>
        <small>当前接口没有提供这个期限的 raw/calibrated/final 诊断。</small>
      </div>
    );
  }

  const runtimeFinal = diagnostic.runtime_final_probability ?? diagnostic.final_probability;
  const gateRows = buildGateRows(diagnostic);

  return (
    <div className="probability-diagnostics">
      <div className="probability-chain">
        <span>模型链路</span>
        <strong>
          raw {formatProbabilityPercentExact(diagnostic.raw_probability)} · calibrated{" "}
          {formatProbabilityPercentExact(diagnostic.calibrated_probability)} · runtime{" "}
          {formatProbabilityPercentExact(runtimeFinal)}
        </strong>
      </div>
      {gateRows.length > 0 ? (
        <div className="probability-gates">
          <span>风险族 gate</span>
          {gateRows.map((row) => (
            <div className="probability-gate-row" key={row.familyId}>
              <strong>{row.label}</strong>
              <small>
                {row.value} / 入场 {row.threshold}
              </small>
              <em className={row.className}>{row.status}</em>
            </div>
          ))}
        </div>
      ) : (
        <small>该期限没有配置 overlay gate，直接使用正式概率头输出。</small>
      )}
    </div>
  );
}

export function ProbabilityTile({
  label,
  value,
  hint,
  threshold,
  thresholdLabel,
  diagnostic
}: {
  label: string;
  value: number;
  hint: string;
  threshold: number;
  thresholdLabel: string;
  diagnostic?: ProbabilityHorizonOverlayDiagnostics;
}) {
  const band = describeProbabilityBand(value);
  const thresholdGap = Math.max(0, threshold - value);
  const thresholdShare = threshold > 0 ? value / threshold : null;
  const thresholdCopy =
    thresholdGap === 0
      ? `已达到${thresholdLabel} ${formatPercentPrecise(threshold)}`
      : `距${thresholdLabel} ${formatPercentPrecise(threshold)} 还差 ${formatPercentagePointGap(
          thresholdGap
        )}`;
  const thresholdShareCopy =
    thresholdShare === null
      ? "未配置进入线"
      : thresholdGap === 0
        ? `已达到${thresholdLabel} · 进入线 ${formatPercentPrecise(threshold)}`
        : value > 0
          ? `当前仅为${thresholdLabel}的 ${formatPercentPrecise(
              thresholdShare
            )} · 触线约需当前值的 ${formatThresholdMultiple(threshold / value)}`
          : `当前为 0 · 进入线 ${formatPercentPrecise(threshold)}`;

  return (
    <div className={`probability-tile ${band.className}`}>
      <div className="tile-head">
        <span>{label}</span>
        <em>{band.label}</em>
      </div>
      <strong>{formatProbabilityPercentExact(value)}</strong>
      <div className="probability-exact">{thresholdShareCopy}</div>
      <div className="probability-raw">
        接口值 {formatProbabilityDecimal(value)} · {formatProbabilityBasisPoints(value)}
      </div>
      <div className="probability-reading-note">{probabilityReadingNote(value)}</div>
      <p>{hint}</p>
      <div className="probability-threshold">{thresholdCopy}</div>
      <small>{band.note}</small>
      <ProbabilityDiagnosticsBlock diagnostic={diagnostic} />
    </div>
  );
}

export function PostureLadder({
  current
}: {
  current: AssessmentSnapshot["posture"];
}) {
  return (
    <div className="posture-ladder">
      {POSTURE_STEPS.map((step) => {
        const active = step.id === current;
        return (
          <div className={active ? "posture-step active" : "posture-step"} key={step.id}>
            <div className="posture-step-head">
              <strong>{step.label}</strong>
              {active && <span>当前</span>}
            </div>
            <p>{step.description}</p>
          </div>
        );
      })}
    </div>
  );
}

export function SignalLayerRows({
  rows
}: {
  rows: DecisionSignalLayerRowModel[];
}) {
  return (
    <div className="signal-layer-list">
      {rows.map((row) => (
        <div className="signal-layer-row" key={row.id}>
          <div>
            <strong>{row.title}</strong>
            <span>{row.description}</span>
          </div>
          <div className="signal-layer-meta">
            <b>{row.value}</b>
            <small>{row.detail}</small>
          </div>
        </div>
      ))}
    </div>
  );
}

export function ClauseList({
  title,
  clauses,
  emptyText
}: {
  title: string;
  clauses: ReturnType<typeof describePostureClause>[];
  emptyText: string;
}) {
  return (
    <div className="clause-section">
      <strong className="clause-section-title">{title}</strong>
      {clauses.length === 0 ? (
        <RuleBox label="当前状态">{emptyText}</RuleBox>
      ) : (
        <div className="clause-grid">
          {clauses.map((clause) => (
            <div className={`clause-card ${clause.kind}`} key={`${title}-${clause.label}`}>
              <strong>{clause.label}</strong>
              <span>{clause.summary}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export function BudgetBar({
  label,
  value,
  valueLabel,
  note,
  tone
}: {
  label: string;
  value: number;
  valueLabel?: string;
  note: string;
  tone: "risk" | "cash" | "hedge" | "leverage" | "option";
}) {
  return (
    <div className="budget-bar">
      <div className="budget-bar-head">
        <strong>{label}</strong>
        <span>{valueLabel ?? formatNumber(value, "%")}</span>
      </div>
      <div className="track budget-track">
        <div className={`fill budget-fill tone-${tone}`} style={{ width: `${value}%` }} />
      </div>
      <span className="budget-note">{note}</span>
    </div>
  );
}
