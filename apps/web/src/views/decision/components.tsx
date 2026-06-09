import {
  describePostureClause,
  formatNumber,
  formatPercentPrecise,
  formatProbabilityPercent
} from "../../format";
import type { AssessmentSnapshot } from "../../types";
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

export function ProbabilityTile({
  label,
  value,
  hint,
  threshold,
  thresholdLabel
}: {
  label: string;
  value: number;
  hint: string;
  threshold: number;
  thresholdLabel: string;
}) {
  const band = describeProbabilityBand(value);
  const thresholdGap = Math.max(0, threshold - value);
  const thresholdCopy =
    thresholdGap === 0
      ? `已达到${thresholdLabel} ${formatPercentPrecise(threshold)}`
      : `距${thresholdLabel} ${formatPercentPrecise(threshold)} 还差 ${formatPercentPrecise(
          thresholdGap
        )}`;

  return (
    <div className={`probability-tile ${band.className}`}>
      <div className="tile-head">
        <span>{label}</span>
        <em>{band.label}</em>
      </div>
      <strong>{formatProbabilityPercent(value)}</strong>
      <p>{hint}</p>
      <div className="probability-threshold">{thresholdCopy}</div>
      <small>{band.note}</small>
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
  note,
  tone
}: {
  label: string;
  value: number;
  note: string;
  tone: "risk" | "cash" | "hedge" | "leverage" | "option";
}) {
  return (
    <div className="budget-bar">
      <div className="budget-bar-head">
        <strong>{label}</strong>
        <span>{formatNumber(value, "%")}</span>
      </div>
      <div className="track budget-track">
        <div className={`fill budget-fill tone-${tone}`} style={{ width: `${value}%` }} />
      </div>
      <span className="budget-note">{note}</span>
    </div>
  );
}
