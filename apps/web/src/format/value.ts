export function formatNumber(value: number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `${value.toFixed(1)}${suffix}`;
}

export function formatSignedNumber(value: number | null | undefined, digits = 1, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const prefix = value > 0 ? "+" : "";
  return `${prefix}${value.toFixed(digits)}${suffix}`;
}

export function formatPercent(value: number | null | undefined, digits = 0): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `${(value * 100).toFixed(digits)}%`;
}

export function formatDate(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }
  return value.slice(0, 10);
}

export function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return "—";
  }

  const normalized = value.replace("T", " ");
  return `${normalized.slice(0, 16)} UTC`;
}

export function wrapTimelineLabel(value: string): string {
  const match = value.match(/^(\d{4}(?:-\d{4})?)(.*)$/);
  if (!match) {
    return value;
  }

  const [, prefix, suffix] = match;
  return `${prefix}\n${suffix.trim()}`;
}
