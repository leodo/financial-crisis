export function formatNumber(value: number | null | undefined, suffix = ""): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  if (absolute === 0) {
    return `0.0${suffix}`;
  }
  if (absolute < 0.1) {
    const digits = absolute < 0.01 ? 4 : 3;
    return `${trimTrailingZeros(value.toFixed(digits))}${suffix}`;
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

export function formatPercentPrecise(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  if (absolute === 0) {
    return "0%";
  }
  if (absolute < 0.0001) {
    return "<0.01%";
  }
  if (absolute < 0.01) {
    return `${trimTrailingZeros((value * 100).toFixed(2))}%`;
  }
  if (absolute < 1) {
    return `${trimTrailingZeros((value * 100).toFixed(1))}%`;
  }
  return `${trimTrailingZeros((value * 100).toFixed(0))}%`;
}

function trimTrailingZeros(value: string): string {
  return value.replace(/(?:\.0+|(\.\d*?[1-9])0+)$/, "$1");
}

export function formatProbabilityPercent(
  value: number | null | undefined,
  options: { zeroLabel?: string } = {}
): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  if (absolute === 0) {
    return options.zeroLabel ?? "0.0%";
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

export function formatProbabilityPercentExact(
  value: number | null | undefined
): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const percent = value * 100;
  const absolutePercent = Math.abs(percent);
  if (absolutePercent === 0) {
    return "0%";
  }
  if (absolutePercent < 0.0001) {
    return "<0.0001%";
  }
  const digits =
    absolutePercent < 0.01
      ? 4
      : absolutePercent < 0.1
        ? 3
        : absolutePercent < 1
          ? 2
          : 1;
  return `${trimTrailingZeros(percent.toFixed(digits))}%`;
}

export function formatProbabilityBasisPoints(
  value: number | null | undefined
): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const basisPoints = value * 10000;
  const absolute = Math.abs(basisPoints);
  if (absolute === 0) {
    return "0 bp";
  }
  const digits = absolute < 1 ? 2 : absolute < 10 ? 1 : 0;
  return `${trimTrailingZeros(basisPoints.toFixed(digits))} bp`;
}

export function formatProbabilityDecimal(value: number | null | undefined): string {
  if (value === null || value === undefined || Number.isNaN(value)) {
    return "—";
  }
  const absolute = Math.abs(value);
  if (absolute === 0) {
    return "0";
  }
  if (absolute < 0.000001) {
    return value.toExponential(2);
  }
  return trimTrailingZeros(value.toFixed(6));
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

function parseDateTime(value: string | null | undefined): Date | null {
  if (!value) {
    return null;
  }

  const trimmed = value.trim();
  if (!trimmed) {
    return null;
  }

  const millisecondsPrecision = trimmed.replace(
    /\.(\d{3})\d+(?=Z$|[+-]\d{2}:?\d{2}$|$)/,
    ".$1"
  );
  const withTimezone = /(?:Z|[+-]\d{2}:?\d{2})$/.test(millisecondsPrecision)
    ? millisecondsPrecision
    : `${millisecondsPrecision}Z`;
  const parsed = new Date(withTimezone);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
}

export function formatDateTimeWithLocal(value: string | null | undefined): string {
  const utcText = formatDateTime(value);
  const parsed = parseDateTime(value);
  if (!parsed) {
    return utcText;
  }

  const localParts = new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
    timeZoneName: "short"
  })
    .formatToParts(parsed)
    .reduce<Record<string, string>>((parts, part) => {
      if (part.type !== "literal") {
        parts[part.type] = part.value;
      }
      return parts;
    }, {});
  const localText = `${localParts.year}-${localParts.month}-${localParts.day} ${localParts.hour}:${localParts.minute}${
    localParts.timeZoneName ? ` ${localParts.timeZoneName}` : ""
  }`;

  return `${utcText}（本地 ${localText}）`;
}

export function wrapTimelineLabel(value: string): string {
  const match = value.match(/^(\d{4}(?:-\d{4})?)(.*)$/);
  if (!match) {
    return value;
  }

  const [, prefix, suffix] = match;
  return `${prefix}\n${suffix.trim()}`;
}
