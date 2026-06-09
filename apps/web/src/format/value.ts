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

function trimTrailingZeros(value: string): string {
  return value.replace(/(?:\.0+|(\.\d*?[1-9])0+)$/, "$1");
}

export function formatProbabilityPercent(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  if (absolute === 0) {
    return "0.0%";
  }
  if (absolute < 0.0001) {
    return "<0.01%";
  }
  if (absolute < 0.001) {
    return `${(value * 100).toFixed(2)}%`;
  }
  if (absolute < 0.01) {
    return `${(value * 100).toFixed(1)}%`;
  }
  return `${(value * 100).toFixed(absolute < 0.1 ? 1 : 0)}%`;
}

export function formatCount(value: number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  return `${Math.round(value)}${suffix}`;
}

export function formatPreciseNumber(value: number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  const digits =
    absolute >= 10
      ? 1
      : absolute >= 1
        ? 2
        : absolute >= 0.1
          ? 3
          : absolute >= 0.01
            ? 4
            : 5;
  return `${trimTrailingZeros(value.toFixed(digits))}${suffix}`;
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
