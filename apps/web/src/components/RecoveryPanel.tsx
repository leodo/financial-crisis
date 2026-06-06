import type { ReactNode } from "react";

export interface RecoveryAction {
  label: string;
  onClick: () => void;
  disabled?: boolean;
  tone?: "primary" | "secondary";
}

interface RecoveryPanelProps {
  title: string;
  summary: string;
  icon: ReactNode;
  tone?: "error" | "warning";
  details?: string[];
  actions?: RecoveryAction[];
  footer?: string;
}

export function RecoveryPanel({
  title,
  summary,
  icon,
  tone = "error",
  details,
  actions,
  footer
}: RecoveryPanelProps) {
  return (
    <section className={`recovery-panel ${tone}`} role="alert">
      <div className="recovery-head">
        <div className="recovery-icon" aria-hidden="true">
          {icon}
        </div>
        <div className="recovery-copy">
          <strong>{title}</strong>
          <p>{summary}</p>
        </div>
      </div>

      {details && details.length > 0 ? (
        <ul className="recovery-detail-list">
          {details.map((detail) => (
            <li key={detail}>{detail}</li>
          ))}
        </ul>
      ) : null}

      {actions && actions.length > 0 ? (
        <div className="action-row">
          {actions.map((action) => (
            <button
              key={action.label}
              className={
                action.tone === "primary"
                  ? "action-button primary"
                  : "action-button secondary"
              }
              disabled={action.disabled}
              onClick={action.onClick}
              type="button"
            >
              {action.label}
            </button>
          ))}
        </div>
      ) : null}

      {footer ? <small className="recovery-footer">{footer}</small> : null}
    </section>
  );
}
