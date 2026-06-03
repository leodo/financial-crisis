import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import {
  describePostureClause,
  formatNumber,
  humanizeNarrativeCopy,
  indicatorRefLabel
} from "../../format";
import type { AssessmentSnapshot } from "../../types";

export interface MetricItem {
  label: string;
  value: string;
  hint?: string;
  valueClassName?: string;
}

export type MetricPair = [string, string];

export interface VersionRowItem {
  label: string;
  value: string;
  hint?: string;
  note?: string;
  valueClassName?: string;
}

export interface DetailRowItem {
  id: string;
  title: ReactNode;
  detail: ReactNode;
  meta?: ReactNode;
  note?: ReactNode;
}

export type ClauseSummary = ReturnType<typeof describePostureClause>;

export function DetailRows({
  items,
  compact = false
}: {
  items: DetailRowItem[];
  compact?: boolean;
}) {
  return (
    <div className={compact ? "list-stack compact" : "list-stack"}>
      {items.map((item) => (
        <div className={compact ? "list-row compact" : "list-row"} key={item.id}>
          <div>
            <strong>{item.title}</strong>
            <span>{item.detail}</span>
            {item.note !== undefined && item.note !== null ? (
              <small className="metric-note">{item.note}</small>
            ) : null}
          </div>
          {item.meta !== undefined && item.meta !== null ? <b>{item.meta}</b> : null}
        </div>
      ))}
    </div>
  );
}

export function GuideList({
  rows
}: {
  rows: ReadonlyArray<readonly [string, string]>;
}) {
  return <DetailRows items={rows.map(([title, text]) => ({ id: title, title, detail: text }))} />;
}

export function SurfaceHeader({
  title,
  icon: Icon
}: {
  title: ReactNode;
  icon: LucideIcon;
}) {
  return (
    <div className="surface-head">
      <h2>{title}</h2>
      <Icon size={18} />
    </div>
  );
}

export function BulletList({
  items,
  emptyText,
  compact = false,
  emptyVariant = "row"
}: {
  items: string[];
  emptyText?: string;
  compact?: boolean;
  emptyVariant?: "row" | "inline";
}) {
  if (items.length === 0) {
    if (emptyVariant === "inline") {
      return emptyText ? <span className="empty-copy">{emptyText}</span> : null;
    }
    return emptyText ? (
      <div className={compact ? "list-stack compact" : "list-stack"}>
        <div className="bullet-row">
          <span className="bullet-dot" />
          <span>{emptyText}</span>
        </div>
      </div>
    ) : null;
  }

  return (
    <div className={compact ? "list-stack compact" : "list-stack"}>
      {items.map((item, index) => (
        <div className="bullet-row" key={`${item}-${index}`}>
          <span className="bullet-dot" />
          <span>{item}</span>
        </div>
      ))}
    </div>
  );
}

export function DriverList({
  rows,
  reverse = false
}: {
  rows: AssessmentSnapshot["top_risk_drivers"];
  reverse?: boolean;
}) {
  return (
    <DetailRows
      items={rows.map((row) => ({
        id: row.indicator_id,
        title:
          indicatorRefLabel(row.indicator_id) !== row.indicator_id
            ? indicatorRefLabel(row.indicator_id)
            : humanizeNarrativeCopy(row.display_name),
        detail: humanizeNarrativeCopy(row.explanation),
        meta: formatNumber(reverse ? 100 - row.score : row.score)
      }))}
    />
  );
}

export function VersionRow({
  label,
  value,
  hint,
  note,
  valueClassName
}: VersionRowItem) {
  return (
    <div className="version-row">
      <span>{label}</span>
      <div className="version-row-value">
        <strong className={valueClassName} title={hint}>
          {value}
        </strong>
        {note ? <small className="metric-note">{note}</small> : null}
      </div>
    </div>
  );
}

export function Metric({
  label,
  value,
  hint,
  valueClassName
}: {
  label: string;
  value: string;
  hint?: string;
  valueClassName?: string;
}) {
  return (
    <div className="metric">
      <span>{label}</span>
      <strong className={valueClassName}>{value}</strong>
      {hint ? <small className="metric-note">{hint}</small> : null}
    </div>
  );
}

export function MetricGrid({
  items,
  className = "mini-metrics"
}: {
  items: MetricItem[];
  className?: string;
}) {
  return (
    <div className={className}>
      {items.map((item) => (
        <Metric
          key={item.label}
          label={item.label}
          value={item.value}
          hint={item.hint}
          valueClassName={item.valueClassName}
        />
      ))}
    </div>
  );
}

export function MetricPairsGrid({
  pairs,
  className
}: {
  pairs: MetricPair[];
  className?: string;
}) {
  return <MetricGrid items={pairs.map(([label, value]) => ({ label, value }))} className={className} />;
}

export function RuleBox({
  label,
  children,
  className
}: {
  label: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={className ? `rule-box ${className}` : "rule-box"}>
      <strong>{label}</strong>
      {typeof children === "string" ? <span>{children}</span> : children}
    </div>
  );
}

export function ResponsiveTable({
  columns,
  children,
  className,
  note
}: {
  columns: ReactNode[];
  children: ReactNode;
  className?: string;
  note?: ReactNode;
}) {
  return (
    <>
      {note ? <div className="table-note">{note}</div> : null}
      <div className={className ? `table-wrap ${className}` : "table-wrap"}>
        <table>
          <thead>
            <tr>
              {columns.map((column, index) => (
                <th key={`column-${index}`}>{column}</th>
              ))}
            </tr>
          </thead>
          <tbody>{children}</tbody>
        </table>
      </div>
    </>
  );
}

export function StackedTableCell({
  title,
  details
}: {
  title: ReactNode;
  details: ReactNode | ReactNode[];
}) {
  const rows = Array.isArray(details) ? details : [details];

  return (
    <td>
      <strong>{title}</strong>
      {rows
        .filter((row) => row !== undefined && row !== null && row !== false)
        .map((row, index) => (
          <span key={`detail-${index}`}>{row}</span>
        ))}
    </td>
  );
}

export function PillTableCell({
  label,
  className
}: {
  label: ReactNode;
  className?: string;
}) {
  return (
    <td>
      <span className={className ? `state-pill ${className}` : "state-pill"}>{label}</span>
    </td>
  );
}

export function StateSummary({
  pillLabel,
  pillClassName,
  score,
  summary
}: {
  pillLabel: ReactNode;
  pillClassName?: string;
  score: ReactNode;
  summary: ReactNode;
}) {
  return (
    <>
      <div className="jpy-state">
        <span className={pillClassName ? `state-pill ${pillClassName}` : "state-pill"}>
          {pillLabel}
        </span>
        <b>{score}</b>
      </div>
      <p className="body-copy">{summary}</p>
    </>
  );
}

export function renderClauseBulletRows({
  clauses,
  emptyText,
  leadText
}: {
  clauses: ClauseSummary[];
  emptyText?: string;
  leadText?: ReactNode;
}) {
  if (clauses.length === 0) {
    return emptyText ? (
      <div className="bullet-row">
        <span className="bullet-dot" />
        <span>{emptyText}</span>
      </div>
    ) : null;
  }

  return (
    <>
      {leadText ? (
        <div className="bullet-row">
          <span className="bullet-dot" />
          <span>{leadText}</span>
        </div>
      ) : null}
      {clauses.map((clause) => (
        <div className="bullet-row" key={clause.label}>
          <span className="bullet-dot" />
          <span>
            <strong>{clause.label}</strong> {clause.summary}
          </span>
        </div>
      ))}
    </>
  );
}
