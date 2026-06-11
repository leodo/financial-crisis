import {
  describePostureClause,
  formatNumber,
  formatProbabilityBasisPoints,
  formatProbabilityDecimal,
  formatPercentPrecise,
  formatProbabilityPercentExact,
  formatSignedNumber
} from "../../format";
import type { AssessmentSnapshot, ProbabilityHorizonOverlayDiagnostics } from "../../types";
import { RuleBox } from "../shared/panelHelpers";
import type { DecisionSignalLayerRowModel } from "./useDecisionViewModel";
import {
  findProbabilityDiagnosticAnomaly,
  type ProbabilityDiagnosticAnomaly
} from "./probabilityDiagnostics";

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

export function formatPercentagePointGap(value: number): string {
  return formatPercentPrecise(value).replace("%", " 个百分点");
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

function featureLabel(featureName: string): string {
  const exactLabels: Record<string, string> = {
    us_usdjpy_level: "USDJPY",
    us_usdjpy_change_20d: "USDJPY 20日变化",
    us_curve_10y2y_level: "10Y-2Y 利差",
    us_fed_funds_level: "联邦基金利率",
    us_baa_10y_spread_level: "BAA-10Y 信用利差",
    us_market_vix_level: "VIX",
    external_dimension_score: "外部冲击分"
  };
  if (exactLabels[featureName]) {
    return exactLabels[featureName];
  }
  return featureName
    .replace(/^tail_neg__/, "低尾 ")
    .replace(/^tail_pos__/, "高尾 ")
    .replace(/^interaction__/, "交互 ")
    .replace(/^family_context__/, "风险族上下文 ")
    .replace(/^family_proxy__/, "风险族代理 ")
    .replaceAll("__", " × ")
    .replaceAll("_", " ");
}

function buildBaseContributionRows(diagnostic?: ProbabilityHorizonOverlayDiagnostics) {
  return (diagnostic?.base_contributions ?? []).slice(0, 6).map((contribution) => ({
    id: contribution.name,
    label: featureLabel(contribution.name),
    rawName: contribution.name,
    rawValue: formatNumber(contribution.raw_value),
    contribution: formatSignedNumber(contribution.contribution, 3),
    className: contribution.contribution >= 0 ? "base-positive" : "base-negative"
  }));
}

function probabilityReadingNote(
  value: number,
  anomaly: ProbabilityDiagnosticAnomaly | null,
  diagnostic?: ProbabilityHorizonOverlayDiagnostics
): string {
  const runtimeFinal = diagnostic?.runtime_final_probability;
  const modelFinal = diagnostic?.final_probability ?? value;
  const hasRuntimeOverride =
    runtimeFinal !== undefined &&
    Math.abs(runtimeFinal - modelFinal) > 1e-9;
  if (anomaly) {
    return hasRuntimeOverride
      ? `当前页面显示的是运行口径参考值；模型原始最终值仅 ${formatProbabilityPercentExact(
          modelFinal
        )}，因此这张卡不把低概率解释成风险很远或风险为零。`
      : "当前概率数值来自 active release，但该期限命中语义异常；这张卡把它作为参考值展示，不把低概率解释成风险很远或风险为零。";
  }
  if (value === 0) {
    return "当前接口精确返回 0，需要结合数据日期、release 状态和模型链路复核；它不等于市场风险被证明为零。";
  }
  if (hasRuntimeOverride) {
    return `当前页面值是运行口径参考值 ${formatProbabilityPercentExact(
      value
    )}；模型原始最终值为 ${formatProbabilityPercentExact(modelFinal)}。这条参考值用于辅助观察，不单独决定风险时距。`;
  }
  if (value < 0.01) {
    return `当前是低位小概率但不是 0；原始接口值为 ${formatProbabilityDecimal(
      value
    )}，也就是 ${formatProbabilityBasisPoints(value)}。`;
  }
  return "当前概率已高于 1%，应重点看是否接近对应进入线和动作档位。";
}

function describeThresholdDistance(
  value: number,
  threshold: number,
  thresholdLabel: string,
  anomaly: ProbabilityDiagnosticAnomaly | null
): { label: string; note: string } {
  if (threshold <= 0) {
    return {
      label: "未配置进入线",
      note: "当前 release 没有返回可用于比较的动作进入线，需要先复核方法接口。"
    };
  }
  if (anomaly && value < threshold) {
    return {
      label: "参考解读",
      note: `${anomaly.title}；该读数只代表当前 active release 的机械输出，不能当成“离危机很远”的证明。`
    };
  }
  if (value >= threshold) {
    return {
      label: `已触达${thresholdLabel}`,
      note: `正式概率已经达到 ${thresholdLabel}，应优先看执行节奏、事件确认和人工复核清单。`
    };
  }
  if (value === 0) {
    return {
      label: `低于${thresholdLabel}`,
      note: "当前正式概率为精确 0；这通常是模型输出或数据链路需要复核的信号，不应被解释成零风险证明。"
    };
  }

  const share = value / threshold;
  if (share < 0.001) {
    return {
      label: `极低于${thresholdLabel}`,
      note: `当前不到 ${thresholdLabel} 的 0.1%，说明活跃正式模型没有捕捉到这个期限的危机前证据；若直觉上不合理，应进入模型审计，而不是在 UI 上硬抬概率。`
    };
  }
  if (share < 0.01) {
    return {
      label: `远低于${thresholdLabel}`,
      note: `当前不到 ${thresholdLabel} 的 1%，仍属于非常低位，需要结合关键指标和历史类比确认模型是否漏看风险。`
    };
  }
  if (share < 0.1) {
    return {
      label: `低于${thresholdLabel}`,
      note: `当前已经是非零概率，但仍明显低于 ${thresholdLabel}；更适合继续观察，而不是按危机临近处理。`
    };
  }
  if (share < 0.5) {
    return {
      label: `接近${thresholdLabel}前段`,
      note: `当前离 ${thresholdLabel} 还有距离，但已经不是极低位，应重点看是否连续升温。`
    };
  }
  return {
    label: `接近${thresholdLabel}`,
    note: `当前已经接近 ${thresholdLabel}，若事件确认和关键指标同步恶化，应准备动作升级。`
  };
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
  const baseRows = buildBaseContributionRows(diagnostic);

  return (
    <div className="probability-diagnostics">
      <div className="probability-chain">
        <span>模型链路</span>
        <strong>
          raw {formatProbabilityPercentExact(diagnostic.raw_probability)} · calibrated{" "}
          {formatProbabilityPercentExact(diagnostic.calibrated_probability)} · model final{" "}
          {formatProbabilityPercentExact(diagnostic.final_probability)} · runtime{" "}
          {formatProbabilityPercentExact(runtimeFinal)}
        </strong>
      </div>
      {baseRows.length > 0 ? (
        <div className="probability-base-contributions">
          <span>Base 头贡献</span>
          {baseRows.map((row) => (
            <div className="probability-base-row" key={row.id}>
              <strong>{row.label}</strong>
              <small>{row.rawName}</small>
              <em className={row.className}>{row.contribution}</em>
              <b>原始值 {row.rawValue}</b>
            </div>
          ))}
        </div>
      ) : null}
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
  diagnostic,
  forceAuditOnly = false
}: {
  label: string;
  value: number;
  hint: string;
  threshold: number;
  thresholdLabel: string;
  diagnostic?: ProbabilityHorizonOverlayDiagnostics;
  forceAuditOnly?: boolean;
}) {
  const band = describeProbabilityBand(value);
  const anomaly = findProbabilityDiagnosticAnomaly(diagnostic);
  const modelFinal = diagnostic?.final_probability ?? value;
  const runtimeFinal = diagnostic?.runtime_final_probability ?? modelFinal;
  const hasRuntimeOverride =
    diagnostic?.runtime_final_probability !== undefined &&
    Math.abs(runtimeFinal - modelFinal) > 1e-9;
  const thresholdGap = Math.max(0, threshold - value);
  const thresholdShare = threshold > 0 ? value / threshold : null;
  const thresholdDistance = describeThresholdDistance(value, threshold, thresholdLabel, anomaly);
  const displayThresholdDistance =
    forceAuditOnly && !anomaly
      ? {
          label: "参考解读",
          note: "MVP 当前把正式概率作为参考输入；这一期限仍显示实际数值，但不单独决定风险时距或动作档位。"
        }
      : thresholdDistance;
  const thresholdShareValue =
    thresholdShare === null ? "—" : formatProbabilityPercentExact(thresholdShare);
  const distanceJudgmentDisabled = anomaly !== null;
  const valueLabel = anomaly
    ? hasRuntimeOverride
      ? "当前参考概率（运行口径）"
      : "当前参考概率"
    : forceAuditOnly
      ? hasRuntimeOverride
        ? "当前参考概率（运行口径）"
        : "当前参考概率"
      : "当前正式概率";
  const primaryValue = formatProbabilityPercentExact(value);
  const distanceHeadline = distanceJudgmentDisabled
    ? "参考判断"
    : forceAuditOnly
      ? "参考态"
    : thresholdGap === 0
      ? "已触线"
      : `还差 ${formatPercentagePointGap(thresholdGap)}`;
  const distanceLabel = distanceJudgmentDisabled
    ? "当前限制"
    : forceAuditOnly
      ? "参考距离"
      : "距离动作线";
  const distanceDetail =
    thresholdShare === null
      ? null
      : distanceJudgmentDisabled
        ? "当前先按参考值解读；下方接口值和模型链路只用于复核 active release 为什么偏冷。"
        : forceAuditOnly
          ? "当前不按阈值占比、差值或放大倍数解释这条概率；动作升级仍以规则层、事件确认和数据新鲜度为主。"
          : `当前读数约为动作线的 ${thresholdShareValue}；这是阈值相对位置，不是剩余天数，也不是自动交易信号。`;
  const thresholdCopy =
    distanceJudgmentDisabled
      ? `${thresholdLabel} ${formatPercentPrecise(
          threshold
        )} 仅作为参考线；当前优先复核模型读数，再结合规则层解释。`
      : forceAuditOnly
        ? `${thresholdLabel} ${formatPercentPrecise(
            threshold
          )} 作为参考动作线展示；页面先按规则层给出主结论。`
      : thresholdGap === 0
        ? `已达到${thresholdLabel} ${formatPercentPrecise(threshold)}`
        : `距${thresholdLabel} ${formatPercentPrecise(threshold)} 还差 ${formatPercentagePointGap(
            thresholdGap
          )}`;
  const bandNote = distanceJudgmentDisabled
    ? "当前属于模型异常参考值，不按低位、准备区、对冲区或防守区直接解释。"
    : forceAuditOnly
      ? "当前数值用于辅助观察，不单独决定动作档位。"
      : band.note;

  return (
    <div
      className={`probability-tile ${band.className}${
        distanceJudgmentDisabled ? " model-anomaly" : ""
      }`}
    >
      <div className="tile-head">
        <span>{label}</span>
        <em>{displayThresholdDistance.label}</em>
      </div>
      <span className="probability-value-label">{valueLabel}</span>
      <strong>{primaryValue}</strong>
      {distanceJudgmentDisabled || forceAuditOnly ? (
        <div className="probability-reading-status">
          页面值 {formatProbabilityPercentExact(value)} · {formatProbabilityBasisPoints(value)}
          {hasRuntimeOverride ? `；模型原始 ${formatProbabilityPercentExact(modelFinal)}` : ""}；
          {distanceJudgmentDisabled ? " 当前先不单独用于风险时距判断" : " 当前以规则层结论为主"}
        </div>
      ) : null}
      {anomaly ? (
        <div className="probability-model-warning">
          <strong>{anomaly.title}</strong>
          <small>{anomaly.detail}</small>
        </div>
      ) : null}
      <div className="probability-distance-summary">
        <span>{distanceLabel}</span>
        <strong>{distanceHeadline}</strong>
        {distanceDetail ? <small>{distanceDetail}</small> : null}
        <small>{displayThresholdDistance.note}</small>
      </div>
      <div className="probability-distance-grid">
        <div>
          <span>{thresholdLabel}</span>
          <strong>{formatPercentPrecise(threshold)}</strong>
        </div>
        <div>
          <span>
            {distanceJudgmentDisabled
              ? "模型状态"
              : forceAuditOnly
                ? "主结论口径"
                : "当前占动作线"}
          </span>
          <strong>
            {distanceJudgmentDisabled
              ? "待修复"
              : forceAuditOnly
                ? "规则层优先"
                : thresholdShareValue}
          </strong>
        </div>
        <div>
          <span>
            {distanceJudgmentDisabled
              ? "距离判断"
              : forceAuditOnly
                ? "时距解读"
                : "差值"}
          </span>
          <strong>
            {distanceJudgmentDisabled
              ? "不适用"
              : forceAuditOnly
                ? "参考态"
              : thresholdGap === 0
                ? "已触线"
                : formatPercentagePointGap(thresholdGap)}
          </strong>
        </div>
      </div>
      <div className="probability-raw">
        页面值 {formatProbabilityDecimal(value)} · {formatProbabilityBasisPoints(value)}
        {hasRuntimeOverride ? `；模型原始 ${formatProbabilityDecimal(modelFinal)}` : ""}
      </div>
      <div className="probability-reading-note">
        {probabilityReadingNote(value, anomaly, diagnostic)}
      </div>
      <p>{hint}</p>
      <div className="probability-threshold">{thresholdCopy}</div>
      <small>{bandNote}</small>
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
